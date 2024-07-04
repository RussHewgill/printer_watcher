use crate::status::{GenericPrinterStateUpdate, PrinterState};

#[derive(Debug, Clone)]
pub enum WorkerMsg {
    StatusUpdate(GenericPrinterStateUpdate),

    Connecting,
    Connected,
    Reconnecting,
    Disconnected,
}

impl From<super::conn_bambu::message::Message> for WorkerMsg {
    fn from(msg: super::conn_bambu::message::Message) -> Self {
        use super::conn_bambu::message::Message;
        match msg {
            // Message::Print(print) => todo!(),
            Message::Print(print) => {
                let time_remaining = print
                    .print
                    .mc_remaining_time
                    .map(|v| Some(chrono::Duration::seconds(v)));

                let state = if let Some(s) = print.print.gcode_state.as_ref() {
                    match s.as_str() {
                        "IDLE" => Some(PrinterState::Idle),
                        "READY" => Some(PrinterState::Idle),
                        "FINISH" => Some(PrinterState::Idle),
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

                unimplemented!()
                // Self::StatusUpdate(GenericPrinterStateUpdate {
                //     state,
                //     nozzle_temp: print.print.nozzle_temper.map(|v| v as f32),
                //     bed_temp: print.print.bed_temper.map(|v| v as f32),
                //     nozzle_temp_target: print.print.nozzle_target_temper.map(|v| v as f32),
                //     bed_temp_target: print.print.bed_target_temper.map(|v| v as f32),
                //     progress: None,
                //     current_file: None,
                //     layer: None,
                //     time_printing: None,
                //     time_remaining,
                // })
            }
            Message::Info(_) => todo!(),
            Message::System(_) => todo!(),
            Message::Unknown(_) => todo!(),
            Message::Connecting => Self::Connecting,
            Message::Connected => Self::Connected,
            Message::Reconnecting => Self::Reconnecting,
            Message::Disconnected => Self::Disconnected,
        }
    }
}
