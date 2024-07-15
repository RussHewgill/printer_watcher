use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use rumqttc::Incoming;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{
    config::{printer_config::PrinterConfigBambu, printer_id::PrinterId},
    conn_manager::{
        conn_bambu::{command::Command, message::Message},
        worker_message::WorkerMsg,
    },
    status::{GenericPrinterStateUpdate, PrinterState, PrinterStateUpdate},
};

pub(super) struct BambuListener {
    pub(super) printer_cfg: Arc<RwLock<PrinterConfigBambu>>,
    pub(super) client: rumqttc::AsyncClient,
    pub(super) eventloop: rumqttc::EventLoop,
    pub(super) tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, WorkerMsg)>,
    pub(super) topic_device_report: String,
    pub(super) topic_device_request: String,
}

impl BambuListener {
    pub fn new(
        printer_cfg: Arc<RwLock<PrinterConfigBambu>>,
        client: rumqttc::AsyncClient,
        eventloop: rumqttc::EventLoop,
        tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, WorkerMsg)>,
        topic_device_report: String,
        topic_device_request: String,
    ) -> Self {
        Self {
            printer_cfg,
            client,
            eventloop,
            tx,
            topic_device_report,
            topic_device_request,
        }
    }

    /// MARK: main event handler
    pub(super) async fn poll_eventloop(&mut self) -> Result<()> {
        use rumqttc::Event;
        loop {
            let event = match self.eventloop.poll().await {
                Ok(event) => event,
                Err(e) => {
                    error!("Error in eventloop: {:?}", e);
                    continue;
                }
            };
            match event {
                Event::Outgoing(event) => {
                    // debug!("outgoing event: {:?}", event);
                }
                Event::Incoming(Incoming::PingResp) => {}
                Event::Incoming(Incoming::ConnAck(c)) => {
                    debug!("got ConnAck: {:?}", c.code);
                    if c.code == rumqttc::ConnectReturnCode::Success {
                        // debug!("Connected to MQTT");
                        self.client
                            .subscribe(&self.topic_device_report, rumqttc::QoS::AtMostOnce)
                            .await?;
                        debug!("sent subscribe to topic");
                        // self.send_pushall().await?;
                    } else {
                        error!("Failed to connect to MQTT: {:?}", c.code);
                    }
                }
                Event::Incoming(Incoming::SubAck(s)) => {
                    debug!("got SubAck");
                    if s.return_codes
                        .iter()
                        .any(|&r| r == rumqttc::SubscribeReasonCode::Failure)
                    {
                        error!("Failed to subscribe to topic");
                    } else {
                        debug!("sending pushall");
                        self.send_pushall().await?;
                        debug!("sent");
                        // debug!("sending get version");
                        // self.send_get_version().await?;
                        // debug!("sent");
                    }
                }
                Event::Incoming(Incoming::Publish(p)) => {
                    // debug!("incoming publish");
                    let msg = crate::conn_manager::conn_bambu::parse::parse_message(&p);
                    // debug!("incoming publish: {:?}", msg);
                    let id = self.printer_cfg.read().await.id.clone();
                    // self.tx.send((id, WorkerMsg::from_bambu(msg)?))?;
                    match bambu_to_workermsg(msg) {
                        Ok(Some(workermsg)) => {
                            self.tx.send((id, workermsg))?;
                        }
                        Ok(None) => {}
                        Err(e) => {
                            error!("Error converting bambu message to worker message: {:?}", e);
                        }
                    }
                }
                Event::Incoming(event) => {
                    debug!("incoming other event: {:?}", event);
                }
            }
        }
    }

    pub(super) async fn send_get_version(&mut self) -> Result<()> {
        let payload = Command::GetVersion.get_payload();

        self.client
            .publish(
                &self.topic_device_request,
                rumqttc::QoS::AtMostOnce,
                false,
                payload,
            )
            .await?;

        Ok(())
    }

    pub(super) async fn send_pushall(&mut self) -> Result<()> {
        let command = Command::PushAll;
        let payload = command.get_payload();

        let qos = rumqttc::QoS::AtMostOnce;
        self.client
            .publish(&self.topic_device_request, qos, false, payload)
            .await?;

        Ok(())
    }
}

fn bambu_to_workermsg(msg: Message) -> Result<Option<WorkerMsg>> {
    let out = match msg {
        // Message::Print(print) => todo!(),
        Message::Print(print) => {
            let mut out = vec![];

            let time_remaining = print
                .print
                .mc_remaining_time
                // .map(|v| Some(chrono::Duration::seconds(v)));
                .and_then(|v| chrono::TimeDelta::new(v * 60, 0));

            if let Some(time_remaining) = time_remaining {
                out.push(PrinterStateUpdate::TimeRemaining(time_remaining));
            }

            match (print.print.layer_num, print.print.total_layer_num) {
                (Some(layer), Some(total)) => {
                    out.push(PrinterStateUpdate::ProgressLayers(
                        layer as u32,
                        total as u32,
                    ));
                }
                _ => {}
            }

            if let Some(f) = print.print.gcode_file {
                out.push(PrinterStateUpdate::CurrentFile(f.clone()));
            }

            let state = if let Some(s) = print.print.gcode_state.as_ref() {
                match s.as_str() {
                    "IDLE" => Some(PrinterState::Idle),
                    "READY" => Some(PrinterState::Idle),
                    "FINISH" => Some(PrinterState::Finished),
                    "CREATED" => Some(PrinterState::Printing),
                    "RUNNING" => Some(PrinterState::Printing),
                    "PREPARE" => Some(PrinterState::Printing),
                    "PAUSE" => {
                        if let Some(e) = print.print.print_error {
                            // Some(PrinterState::Error(format!("Error: {}", e)))
                            Some(PrinterState::Error)
                        } else {
                            Some(PrinterState::Paused)
                        }
                    }
                    "FAILED" => Some(PrinterState::Error),
                    // s => panic!("Unknown gcode state: {}", s),
                    s => Some(PrinterState::Unknown(s.to_string())),
                }
            } else {
                None
            };

            if let Some(state) = state {
                out.push(PrinterStateUpdate::State(state.clone()));
            }

            if let Some(t) = print.print.nozzle_temper {
                out.push(PrinterStateUpdate::NozzleTemp(
                    None,
                    t as f32,
                    print.print.nozzle_target_temper.map(|v| v as f32),
                ));
            }

            if let Some(t) = print.print.bed_temper {
                out.push(PrinterStateUpdate::BedTemp(
                    t as f32,
                    print.print.bed_target_temper.map(|v| v as f32),
                ));
            }

            if let Some(p) = print.print.mc_percent {
                out.push(PrinterStateUpdate::Progress(p as f32));
            }

            Some(WorkerMsg::StatusUpdate(GenericPrinterStateUpdate(out)))
        }
        Message::Info(info) => {
            warn!("Unhandled Info message: {:?}", info);
            None
        }
        Message::System(msg) => {
            warn!("Unhandled System message: {:?}", msg);
            None
        }
        Message::Unknown(msg) => {
            warn!("Unhandled Unknown message: {:?}", msg);
            None
        }
        Message::Connecting => Some(WorkerMsg::Connecting),
        Message::Connected => Some(WorkerMsg::Connected),
        Message::Reconnecting => Some(WorkerMsg::Reconnecting),
        Message::Disconnected => Some(WorkerMsg::Disconnected),
    };
    Ok(out)
}
