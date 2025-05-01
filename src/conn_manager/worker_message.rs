use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use crate::{
    config::printer_id::PrinterId,
    status::{GenericPrinterStateUpdate, PrinterState, PrinterStateUpdate},
};

#[derive(Debug, Clone)]
pub enum WorkerMsg {
    StatusUpdate(GenericPrinterStateUpdate),
    StatusUpdatePrusa(super::conn_prusa::prusa_local_types::PrusaStatus),
    StatusUpdateBambu(crate::status::bambu_status::PrinterStateBambu),
    SetBambuType(crate::status::bambu_status::BambuPrinterType),
    FetchedThumbnail(PrinterId, String, Vec<u8>),

    Connecting,
    Connected,
    Reconnecting,
    Disconnected,
}

#[cfg(feature = "nope")]
impl WorkerMsg {
    pub fn from_bambu(msg: super::conn_bambu::message::Message) -> Result<Self> {
        use super::conn_bambu::message::Message;
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

                Self::StatusUpdate(GenericPrinterStateUpdate(out))
            }
            Message::Info(info) => bail!("Unhandled Info message: {:?}", info),
            Message::System(msg) => bail!("Unhandled System message: {:?}", msg),
            Message::Unknown(msg) => bail!("Unhandled Unknown message: {:?}", msg),
            Message::Connecting => Self::Connecting,
            Message::Connected => Self::Connected,
            Message::Reconnecting => Self::Reconnecting,
            Message::Disconnected => Self::Disconnected,
        };
        Ok(out)
    }
}

#[cfg(feature = "nope")]
impl From<super::conn_bambu::message::Message> for WorkerMsg {
    fn from(msg: super::conn_bambu::message::Message) -> Self {}
}
