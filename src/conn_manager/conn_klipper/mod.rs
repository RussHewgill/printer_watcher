mod klipper_types;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use futures::{
    stream::{SplitSink, SplitStream},
    StreamExt,
};
use std::sync::Arc;
use tokio::{net::TcpStream, sync::RwLock};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

use super::{worker_message::WorkerMsg, WorkerCmd};
use crate::config::{printer_config::PrinterConfigKlipper, printer_id::PrinterId};
use klipper_types::metadata::KlipperMetadata;

// pub(super) struct KlipperClient {
pub struct KlipperClient {
    id: PrinterId,
    printer_cfg: Arc<RwLock<PrinterConfigKlipper>>,
    // client: reqwest::Client,
    // rpc_client: jsonrpc::Client,
    // rpc_client: jsonrpsee::ws_client::WsClient,
    pub(super) ws_write: SplitSink<
        WebSocketStream<MaybeTlsStream<TcpStream>>,
        tokio_tungstenite::tungstenite::Message,
    >,
    pub(super) ws_read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    msg_id: usize,

    tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, WorkerMsg)>,
    cmd_rx: tokio::sync::mpsc::UnboundedReceiver<WorkerCmd>,
    kill_rx: tokio::sync::oneshot::Receiver<()>,
    update_timer: tokio::time::Interval,

    current_print: Option<(String, KlipperMetadata)>,
}

/// new, run
impl KlipperClient {
    pub async fn new(
        id: PrinterId,
        printer_cfg: Arc<RwLock<PrinterConfigKlipper>>,
        tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, WorkerMsg)>,
        cmd_rx: tokio::sync::mpsc::UnboundedReceiver<WorkerCmd>,
        kill_rx: tokio::sync::oneshot::Receiver<()>,
    ) -> Result<Self> {
        let url = printer_cfg.read().await.host.clone();
        let url = format!("ws://{}:7125/websocket", url);

        let (ws_stream, _) = connect_async(&url).await?;
        debug!("Connected to {}", &url);

        let (mut ws_write, mut ws_read) = ws_stream.split();

        let mut out = Self {
            id,
            printer_cfg,
            ws_write,
            ws_read,
            msg_id: 1,

            tx,
            cmd_rx,
            kill_rx,
            update_timer: tokio::time::interval(tokio::time::Duration::from_secs(1)),

            current_print: None,
        };

        out.init().await?;

        Ok(out)
    }

    async fn init(&mut self) -> Result<()> {
        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        loop {
            tokio::select! {
                _ = self.update_timer.tick() => {
                    let update = self.get_update().await?;

                    debug!("got update: {:#?}", update);
                    // self.tx.send((self.id.clone(), WorkerMsg::StatusUpdate(update))).unwrap();
                }
                cmd = self.cmd_rx.recv() => {
                    debug!("got worker command");
                    //
                }
                _ = &mut self.kill_rx => {
                    debug!("got kill command");
                    break Ok(());
                }
            }
        }
    }

    async fn get_update(&mut self) -> Result<()> {
        unimplemented!()
    }
}

/// helpers
impl KlipperClient {
    async fn list_objects(&mut self) -> Result<()> {
        unimplemented!()
    }
}
