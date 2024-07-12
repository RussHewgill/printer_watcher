pub mod klipper_types;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use ffmpeg_next::codec::debug;
use futures::StreamExt;
use rumqttc::Incoming;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::{printer_config::PrinterConfigKlipper, printer_id::PrinterId};

use super::{worker_message::WorkerMsg, WorkerCmd};

// pub(super) struct KlipperClient {
pub struct KlipperClient {
    id: PrinterId,
    printer_cfg: Arc<RwLock<PrinterConfigKlipper>>,
    client: reqwest::Client,
    // rpc_client: jsonrpc::Client,
    // rpc_client: jsonrpsee::ws_client::WsClient,
    tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, WorkerMsg)>,
    cmd_rx: tokio::sync::mpsc::UnboundedReceiver<WorkerCmd>,
    kill_rx: tokio::sync::oneshot::Receiver<()>,
    update_timer: tokio::time::Interval,
}

impl KlipperClient {
    // pub(super) async fn new(
    pub async fn new(
        id: PrinterId,
        printer_cfg: Arc<RwLock<PrinterConfigKlipper>>,
        tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, WorkerMsg)>,
        cmd_rx: tokio::sync::mpsc::UnboundedReceiver<WorkerCmd>,
        kill_rx: tokio::sync::oneshot::Receiver<()>,
    ) -> Result<Self> {
        let client = reqwest::ClientBuilder::new().build()?;

        let url = format!("ws://{}:{}/websocket", printer_cfg.try_read()?.host, 80);

        // let rpc_client = jsonrpsee::ws_client::WsClientBuilder::default()
        //     .build(&url)
        //     .await?;

        // let t = jsonrpc::simple_http::SimpleHttpTransport::builder()
        //     .url(&url)?
        //     // .auth(user, Some(pass))
        //     .build();

        // let rpc_client = jsonrpc::Client::with_transport(t);

        let update_timer = tokio::time::interval(tokio::time::Duration::from_secs(1));

        Ok(Self {
            id,
            printer_cfg,
            client,
            // rpc_client,
            tx,
            cmd_rx,
            kill_rx,
            update_timer,
        })
    }

    #[cfg(feature = "nope")]
    pub async fn run(&mut self) -> Result<()> {
        loop {
            //
        }
    }

    /// jsonrpsee
    #[cfg(feature = "nope")]
    pub async fn run(&mut self) -> Result<()> {
        use jsonrpsee::core::client::ClientT;

        let mut subs = self.init_subscriptions().await?;

        // #[cfg(feature = "nope")]
        loop {
            let mut futures = subs.iter_mut().map(|sub| Box::pin(sub.next()));

            tokio::select! {
                _ = self.update_timer.tick() => {
                    debug!("updating");
                    self.update().await?;
                }
                (Some(msg), _, _) = futures::future::select_all(futures) => {

                    // let msg = serde_json::to_string_pretty(&msg)?;

                    debug!("got message: {:#?}", msg);
                    unimplemented!()
                }
                cmd = self.cmd_rx.recv() => {
                    debug!("got worker command");
                    //
                }
                _ = &mut self.kill_rx => {
                    debug!("got kill command");
                    break Ok(());
                }
                // _ = self.handle_ws_write(write) => {}
            }
        }

        //
    }

    /// tungtenite
    // #[cfg(feature = "nope")]
    pub async fn run(&mut self) -> Result<()> {
        let host = self.printer_cfg.read().await.host.clone();
        // let url = url::Url::parse(&format!("ws://{}:{}/websocket", host, 80))?;
        let url = format!("ws://{}:{}/websocket", host, 80);

        let (mut ws_stream, s) = tokio_tungstenite::connect_async(url)
            .await
            .expect("Failed to connect");

        let conn_id = self.get_conn_id(&mut ws_stream).await?;

        let (mut write, mut read) = futures::StreamExt::split(ws_stream);

        self.init_subscriptions(&mut write).await?;

        loop {
            tokio::select! {
                _ = self.update_timer.tick() => {
                    debug!("updating");
                    self.update().await?;
                }
                msg = read.next() => {
                    let Some(Ok(data)) = msg else {
                        continue;
                    };


                    self.handle_msg(data).await?;

                    //
                }
                cmd = self.cmd_rx.recv() => {
                    debug!("got worker command");
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

/// get update
impl KlipperClient {
    async fn get_conn_id(
        &self,
        ws_stream: &mut tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    ) -> Result<String> {
        let params = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "server.connection.identify",
            "params": {
                "client_name": "printer_watcher",
                "version": "0.0.1",
                "type": "web",
                // "access_token": "<base64 encoded token>",
                // "api_key": "<system API key>"
            },
            "id": 10,
        });
        let msg = tokio_tungstenite::tungstenite::Message::Text(serde_json::to_string(&params)?);

        unimplemented!()
    }

    #[cfg(feature = "nope")]
    async fn init_subscriptions(&self) -> Result<Vec<Subscription<serde_json::Value>>> {
        // let url = format!(
        //     "http://{}:{}/printer/objects/subscribe?connection_id=123456789&gcode_move&extruder",
        //     self.printer_cfg.try_read()?.host,
        //     80
        // );

        let mut out = vec![];

        let params = serde_json::json!({
            "objects": {
                "toolhead": ["position", "status"],
            }
        });

        debug!("params = {}", serde_json::to_string_pretty(&params)?);

        let params = jsonrpsee::rpc_params![params];

        let sub1: jsonrpsee::core::client::Subscription<serde_json::Value> =
            jsonrpsee::core::client::SubscriptionClientT::subscribe(
                &self.rpc_client,
                "printer.objects.subscribe",
                params,
                "unsubscribe_all",
            )
            .await?;
        out.push(sub1);

        // self.subscribe().await?;
        // Ok(())
        // Ok(out)
        unimplemented!()
    }

    async fn init_subscriptions(
        &self,
        write: &mut futures::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
            tokio_tungstenite::tungstenite::Message,
        >,
    ) -> Result<()> {
        unimplemented!()
    }

    #[cfg(feature = "nope")]
    async fn subscribe(&self) -> Result<()> {
        let params = jsonrpc::arg(serde_json::json!({
            "objects": {
                "toolhead": ["position", "status"],
            }
        }));
        let req = self
            .rpc_client
            .build_request("printer.objects.subscribe", Some(&params));

        let response: serde_json::Value = self
            .rpc_client
            .send_request(req)
            .expect("send_request failed")
            .result()?;

        debug!("response = {}", serde_json::to_string_pretty(&response)?);

        Ok(())
    }

    async fn update(&self) -> Result<()> {
        // let url = "printer/objects/list";
        let url = "printer/objects/query?webhooks&virtual_sdcard&print_stats";

        let resp: serde_json::Value = self.get_response(url).await?;

        debug!("resp = {}", serde_json::to_string_pretty(&resp)?);

        Ok(())
    }

    pub async fn get_response<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        let printer = self.printer_cfg.read().await;

        let url = format!("http://{}:{}/{}", printer.host, 80, url);
        let req = self.client.get(&url);

        // let req = self.set_headers(req).await?;
        let resp = req.send().await?;

        if !resp.status().is_success() {
            debug!("status {:#?}", resp.status());
            bail!("Failed to get response, url = {}", url);
        }

        // let j: serde_json::Value = resp.json().await?;
        // debug!("json = {:#?}", j);

        Ok(resp.json().await?)
    }
}

/// handle message, command
impl KlipperClient {
    async fn handle_msg(&self, msg: tokio_tungstenite::tungstenite::Message) -> Result<()> {
        match msg {
            tokio_tungstenite::tungstenite::Message::Text(t) => {
                // debug!("got text message: {}", t);

                match serde_json::from_str::<klipper_types::Rpc>(&t) {
                    Ok(rpc) => {
                        debug!("got RPC, method = {}", rpc.method);
                        debug!("params = {:#?}", rpc.params);
                    }
                    Err(e) => {
                        let v: serde_json::Value = serde_json::from_str(&t)?;
                        debug!("got json: {}", serde_json::to_string_pretty(&v)?);
                    }
                };

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
