use crate::status::GenericPrinterStateUpdate;

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
            Message::Print(print) => todo!(),
            // Message::Print(print) => Self::StatusUpdate(GenericPrinterStateUpdate {
            //     state: None,
            //     nozzle_temp: print.print.nozzle_temper.map(|v| v as f32),
            //     bed_temp: print.print.bed_temper.map(|v| v as f32),
            //     nozzle_temp_target: print.print.nozzle_target_temper.map(|v| v as f32),
            //     bed_temp_target: print.print.bed_target_temper.map(|v| v as f32),
            //     progress: None,
            //     current_file: None,
            // }),
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
