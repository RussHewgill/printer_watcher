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

impl GenericPrinterState {
    pub fn update(&mut self, update: GenericPrinterStateUpdate) {
        if let Some(state) = update.state {
            self.state = state;
        }
        if let Some(nozzle_temp) = update.nozzle_temp {
            self.nozzle_temp = nozzle_temp;
        }
        if let Some(bed_temp) = update.bed_temp {
            self.bed_temp = bed_temp;
        }
        if let Some(nozzle_temp_target) = update.nozzle_temp_target {
            self.nozzle_temp_target = nozzle_temp_target;
        }
        if let Some(bed_temp_target) = update.bed_temp_target {
            self.bed_temp_target = bed_temp_target;
        }
        if let Some(progress) = update.progress {
            self.progress = progress;
        }
        if let Some(current_file) = update.current_file {
            self.current_file = Some(current_file);
        }
    }
}

#[derive(Debug, Clone)]
pub struct GenericPrinterStateUpdate {
    pub state: Option<PrinterState>,
    pub nozzle_temp: Option<f32>,
    pub bed_temp: Option<f32>,
    pub nozzle_temp_target: Option<f32>,
    pub bed_temp_target: Option<f32>,
    pub progress: Option<f32>,
    pub current_file: Option<String>,
}

pub struct PrinterStateBambu {
    //
}
