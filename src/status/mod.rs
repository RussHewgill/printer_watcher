use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone)]
pub enum PrinterState {
    Idle,
    Printing,
    Paused,
    Error,
    Disconnected,
}

impl Default for PrinterState {
    fn default() -> Self {
        PrinterState::Disconnected
    }
}

#[derive(Default, Debug, Clone)]
pub struct GenericPrinterState {
    pub state: PrinterState,
    pub nozzle_temp: f32,
    pub bed_temp: f32,
    pub nozzle_temp_target: f32,
    pub bed_temp_target: f32,
    pub progress: f32,
    pub current_file: Option<String>,
}

pub struct PrinterStateBambu {
    //
}
