pub mod klipper_types;

use anyhow::{anyhow, bail, ensure, Context, Result};
use klipper_types::metadata::KlipperMetadata;
use serde_json::Value;
use tracing::{debug, error, info, trace, warn};

use futures::StreamExt;
use rumqttc::Incoming;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(feature = "rtsp")]
use ffmpeg_next::codec::debug;

use jsonrpsee::core::{
    client::{ClientT, SubscriptionClientT},
    params::ObjectParams,
};

use crate::{
    config::{printer_config::PrinterConfigKlipper, printer_id::PrinterId},
    status::{GenericPrinterStateUpdate, PrinterState, PrinterStateUpdate},
};

use super::{worker_message::WorkerMsg, WorkerCmd};

// pub(super) struct KlipperClient {
pub struct KlipperClient {
    id: PrinterId,
    printer_cfg: Arc<RwLock<PrinterConfigKlipper>>,
    client: reqwest::Client,
    // rpc_client: jsonrpc::Client,
    rpc_client: jsonrpsee::ws_client::WsClient,
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
        let client = reqwest::ClientBuilder::new().build()?;

        let url = format!("ws://{}:{}/websocket", printer_cfg.try_read()?.host, 80);

        let rpc_client = jsonrpsee::ws_client::WsClientBuilder::default()
            .build(&url)
            .await?;

        let update_timer = tokio::time::interval(tokio::time::Duration::from_secs(1));

        Ok(Self {
            id,
            printer_cfg,
            client,
            rpc_client,
            tx,
            cmd_rx,
            kill_rx,
            update_timer,

            current_print: None,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut params = jsonrpsee::core::params::ObjectParams::new();
        params.insert(
            "objects",
            serde_json::json!({
                // "gcode_move": serde_json::Value::Null,
                // "toolhead": ["position", "status"],
                "extruder": ["temperature", "target"],
                "heater_bed": ["temperature", "target"],
                // "print_stats": ["filename", "total_duration", "print_duration", "state", "message", ],
                "print_stats": Value::Null,
                "webhooks": Value::Null,
                "virtual_sdcard": Value::Null,
            }),
        )?;

        let mut params2 = ObjectParams::new();
        // params2.insert(
        //     "objects"
        // );

        let res = self
            .rpc_client
            .request("printer.objects.list", params2)
            .await?;

        debug!("list = {}", res);

        loop {
            tokio::select! {
                _ = self.update_timer.tick() => {
                    // self.get_update(params.clone(), &mut current_thumbnail, &mut current_metadata).await?;
                    let update = self.get_update(params.clone()).await?;

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
}

/// get update
impl KlipperClient {
    async fn get_update(
        &mut self,
        params: jsonrpsee::core::params::ObjectParams,
    ) -> Result<GenericPrinterStateUpdate> {
        let res: klipper_types::StatusUpdateResponse = self
            .rpc_client
            .request("printer.objects.query", params.clone())
            .await?;

        let mut out = vec![];

        let res = res.status;
        // debug!("res = {:#?}", res);

        out.push(PrinterStateUpdate::NozzleTemp(
            None,
            res.extruder.temperature as f32,
            if res.extruder.target > 0.0 {
                Some(res.extruder.target as f32)
            } else {
                None
            },
        ));

        out.push(PrinterStateUpdate::BedTemp(
            res.heater_bed.temperature as f32,
            if res.heater_bed.target > 0.0 {
                Some(res.heater_bed.target as f32)
            } else {
                None
            },
        ));

        let state: PrinterState = match res.print_stats.state.as_str() {
            "standby" => PrinterState::Idle,
            "printing" => PrinterState::Printing,
            "paused" => PrinterState::Paused,
            "error" => PrinterState::Error(Some(res.print_stats.message.clone())),
            "complete" => PrinterState::Idle,
            _ => PrinterState::Unknown(res.print_stats.state.clone()),
        };
        out.push(PrinterStateUpdate::State(state.clone()));

        if !matches!(state, PrinterState::Idle) {
            if self.current_print.as_ref().map(|(f, _)| f) != Some(&res.print_stats.filename) {
                self.get_print_info(&res.print_stats.filename).await?;
                debug!("pushing current file");
                out.push(PrinterStateUpdate::CurrentFile(
                    res.print_stats.filename.clone(),
                ));
            }

            // let t = self.current_print.as_ref().unwrap().1.estimated_time;
            // debug!("estimated_time = {}", t / 60);
            // debug!("print_duration = {}", res.print_stats.print_duration / 60.);
            // debug!("total_duration = {}", res.print_stats.total_duration / 60.);

            // let time_remaining = res.print_stats.total_duration - res.print_stats.print_duration;

            match self.current_print {
                Some((_, ref md)) => {
                    let rem = md.estimated_time - res.print_stats.print_duration as i64;
                    out.push(PrinterStateUpdate::TimeRemaining(
                        chrono::Duration::new(rem, 0).unwrap(),
                    ));
                }
                None => {}
            }

            // out.push(PrinterStateUpdate::TimeRemaining(
            //     chrono::Duration::new(time_remaining as i64, 0).unwrap(),
            // ));

            out.push(PrinterStateUpdate::Progress(
                (res.virtual_sdcard.progress as f32 * 1000.0).round() / 10.0,
            ));
            match (
                res.print_stats.info.current_layer,
                res.print_stats.info.total_layer,
            ) {
                (Some(current), Some(total)) => {
                    out.push(PrinterStateUpdate::ProgressLayers(
                        current as u32,
                        total as u32,
                    ));
                }
                _ => {}
            }
        } else {
            self.current_print = None;
        }

        Ok(GenericPrinterStateUpdate(out))
    }

    async fn get_print_info(&mut self, filename: &str) -> Result<()> {
        let md = self.get_metadata(filename).await?;

        // debug!("md = {}", serde_json::to_string_pretty(&md)?);

        let mut thumbs = md.thumbnails.clone();
        thumbs.sort_by_key(|x| x.size);
        // debug!("thumbs = {:#?}", thumbs);

        let path = &thumbs[thumbs.len() - 1].relative_path;
        // debug!("path = {}", path);
        self.get_thumbnail(path).await?;

        self.current_print = Some((filename.to_string(), md));

        Ok(())
    }

    async fn get_metadata(&self, filename: &str) -> Result<KlipperMetadata> {
        let mut params = jsonrpsee::core::params::ObjectParams::new();
        params.insert("filename", filename)?;

        let res = self
            .rpc_client
            .request("server.files.metadata", params.clone())
            .await?;

        // debug!("metadata = {}", serde_json::to_string_pretty(&res)?);

        Ok(res)
    }

    async fn get_thumbnail(&self, path: &str) -> Result<()> {
        let url = format!(
            "http://{}/server/files/gcodes/.thumbs/{}",
            self.printer_cfg.try_read()?.host,
            path
        );

        let resp = self.client.get(&url).send().await?;

        let bytes = resp.bytes().await?;

        self.tx
            .send((
                self.id.clone(),
                WorkerMsg::FetchedThumbnail(self.id.clone(), path.to_string(), bytes.to_vec()),
            ))
            .unwrap();

        Ok(())
    }

    async fn get_conn_id(&self) -> Result<u64> {
        let mut params = jsonrpsee::core::params::ObjectParams::new();
        params.insert("client_name", "printer_watcher")?;
        params.insert("version", "0.1.0")?;
        params.insert("type", "other")?;
        params.insert("url", "http://github.com/arksine/moontest")?;

        let res: serde_json::Value = self
            .rpc_client
            .request("server.connection.identify", params)
            .await?;
        let id = res["connection_id"].as_u64().unwrap();
        // debug!("id = {:?}", id);
        Ok(id)
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
#[cfg(feature = "nope")]
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
