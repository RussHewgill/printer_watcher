pub mod conn_bambu;
pub mod conn_klipper;
pub mod conn_octoprint;
pub mod conn_prusa;
pub mod helpers;
pub mod worker_message;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use core::error;
use std::{collections::HashMap, sync::Arc};

use dashmap::DashMap;
use tokio::sync::RwLock;
use worker_message::WorkerMsg;

use crate::{
    config::{printer_config::PrinterConfig, printer_id::PrinterId, AppConfig},
    status::{GenericPrinterState, PrinterState},
    streaming::StreamCmd,
};
use conn_bambu::{errors::ErrorMap, message::Message};

/// messages from PrinterConnManager to UI
#[derive(Debug, Clone)]
pub enum PrinterConnMsg {
    WorkerMsg(PrinterId, WorkerMsg),
    NewThumbnail(PrinterId, String, Vec<u8>),
}

/// messages from UI to PrinterConnManager
#[derive(Debug, Clone)]
pub enum PrinterConnCmd {
    FetchThumbnail(PrinterId, String),
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
    worker_msg_tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, WorkerMsg)>,
    /// to receive messages from worker tasks
    worker_msg_rx: tokio::sync::mpsc::UnboundedReceiver<(PrinterId, WorkerMsg)>,

    kill_chans: HashMap<PrinterId, tokio::sync::oneshot::Sender<()>>,

    stream_tx: tokio::sync::mpsc::UnboundedSender<StreamCmd>,

    error_map: ErrorMap,
}

/// new, start listeners
impl PrinterConnManager {
    pub async fn new(
        config: AppConfig,
        printer_states: Arc<DashMap<PrinterId, GenericPrinterState>>,
        cmd_tx: tokio::sync::mpsc::UnboundedSender<PrinterConnCmd>,
        cmd_rx: tokio::sync::mpsc::UnboundedReceiver<PrinterConnCmd>,
        msg_tx: tokio::sync::mpsc::UnboundedSender<PrinterConnMsg>,
        stream_tx: tokio::sync::mpsc::UnboundedSender<StreamCmd>,
    ) -> Self {
        let (worker_msg_tx, mut worker_msg_rx) =
            tokio::sync::mpsc::unbounded_channel::<(PrinterId, WorkerMsg)>();

        // let (worker_cmd_tx, worker_cmd_rx) =
        //     tokio::sync::mpsc::unbounded_channel::<(PrinterId, PrinterConnCmd)>();

        /// fetch error codes
        let error_map = ErrorMap::read_or_fetch().await.unwrap_or_default();

        Self {
            config,

            // printer_states,

            // printers: HashMap::new(),
            printer_states,
            worker_cmd_txs: HashMap::new(),
            cmd_tx,
            cmd_rx,
            msg_tx,

            worker_msg_tx,
            worker_msg_rx,
            kill_chans: HashMap::new(),

            stream_tx,

            // graphs,
            error_map,
        }
    }

    pub async fn init(&mut self) -> Result<()> {
        for printer in self.config.printers() {
            debug!("adding printer");
            self.add_printer(printer).await?;
        }
        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        loop {
            tokio::select! {
                Some(cmd) = self.cmd_rx.recv() => {
                    debug!("got cmd");
                    self.handle_command(cmd).await?;
                }
                Some((id, printer_msg)) = self.worker_msg_rx.recv() => {
                    // debug!("got printer_msg, id = {:?} = {:?}", id, printer_msg);
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

        let (worker_cmd_tx, worker_cmd_rx) = tokio::sync::mpsc::unbounded_channel::<WorkerCmd>();

        match printer {
            PrinterConfig::Bambu(_, printer) => {
                let mut client = conn_bambu::bambu_proto::BambuClient::new_and_init(
                    self.config.clone(),
                    printer.clone(),
                    self.worker_msg_tx.clone(),
                    worker_cmd_rx,
                    kill_rx,
                )
                .await?;

                self.worker_cmd_txs.insert(id.clone(), worker_cmd_tx);
            }
            PrinterConfig::Klipper(_, printer) => {
                self.worker_cmd_txs.insert(id.clone(), worker_cmd_tx);

                let worker_msg_tx = self.worker_msg_tx.clone();
                tokio::task::spawn(async move {
                    let mut klipper = match conn_klipper::KlipperClient::new(
                        id.clone(),
                        printer.clone(),
                        worker_msg_tx,
                        worker_cmd_rx,
                        kill_rx,
                    )
                    .await
                    {
                        Ok(k) => k,
                        Err(e) => {
                            error!("error creating klipper client: {:?}", e);
                            return;
                        }
                    };

                    loop {
                        if let Err(e) = klipper.run().await {
                            error!("error running klipper client: {:?}", e);
                        }
                    }
                });
            }
            PrinterConfig::Prusa(_, printer) => {
                let mut client = conn_prusa::prusa_local::PrusaClientLocal::new(
                    printer.clone(),
                    self.worker_msg_tx.clone(),
                    worker_cmd_rx,
                    kill_rx,
                    None,
                )
                .await?;
                self.worker_cmd_txs.insert(id.clone(), worker_cmd_tx);
                tokio::task::spawn(async move {
                    loop {
                        if let Err(e) = client.run().await {
                            error!("error running prusa client: {:?}", e);
                        }
                    }
                });
            } // PrinterConfig::Octoprint(_, printer) => {
              //     todo!();
              // }
        }

        Ok(())
    }
}

/// handle messages, commands
impl PrinterConnManager {
    async fn handle_printer_msg(&mut self, id: PrinterId, msg: WorkerMsg) -> Result<()> {
        let Some(printer) = self.config.get_printer(&id) else {
            bail!("printer not found: {:?}", id);
        };

        match msg {
            WorkerMsg::StatusUpdate(update) => {
                // debug!("conn manager got status update: {:?}", id);

                let mut state = self.printer_states.entry(id.clone()).or_default();

                let prev_error = state.is_error();
                let prev_state = state.state.clone();

                /// check for new errors and notify
                state.update(update.clone());

                if !prev_error && state.is_error() {
                    warn!("printer error: {:?}", &printer.name().await);

                    if let PrinterState::Error(Some(error)) = &state.state {
                        if let Ok(e) = error.parse::<i64>() {
                            let error = self
                                .error_map
                                .get_error(e as u64)
                                .unwrap_or("Unknown Error");

                            crate::notifications::alert_printer_error(&printer.name().await, error);
                        } else {
                            crate::notifications::alert_printer_error(&printer.name().await, error);
                        }
                    }
                }

                if prev_state != state.state {
                    info!("printer state changed: {:?}", state.state);

                    if prev_state != PrinterState::Disconnected
                        && (state.state == PrinterState::Finished
                            || state.state == PrinterState::Idle)
                    {
                        warn!("sent finish notification");
                        crate::notifications::alert_print_complete(
                            &printer.name().await,
                            state
                                .current_file
                                .as_ref()
                                .unwrap_or(&"Unknown File".to_string()),
                        )
                    }
                }

                #[cfg(feature = "nope")]
                if !prev_error && state.is_error() {
                    info!("printer state changed: {:?}", state.state);

                    /// print just finished, send notification
                    if prev_state != PrinterState::Disconnected && state.state == PrinterState::Idle
                    {
                        warn!("sent finish notification");
                        crate::notifications::alert_print_complete(
                            &printer.name().await,
                            state
                                .current_file
                                .as_ref()
                                .unwrap_or(&"Unknown File".to_string()),
                        )
                    }

                    // /// either print just started, or app was just started
                    // if state.state == PrinterState::Printing && entry.subtask_id.is_some() {
                    //     state.current_task_thumbnail_url = None;
                    // }
                }

                // self.msg_tx.send(PrinterConnMsg::WorkerMsg(id, msg))?;
            }
            WorkerMsg::StatusUpdatePrusa(update) => {
                // debug!("conn manager got status update: {:?}", id);

                let mut state = self.printer_states.entry(id.clone()).or_default();
                state.update_prusa(update);

                // self.msg_tx.send(PrinterConnMsg::WorkerMsg(id, msg))?;
            }
            WorkerMsg::StatusUpdateBambu(update) => {
                let mut state = self.printer_states.entry(id.clone()).or_default();
                state.update_bambu(update);
            }

            WorkerMsg::FetchedThumbnail(id, file, img) => {
                self.msg_tx
                    .send(PrinterConnMsg::NewThumbnail(id, file, img))?;
            }

            WorkerMsg::Connecting => {}
            WorkerMsg::Connected => {}
            WorkerMsg::Reconnecting => {}
            WorkerMsg::Disconnected => {}
        }

        Ok(())
    }

    async fn handle_command(&mut self, cmd: PrinterConnCmd) -> Result<()> {
        match cmd {
            PrinterConnCmd::FetchThumbnail(id, file) => {
                let Some(printer) = self.config.get_printer(&id) else {
                    bail!("printer not found: {:?}", id);
                };
                let Some(state) = self.printer_states.get(&id) else {
                    bail!("printer state not found: {:?}", id);
                };
                helpers::spawn_fetch_thumbnail(
                    printer,
                    state.clone(),
                    id,
                    file,
                    self.worker_msg_tx.clone(),
                )
                .await?;
            }
        }
        Ok(())
    }
}
