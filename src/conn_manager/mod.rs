pub mod conn_bambu;
pub mod conn_klipper;
pub mod conn_octoprint;
pub mod conn_prusa;
pub mod worker_message;

use std::{collections::HashMap, sync::Arc};

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use dashmap::DashMap;
use tokio::sync::RwLock;
use worker_message::WorkerMsg;

use crate::{
    config::{printer_config::PrinterConfig, printer_id::PrinterId, AppConfig},
    status::GenericPrinterState,
    streaming::StreamCmd,
};
use conn_bambu::message::Message;

/// messages from PrinterConnManager to UI
#[derive(Debug, Clone)]
pub enum PrinterConnMsg {
    WorkerMsg(PrinterId, WorkerMsg),
}

/// messages from UI to PrinterConnManager
#[derive(Debug, Clone)]
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
    worker_msg_tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, WorkerMsg)>,
    /// to receive messages from worker tasks
    worker_msg_rx: tokio::sync::mpsc::UnboundedReceiver<(PrinterId, WorkerMsg)>,

    kill_chans: HashMap<PrinterId, tokio::sync::oneshot::Sender<()>>,

    stream_tx: tokio::sync::mpsc::UnboundedSender<StreamCmd>,
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
        stream_tx: tokio::sync::mpsc::UnboundedSender<StreamCmd>,
    ) -> Self {
        let (worker_msg_tx, mut worker_msg_rx) =
            tokio::sync::mpsc::unbounded_channel::<(PrinterId, WorkerMsg)>();

        // let (worker_cmd_tx, worker_cmd_rx) =
        //     tokio::sync::mpsc::unbounded_channel::<(PrinterId, PrinterConnCmd)>();

        // /// fetch error codes
        // let error_map = ErrorMap::read_or_fetch().await.unwrap_or_default();

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
            // error_map,
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
                    // match cmd {
                    //     PrinterConnCmd::Login(_, _) => debug!("got cmd = Login"),
                    //     _ => debug!("got cmd = {:?}", cmd),
                    // }
                    // self.handle_command(cmd).await?;
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
                let mut klipper = conn_klipper::KlipperClient::new(
                    id.clone(),
                    printer.clone(),
                    self.worker_msg_tx.clone(),
                    worker_cmd_rx,
                    kill_rx,
                );
                self.worker_cmd_txs.insert(id.clone(), worker_cmd_tx);

                tokio::task::spawn(async move {
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
                )?;
                self.worker_cmd_txs.insert(id.clone(), worker_cmd_tx);
                tokio::task::spawn(async move {
                    loop {
                        if let Err(e) = client.run().await {
                            error!("error running prusa client: {:?}", e);
                        }
                    }
                });
            }
            PrinterConfig::Octoprint(_, printer) => {
                todo!();
            }
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

        match &msg {
            WorkerMsg::StatusUpdate(update) => {
                // debug!("conn manager got status update: {:?}", id);

                let mut state = self.printer_states.entry(id.clone()).or_default();
                state.update(update.clone());

                // self.msg_tx.send(PrinterConnMsg::WorkerMsg(id, msg))?;
            }
            WorkerMsg::Connecting => {}
            WorkerMsg::Connected => {}
            WorkerMsg::Reconnecting => {}
            WorkerMsg::Disconnected => {}
        }

        Ok(())
    }

    async fn handle_command(&mut self, cmd: PrinterConnCmd) -> Result<()> {
        Ok(())
    }
}
