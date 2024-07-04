// pub mod bambu_status;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone)]
pub enum PrinterState {
    Idle,
    Busy,
    Printing,
    Paused,
    Error,
    Disconnected,
    Unknown(String),
}

impl Default for PrinterState {
    fn default() -> Self {
        PrinterState::Disconnected
    }
}

impl PrinterState {
    pub fn to_text(&self) -> &'static str {
        match self {
            PrinterState::Idle => "Idle",
            // PrinterState::Finished => "Finished",
            PrinterState::Busy => "Busy",
            PrinterState::Printing => "Printing",
            PrinterState::Error => "Error",
            PrinterState::Paused => "Paused",
            PrinterState::Disconnected => "Disconnected",
            // PrinterState::Unknown(s) => "Unknown",
            PrinterState::Unknown(_) => "Unknown",
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct GenericPrinterState {
    pub state: PrinterState,
    pub nozzle_temp: f32,
    pub bed_temp: f32,
    pub nozzle_temp_target: f32,
    pub bed_temp_target: f32,
    pub layer: Option<u32>,
    pub progress: f32,
    pub time_printing: Option<chrono::Duration>,
    pub time_remaining: Option<chrono::Duration>,
    pub current_file: Option<String>,
}

impl GenericPrinterState {
    pub fn update(&mut self, update: GenericPrinterStateUpdate) {
        for u in update.0 {
            self._update(u);
        }
    }

    fn _update(&mut self, update: PrinterStateUpdate) {
        match update {
            PrinterStateUpdate::State(state) => self.state = state,
            PrinterStateUpdate::NozzleTemp(nozzle, temp, target) => {
                self.nozzle_temp = temp;
                if let Some(target) = target {
                    self.nozzle_temp_target = target;
                }
            }
            PrinterStateUpdate::BedTemp(temp, target) => {
                self.bed_temp = temp;
                if let Some(target) = target {
                    self.bed_temp_target = target;
                }
            }
            PrinterStateUpdate::Progress(progress) => self.progress = progress,
            PrinterStateUpdate::CurrentFile(file) => self.current_file = Some(file),
        }
    }

    #[cfg(feature = "nope")]
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
        match update.layer {
            Some(Some(layer)) => self.layer = Some(layer),
            Some(None) => self.layer = None,
            None => {}
        }
        if let Some(progress) = update.progress {
            self.progress = progress;
        }
        match update.time_printing {
            Some(Some(time_printing)) => self.time_printing = Some(time_printing),
            Some(None) => self.time_printing = None,
            None => {}
        }
        match update.time_remaining {
            Some(Some(time_remaining)) => self.time_remaining = Some(time_remaining),
            Some(None) => self.time_remaining = None,
            None => {}
        }
        if let Some(current_file) = update.current_file {
            self.current_file = Some(current_file);
        }
    }
}

#[derive(Debug, Clone)]
pub enum PrinterStateUpdate {
    State(PrinterState),
    NozzleTemp(usize, f32, Option<f32>),
    BedTemp(f32, Option<f32>),
    Progress(f32),
    CurrentFile(String),
}

#[derive(Debug, Default, Clone)]
pub struct GenericPrinterStateUpdate(pub Vec<PrinterStateUpdate>);

// #[derive(Debug, Default, Clone)]
// pub struct GenericPrinterStateUpdate {
//     pub state: Option<PrinterState>,
//     pub nozzle_temp: Option<f32>,
//     pub nozzle_temp_target: Option<f32>,
//     pub bed_temp: Option<f32>,
//     pub bed_temp_target: Option<f32>,
//     pub layer: Option<Option<u32>>,
//     pub progress: Option<f32>,
//     pub time_printing: Option<Option<chrono::Duration>>,
//     pub time_remaining: Option<Option<chrono::Duration>>,
//     pub current_file: Option<String>,
// }
