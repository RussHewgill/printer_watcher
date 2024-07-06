use anyhow::{anyhow, bail, ensure, Context, Result};
use futures::StreamExt;
use tracing::{debug, error, info, trace, warn};

use rumqttc::Incoming;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::{printer_config::PrinterConfigKlipper, printer_id::PrinterId};

use super::{worker_message::WorkerMsg, WorkerCmd};

pub(super) struct KlipperClient {
    id: PrinterId,
    printer_cfg: Arc<RwLock<PrinterConfigKlipper>>,
    tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, WorkerMsg)>,
    cmd_rx: tokio::sync::mpsc::UnboundedReceiver<WorkerCmd>,
    kill_rx: tokio::sync::oneshot::Receiver<()>,
}

impl KlipperClient {
    pub(super) fn new(
        id: PrinterId,
        printer_cfg: Arc<RwLock<PrinterConfigKlipper>>,
        tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, WorkerMsg)>,
        cmd_rx: tokio::sync::mpsc::UnboundedReceiver<WorkerCmd>,
        kill_rx: tokio::sync::oneshot::Receiver<()>,
    ) -> Self {
        Self {
            id,
            printer_cfg,
            tx,
            cmd_rx,
            kill_rx,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        unimplemented!()
    }

    #[cfg(feature = "nope")]
    pub async fn run(&mut self) -> Result<()> {
        let host = self.printer_cfg.read().await.host.clone();
        let url = url::Url::parse(&format!("ws://{}:{}/websocket", host, 80))?;

        let (ws_stream, s) = tokio_tungstenite::connect_async(url)
            .await
            .expect("Failed to connect");

        let (write, mut read) = futures::StreamExt::split(ws_stream);

        loop {
            tokio::select! {
                msg = read.next() => {
                    debug!("got message");

                    let Some(Ok(data)) = msg else {
                        continue;
                    };


                    self.handle_msg(data).await?;

                    //
                }
                cmd = self.cmd_rx.recv() => {
                    //
                }
                _ = &mut self.kill_rx => {
                    debug!("got kill command");
                    break Ok(());
                }
                // _ = self.handle_ws_write(write) => {}
            }
        }

        // Ok(())
    }
}

/// handle message, command
impl KlipperClient {
    async fn handle_msg(&self, msg: tokio_tungstenite::tungstenite::Message) -> Result<()> {
        match msg {
            tokio_tungstenite::tungstenite::Message::Text(t) => {
                // debug!("got text message: {}", t);
                let v: serde_json::Value = serde_json::from_str(&t)?;
                debug!("got json: {:?}", v);
                Ok(())
            }
            tokio_tungstenite::tungstenite::Message::Binary(_) => todo!(),
            tokio_tungstenite::tungstenite::Message::Ping(_) => Ok(()),
            tokio_tungstenite::tungstenite::Message::Pong(_) => todo!(),
            tokio_tungstenite::tungstenite::Message::Close(_) => todo!(),
            tokio_tungstenite::tungstenite::Message::Frame(_) => todo!(),
        }
    }
}
