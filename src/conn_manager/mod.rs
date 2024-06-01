pub mod conn_bambu;
pub mod conn_klipper;

use std::{collections::HashMap, sync::Arc};

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use dashmap::DashMap;
use tokio::sync::RwLock;

use crate::{
    config::{printer_config::PrinterConfig, printer_id::PrinterId, AppConfig},
    status::GenericPrinterState,
};
use conn_bambu::message::Message;

/// messages from PrinterConnManager to UI
#[derive(Debug)]
pub enum PrinterConnMsg {
    //
}

/// messages from UI to PrinterConnManager
#[derive(Debug)]
pub enum PrinterConnCmd {
    //
}

#[derive(Debug)]
pub enum WorkerCmd {
    //
}

pub struct PrinterConnManager {
    config: AppConfig,

    // printers: HashMap<PrinterId, BambuClient>,
    printer_states: Arc<DashMap<PrinterId, GenericPrinterState>>,

    worker_cmd_txs: HashMap<PrinterId, tokio::sync::mpsc::UnboundedSender<WorkerCmd>>,

    cmd_tx: tokio::sync::mpsc::UnboundedSender<PrinterConnCmd>,
    cmd_rx: tokio::sync::mpsc::UnboundedReceiver<PrinterConnCmd>,
    msg_tx: tokio::sync::mpsc::UnboundedSender<PrinterConnMsg>,

    // /// for sending commands to worker tasks
    // worker_cmd_tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, PrinterConnCmd)>,
    // /// for cloning to pass to worker tasks
    // // worker_cmd_rx: tokio::sync::mpsc::UnboundedReceiver<(PrinterId, PrinterConnCmd)>,
    /// for cloning to pass to worker tasks
    worker_msg_tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, Message)>,
    /// to receive messages from worker tasks
    worker_msg_rx: tokio::sync::mpsc::UnboundedReceiver<(PrinterId, Message)>,

    kill_chans: HashMap<PrinterId, tokio::sync::oneshot::Sender<()>>,
    // error_map: ErrorMap,
}

/// new, start listeners
impl PrinterConnManager {
    pub async fn new(
        config: AppConfig,
        printer_states: Arc<DashMap<PrinterId, GenericPrinterState>>,
        cmd_tx: tokio::sync::mpsc::UnboundedSender<PrinterConnCmd>,
        cmd_rx: tokio::sync::mpsc::UnboundedReceiver<PrinterConnCmd>,
        msg_tx: tokio::sync::mpsc::UnboundedSender<PrinterConnMsg>,
    ) -> Self {
        let (worker_msg_tx, mut worker_msg_rx) =
            tokio::sync::mpsc::unbounded_channel::<(PrinterId, Message)>();

        // let (worker_cmd_tx, worker_cmd_rx) =
        //     tokio::sync::mpsc::unbounded_channel::<(PrinterId, PrinterConnCmd)>();

        // /// fetch error codes
        // let error_map = ErrorMap::read_or_fetch().await.unwrap_or_default();

        Self {
            config,

            printer_states,

            // printers: HashMap::new(),
            // printer_states,
            worker_cmd_txs: HashMap::new(),
            cmd_tx,
            cmd_rx,
            msg_tx,

            worker_msg_tx,
            worker_msg_rx,
            kill_chans: HashMap::new(),
            // stream_cmd_tx,
            // graphs,
            // error_map,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        for printer in self.config.printers() {
            debug!("adding printer");
            self.add_printer(printer).await?;
        }

        loop {
            tokio::select! {
                Some(cmd) = self.cmd_rx.recv() => {
                    debug!("got cmd");
                    // match cmd {
                    //     PrinterConnCmd::Login(_, _) => debug!("got cmd = Login"),
                    //     _ => debug!("got cmd = {:?}", cmd),
                    // }
                    // self.handle_command(cmd).await?;
                }
                Some((id, printer_msg)) = self.worker_msg_rx.recv() => {
                    debug!("got printer_msg, id = {:?} = {:?}", id, printer_msg);
                    // if let Some(printer) = self.config.get_printer(&id) {
                    // }
                    self.handle_printer_msg(id, printer_msg).await?;
                    // panic!("TODO: handle printer message");
                }
            }
        }
    }

    async fn add_printer(&mut self, printer: PrinterConfig) -> Result<()> {
        // if !from_cfg {
        //     // self.config.add_printer(printer.unwrap_or_clone()));
        //     self.config.add_printer(printer.clone()).await;
        // }

        let (kill_tx, kill_rx) = tokio::sync::oneshot::channel::<()>();

        let id = printer.id();
        if self.kill_chans.contains_key(&id) {
            bail!("printer already exists: {:?}", id);
        }
        self.kill_chans.insert(id.clone(), kill_tx);

        match printer {
            PrinterConfig::Bambu(_, printer) => {
                let (worker_cmd_tx, worker_cmd_rx) =
                    tokio::sync::mpsc::unbounded_channel::<WorkerCmd>();

                let mut client = conn_bambu::bambu_proto::BambuClient::new_and_init(
                    self.config.clone(),
                    printer.clone(),
                    self.worker_msg_tx.clone(),
                    worker_cmd_rx,
                    kill_rx,
                )
                .await?;

                // self.printers.insert(id.clone(), client);
                self.worker_cmd_txs.insert(id.clone(), worker_cmd_tx);
            }
            _ => todo!(),
        }

        Ok(())
    }
}

/// handle messages, commands
impl PrinterConnManager {
    async fn handle_printer_msg(
        &mut self,
        // printer: Arc<PrinterConfig>,
        id: PrinterId,
        msg: Message,
    ) -> Result<()> {
        let Some(printer) = self.config.get_printer(&id) else {
            bail!("printer not found: {:?}", id);
        };

        #[cfg(feature = "nope")]
        match msg {
            Message::Print(report) => {
                // debug!("got print report");

                let printer = printer.read().await;

                self.graphs.update_printer(&printer.serial, &report.print);

                let mut entry = self
                    .printer_states
                    .entry(printer.serial.clone())
                    .or_default();

                let prev_state = entry.state.clone();
                let prev_error = entry.is_error();

                entry.update(&printer, &report.print)?;

                if prev_state != entry.state {
                    info!("printer state changed: {:?}", entry.state);

                    /// print just finished, send notification
                    if prev_state != PrinterState::Disconnected
                        && entry.state == PrinterState::Finished
                    {
                        warn!("sent finish notification");
                        crate::alert::alert_print_complete(
                            &printer.name,
                            entry
                                .current_file
                                .as_ref()
                                .unwrap_or(&"Unknown File".to_string()),
                        )
                    }

                    /// either print just started, or app was just started
                    if entry.state == PrinterState::Printing && entry.subtask_id.is_some() {
                        entry.current_task_thumbnail_url = None;
                    }
                }

                /// logged in and printing, but no thumbnail
                if self.config.logged_in()
                    && entry.state == PrinterState::Printing
                    && entry.subtask_id.is_some()
                    && entry.current_task_thumbnail_url.is_none()
                {
                    let config2 = self.config.clone();
                    // let printer2 = printer.clone();
                    let serial = printer.serial.clone();
                    let printer_states2 = self.printer_states.clone();
                    let task_id = entry.subtask_id.as_ref().unwrap().clone();
                    // warn!("skipping fetch thumnail");
                    warn!("spawning fetch thumnail");
                    tokio::spawn(async {
                        fetch_printer_task_thumbnail(config2, serial, printer_states2, task_id)
                            .await;
                    });
                    warn!("spawned fetch thumnail");
                    //
                }

                if !prev_error && entry.is_error() {
                    warn!("printer error: {:?}", &printer.name);

                    let error = report
                        .print
                        .print_error
                        .clone()
                        .context("no error found?")?;
                    let name = self
                        .config
                        .get_printer(&printer.serial)
                        .context("printer not found")?
                        .read()
                        .await
                        .name
                        .clone();

                    let error = self
                        .error_map
                        .get_error(error as u64)
                        .unwrap_or("Unknown Error");

                    crate::alert::alert_printer_error(&printer.name, error);
                }

                self.ctx.request_repaint();

                if let Err(e) = self.msg_tx.send(PrinterConnMsg::StatusReport(
                    printer.serial.clone(),
                    report.print,
                )) {
                    error!("error sending status report: {:?}", e);
                }

                if entry.printer_type.is_none() {
                    self.cmd_tx
                        .send(PrinterConnCmd::ReportInfo(printer.serial.clone()))?;
                }

                // .await
            }
            Message::Info(info) => {
                // debug!("printer info for {:?}: {:?}", &printer.name, info);
                debug!(
                    "got printer info for printer: {:?}",
                    &printer.read().await.name
                );

                let mut entry = self
                    .printer_states
                    .entry(printer.read().await.serial.clone())
                    .or_default();

                entry.printer_type = Some(crate::utils::get_printer_type(&info.info));

                #[cfg(feature = "nope")]
                for module in info.info.module.iter() {
                    // debug!("module {:?} = {:?}", module.name, module.project_name);

                    // let mut module = module.clone();
                    // module.sn = "redacted".to_string();
                    // debug!("module {:?} = {:?}", module.name, module);

                    #[cfg(feature = "nope")]
                    if module.name == "mc" {
                        // debug!("project_name = {:?}", module.project_name);
                        match module.project_name.as_ref() {
                            None => entry.printer_type = Some(PrinterType::X1),
                            Some(s) => match s.as_str() {
                                "P1" => {
                                    if entry.chamber_fan_speed.is_some() {
                                        entry.printer_type = Some(PrinterType::P1S);
                                    } else {
                                        entry.printer_type = Some(PrinterType::P1P);
                                    }
                                }
                                "N2S" => entry.printer_type = Some(PrinterType::A1),
                                "N1" => entry.printer_type = Some(PrinterType::A1m),
                                _ => {
                                    warn!("unknown printer type: {:?}", s);
                                    entry.printer_type = Some(PrinterType::Unknown);
                                }
                            },
                        }
                        debug!("set printer type: {:?}", entry.printer_type);
                    }
                }
                // entry.printer_type

                //
            }
            Message::System(system) => debug!("printer system: {:?}", system),
            Message::Unknown(unknown) => match unknown {
                Some(unknown) => warn!("unknown message: {:?}", unknown),
                _ => trace!("unknown message: None"),
            },
            Message::Connecting => debug!("printer connecting: {:?}", &printer.read().await.name),
            Message::Connected => {
                let name = &printer.read().await.name;
                info!("printer connected: {:?}", &name);

                let client = self
                    .printers
                    .get(&printer.read().await.serial)
                    .with_context(|| format!("printer not found: {:?}", &name))?;
                if let Err(e) = client.publish(Command::PushAll).await {
                    error!("error publishing status: {:?}", e);
                }
                let mut entry = self
                    .printer_states
                    .entry(printer.read().await.serial.clone())
                    .or_default();
                entry.reset();
                self.ctx.request_repaint();
            }
            Message::Reconnecting => {
                warn!("printer reconnecting: {:?}", &printer.read().await.name)
            }
            Message::Disconnected => {
                error!("printer disconnected: {:?}", &printer.read().await.name);

                let mut entry = self
                    .printer_states
                    .entry(printer.read().await.serial.clone())
                    .or_default();
                entry.state = PrinterState::Disconnected;
                self.ctx.request_repaint();
            }
        }
        Ok(())
    }

    async fn handle_command(&mut self, cmd: PrinterConnCmd) -> Result<()> {
        #[cfg(feature = "nope")]
        match cmd {
            PrinterConnCmd::AddPrinter(printer) => {
                self.add_printer(Arc::new(RwLock::new(printer)), false)
                    .await?;
                // unimplemented!()
            }
            PrinterConnCmd::SyncPrinters => {
                let ctx2 = self.ctx.clone();
                let config2 = self.config.clone();
                let msg_tx2 = self.msg_tx.clone();
                tokio::spawn(async move {
                    if let Err(e) = sync_printers(ctx2, config2, msg_tx2).await {
                        error!("error syncing printers: {:?}", e);
                    }
                });
            }
            PrinterConnCmd::SetPrinterCloud(id, cloud) => {
                debug!("set printer cloud: {:?}", cloud);

                {
                    // let mut cfg = self.config.config.write().await;
                    // if let Some(printer) = cfg.printer_mut(&id) {
                    //     // printer.cloud = cloud;
                    // }
                    error!("TODO: set printer cloud");
                }

                //
            }
            PrinterConnCmd::SyncProjects => {
                let config2 = self.config.clone();
                let msg_tx2 = self.msg_tx.clone();
                tokio::spawn(async move {
                    if let Err(e) = sync_projects(config2, msg_tx2).await {
                        error!("error syncing projects: {:?}", e);
                    }
                });
            }
            PrinterConnCmd::ReportInfo(id) => {
                let client = self
                    .printers
                    .get(&id)
                    .with_context(|| format!("printer not found: {:?}", id))?;
                if let Err(e) = client.publish(Command::GetVersion).await {
                    error!("error publishing status: {:?}", e);
                }
            }
            PrinterConnCmd::ReportStatus(id) => {
                let client = self
                    .printers
                    .get(&id)
                    .with_context(|| format!("printer not found: {:?}", id))?;
                if let Err(e) = client.publish(Command::PushAll).await {
                    error!("error publishing status: {:?}", e);
                }
            }
            PrinterConnCmd::Login(username, password) => {
                // self.get_token(username, pass).await?;
                let tx2 = self.msg_tx.clone();
                let config2 = self.config.clone();

                tokio::spawn(async move {
                    if let Err(e) = login(tx2, config2, username, password).await {
                        error!("error getting token: {:?}", e);
                    }
                });

                #[cfg(feature = "nope")]
                tokio::spawn(async move {
                    // if let Err(e) = login(tx2, auth, username, password).await {
                    //     error!("error getting token: {:?}", e);
                    // }
                    // login(tx2, auth, username, password).await.unwrap();
                    // let t = auth.write().get_token();
                    // debug!("got token: {:?}", t);
                    // tx2.send(PrinterConnMsg::LoggedIn).unwrap();

                    let mut auth2 = auth.write();

                    auth2
                        .login_and_get_token(&username2, &password2)
                        .await
                        .unwrap();

                    // auth.write()
                    //     .login_and_get_token(&username2, &password2)
                    //     .await
                    //     .unwrap();
                    // if let Err(e) = auth.write().login_and_get_token(&username, &password).await {
                    //     error!("error fetching token: {:?}", e);
                    // };
                });
            }
            PrinterConnCmd::Logout => {
                // self.config.config.write().await.logged_in = false;
                self.config.set_logged_in(false);
                if let Err(e) = self.config.auth.write().await.clear_token() {
                    error!("error clearing token: {:?}", e);
                }
            }

            PrinterConnCmd::RemovePrinter(_) => todo!(),
            PrinterConnCmd::UpdatePrinterConfig(id, cfg) => {
                self.config.update_printer(&id, &cfg).await;
                if !cfg.host.is_empty() {
                    self.stream_cmd_tx.send(StreamCmd::RestartStream(id))?;
                } else {
                    self.stream_cmd_tx.send(StreamCmd::StopStream(id))?;
                }
            }
            PrinterConnCmd::Pause => todo!(),
            PrinterConnCmd::Stop => todo!(),
            PrinterConnCmd::Resume => todo!(),
            PrinterConnCmd::SetChamberLight(_) => todo!(),
            PrinterConnCmd::ChangeSpeed(_) => todo!(),
            PrinterConnCmd::GCodeLine(_) => todo!(),
            PrinterConnCmd::Calibration => todo!(),
            PrinterConnCmd::UnloadFilament => todo!(),
            PrinterConnCmd::ChangeFilament(_) => todo!(),
            PrinterConnCmd::ChangeAMSFilamentSetting {
                ams_id,
                tray_id,
                tray_color,
                nozzle_temp_min,
                nozzle_temp_max,
                tray_type,
            } => todo!(),
        }
        Ok(())
    }
}
