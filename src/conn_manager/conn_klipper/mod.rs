mod klipper_types;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use serde_json::Value;
use std::sync::Arc;
use tokio::{net::TcpStream, sync::RwLock};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

use super::{worker_message::WorkerMsg, WorkerCmd};
use crate::{
    config::{printer_config::PrinterConfigKlipper, printer_id::PrinterId},
    status::PrinterStateUpdate,
};
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

    extruders: Vec<String>,
    fans: Vec<String>,

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

        let n_tools = printer_cfg.read().await.tools;
        let mut extruders = vec!["extruder".to_string()];
        for i in 1..n_tools {
            extruders.push(format!("extruder{}", i));
        }

        debug!("extruders: {:#?}", extruders);

        let fans = vec![];

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

            extruders,
            fans,

            current_print: None,
        };

        out.init().await?;

        Ok(out)
    }

    async fn init(&mut self) -> Result<()> {
        // self.list_objects().await?;

        let mut params = serde_json::json!({
            "gcode_move": [
                // "homing_origin",
                "position",
                "gcode_position",
                // "absolute_coordinates",
                ],
            // "gcode_move": null,
            // "toolhead": ["position", "homed_axes"],
            "toolhead": ["homed_axes"],
            // "toolhead": null,
            // "motion_report": null,
            // "idle_timeout": null,
            // "stepper_enable": null,
            "heater_bed": ["temperature", "target", "power"],
            "print_stats": ["state", "filename", "total_duration", "print_duration", "message", "info"],
            "display_status": ["progress"],
            "save_variables": null,
            "fan": null,
        });

        for e in self.extruders.iter() {
            params[e] = serde_json::json!(["temperature", "target", "power"]);
        }

        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "printer.objects.subscribe",
            "params": {
                "objects": params,
            },
            "id": self.get_id(),
        })
        .to_string();

        self.get_variables().await?;

        self.ws_write
            .send(tokio_tungstenite::tungstenite::Message::Text(msg.into()))
            .await?;

        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        loop {
            tokio::select! {
                Some(msg) = self.ws_read.next() => {
                    match msg {
                        Ok(msg) => {
                            self.handle_message(msg).await?;
                        }
                        Err(e) => {
                            error!("error: {:#?}", e);
                            break Err(anyhow!("error: {:#?}", e));
                        }
                    }
                }
                // _ = self.update_timer.tick() => {
                //     // let update = self.get_update().await?;

                //     // debug!("got update: {:#?}", update);
                //     // self.tx.send((self.id.clone(), WorkerMsg::StatusUpdate(update))).unwrap();
                // }
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
        // unimplemented!()
        Ok(())
    }
}

impl KlipperClient {
    async fn handle_message(&mut self, msg: tokio_tungstenite::tungstenite::Message) -> Result<()> {
        match msg {
            tokio_tungstenite::tungstenite::Message::Text(text) => {
                let json = serde_json::from_str(text.as_str());

                let json: serde_json::Value = match json {
                    Ok(json) => json,
                    Err(e) => {
                        // trace!("Failed to parse JSON: {:?}\n{}", msg, e);
                        trace!("Failed to parse JSON: {}", e);
                        return Ok(());
                    }
                };

                // trace!(
                //     "got message: {}",
                //     serde_json::to_string_pretty(&json).unwrap()
                // );
                // self.tx.send((self.id.clone(), WorkerMsg::StatusUpdate(update))).unwrap();

                self.handle_status(json).await?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_status(&mut self, msg: Value) -> Result<()> {
        let status = {
            if let Some(status) = msg.pointer("/params/0").and_then(|s| s.as_object()) {
                status
            } else if let Some(status) = msg.pointer("/result/status").and_then(|s| s.as_object()) {
                status
            } else {
                return Ok(());
            }
        };
        self._handle_status(status).await
    }

    async fn _handle_status(&mut self, status: &serde_json::Map<String, Value>) -> Result<()> {
        // let Some(status) = msg.pointer("/params/0").and_then(|s| s.as_object()) else {
        //     return Ok(());
        // };

        let mut updates: Vec<PrinterStateUpdate> = vec![];

        /// nozzle temps
        for (i, e) in self.extruders.iter().enumerate() {
            if let Some(v) = status.get(e) {
                if let Some(temp) = v
                    .get("temperature")
                    .and_then(|s| s.as_f64().map(|v| v as f32))
                {
                    updates.push(PrinterStateUpdate::NozzleTemp(Some(i), temp));
                }
                if let Some(target) = v.get("target").and_then(|s| s.as_f64().map(|v| v as f32)) {
                    updates.push(PrinterStateUpdate::NozzleTempTarget(Some(i), target));
                }
            }
        }

        /// bed temp
        if let Some(t) = status.get("heater_bed") {
            if let Some(temp) = t
                .get("temperature")
                .and_then(|s| s.as_f64().map(|v| v as f32))
            {
                // debug!("bed temp: {}", temp);
                let target = t.get("target").and_then(|s| s.as_f64().map(|v| v as f32));
                // updates.push(PrinterStateUpdate::BedTemp(temp, target))
                updates.push(PrinterStateUpdate::BedTemp(temp));
            }
            if let Some(target) = t.get("target").and_then(|s| s.as_f64().map(|v| v as f32)) {
                // debug!("bed target: {}", target);
                updates.push(PrinterStateUpdate::BedTempTarget(target));
            }
        }

        /// state
        if let Some(s) = status.get("print_stats") {
            if let Some(state) = s.get("state").and_then(|s| s.as_str()) {
                // debug!("state: {}", state);
                let state = match state {
                    "standby" => crate::status::PrinterState::Idle,
                    "printing" => crate::status::PrinterState::Printing,
                    "paused" => crate::status::PrinterState::Paused,
                    "complete" => crate::status::PrinterState::Finished,
                    "error" => crate::status::PrinterState::Error(None),
                    "cancelled" => {
                        crate::status::PrinterState::Error(Some("Cancelled".to_string()))
                    }
                    _ => crate::status::PrinterState::Unknown(state.to_string()),
                };
                updates.push(PrinterStateUpdate::State(state));
            }
            if let Some(filename) = s.get("filename").and_then(|s| s.as_str()) {
                // debug!("filename: {}", filename);
                updates.push(PrinterStateUpdate::CurrentFile(filename.to_string()));
            }
            // if let Some(p) = s.get("mc_percent").and_then(|s| s.as_f64()) {
            //     // debug!("progress: {}", p);
            //     updates.push(PrinterStateUpdate::Progress(p as f32));
            // }
            // if let Some(t) = s.get("print_duration").and_then(|s| s.as_f64()) {
            //     // debug!("print_duration: {}", t);
            //     updates.push(PrinterStateUpdate::TimeRemaining(t as f32));
            // }
        }

        /// progress
        if let Some(s) = status.get("display_status") {
            if let Some(p) = s.get("progress").and_then(|s| s.as_f64()) {
                // debug!("progress: {}", p);
                updates.push(PrinterStateUpdate::Progress(p as f32 * 100.));
            }
        }

        /// save variables to get current tool
        if let Some(vars) = status.get("save_variables") {
            if let Some(vars) = vars.get("variables") {
                if let Some(t) = vars.get("tool_current") {
                    if let Some(t) = t.as_i64() {
                        // debug!("tool_current: {}", t);
                        if t < 0 {
                            updates.push(PrinterStateUpdate::CurrentTool(None));
                        } else {
                            updates.push(PrinterStateUpdate::CurrentTool(Some(t as usize)));
                        }
                    }
                }
            }
        }

        self.tx.send((
            self.id.clone(),
            WorkerMsg::StatusUpdate(crate::status::GenericPrinterStateUpdate(updates)),
        ))?;

        Ok(())
    }
}

/// helpers
impl KlipperClient {
    pub fn get_id(&mut self) -> usize {
        let id = self.msg_id;
        self.msg_id += 1;
        id
    }

    async fn list_objects(&mut self) -> Result<()> {
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "printer.objects.list",
            "id": self.get_id(),
        })
        .to_string();
        self.ws_write
            .send(tokio_tungstenite::tungstenite::Message::Text(msg.into()))
            .await?;
        Ok(())
    }

    async fn get_variables(&mut self) -> Result<()> {
        let msg = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "printer.objects.query",
                "params": {
                    "objects": {
                        // "save_variables": ["tool_current", "ktcc_state_tool_remap"],
                        "save_variables": null,
                    }
                },
                "id": self.get_id(),
        })
        .to_string();

        self.ws_write
            .send(tokio_tungstenite::tungstenite::Message::Text(msg.into()))
            .await?;
        Ok(())
    }
}
