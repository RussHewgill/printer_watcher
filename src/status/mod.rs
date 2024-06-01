use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone)]
pub enum PrinterState {
    Idle,
    Printing,
    Paused,
    Error,
    Disconnected,
}

#[derive(Debug, Clone)]
pub struct GenericPrinterState {
    pub state: PrinterState,
    pub nozzle_temp: f32,
    pub bed_temp: f32,
    pub progress: f32,
    pub current_file: Option<String>,
}
