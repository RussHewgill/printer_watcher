pub mod bambu_status;
// pub mod bambu_status2;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PrinterState {
    Idle,
    Finished,
    Busy,
    Printing,
    Paused,
    Error(Option<String>),
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
            PrinterState::Finished => "Finished",
            PrinterState::Busy => "Busy",
            PrinterState::Printing => "Printing",
            PrinterState::Error(_) => "Error",
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
    pub wifi_signal: Option<i32>,
    pub nozzle_temp: f32,
    pub bed_temp: f32,
    pub nozzle_temp_target: f32,
    pub bed_temp_target: f32,
    pub nozzle_temps: HashMap<usize, f32>,
    pub nozzle_temps_target: HashMap<usize, f32>,
    pub current_tool: Option<usize>,
    pub fan_speed: f32,
    pub layer: Option<(u32, u32)>,
    pub progress: f32,
    pub time_printing: Option<chrono::Duration>,
    pub time_remaining: Option<chrono::Duration>,
    pub current_file: Option<String>,
    // pub thumbnail_path: Option<String>,
    pub state_prusa: Option<crate::conn_manager::conn_prusa::prusa_local_types::PrusaStatus>,
    pub state_bambu: Option<bambu_status::PrinterStateBambu>,
}

impl GenericPrinterState {
    pub fn is_error(&self) -> bool {
        matches!(self.state, PrinterState::Error(_))
    }
}

impl GenericPrinterState {
    pub fn update_prusa(
        &mut self,
        update: crate::conn_manager::conn_prusa::prusa_local_types::PrusaStatus,
    ) {
        self.state_prusa = Some(update);
    }

    pub fn update_bambu(&mut self, update: bambu_status::PrinterStateBambu) {
        self.state_bambu = Some(update);
    }

    pub fn update(&mut self, update: GenericPrinterStateUpdate) {
        for u in update.0 {
            self._update(u);
        }
    }

    fn _update(&mut self, update: PrinterStateUpdate) {
        match update {
            PrinterStateUpdate::State(state) => self.state = state,

            // PrinterStateUpdate::NozzleTemp(None, temp, target) => {
            //     self.nozzle_temp = temp;
            //     if let Some(target) = target {
            //         self.nozzle_temp_target = target;
            //     }
            // }
            // PrinterStateUpdate::NozzleTemp(Some(idx), temp, target) => {
            //     self.nozzle_temps.insert(idx, temp);
            //     if let Some(target) = target {
            //         debug!("Nozzle Target Temp {}: {}", idx, target);
            //         if target <= 0.0 {
            //             self.nozzle_temps_target.remove(&idx);
            //         } else {
            //             self.nozzle_temps_target.insert(idx, target);
            //         }
            //         // } else {
            //         // self.nozzle_temps_target.remove(&idx);
            //     }
            // }
            // PrinterStateUpdate::BedTemp(temp, target) => {
            //     self.bed_temp = temp;
            //     if let Some(target) = target {
            //         self.bed_temp_target = target;
            //     }
            // }
            PrinterStateUpdate::NozzleTemp(None, temp) => self.nozzle_temp = temp,
            PrinterStateUpdate::NozzleTemp(Some(t), temp) => {
                self.nozzle_temps.insert(t, temp);
            }

            PrinterStateUpdate::NozzleTempTarget(None, temp) => self.nozzle_temp_target = temp,
            PrinterStateUpdate::NozzleTempTarget(Some(t), temp) => {
                if temp <= 0.0 {
                    self.nozzle_temps_target.remove(&t);
                } else {
                    self.nozzle_temps_target.insert(t, temp);
                }
            }

            PrinterStateUpdate::BedTemp(temp) => self.bed_temp = temp,
            PrinterStateUpdate::BedTempTarget(temp) => self.bed_temp_target = temp,

            PrinterStateUpdate::Progress(progress) => self.progress = progress,
            PrinterStateUpdate::ProgressLayers(current, total) => {
                self.layer = Some((current, total))
            }
            PrinterStateUpdate::CurrentFile(file) => self.current_file = Some(file),
            PrinterStateUpdate::TimeRemaining(time) => self.time_remaining = Some(time),
            PrinterStateUpdate::WifiSignal(strength) => self.wifi_signal = Some(strength),
            // _ => tracing::warn!("GenericPrinterState::_update TODO: {:?}", update),
            PrinterStateUpdate::CurrentTool(tool) => self.current_tool = tool,
            PrinterStateUpdate::FanSetting(s) => self.fan_speed = s,
            PrinterStateUpdate::Duration(_) => todo!(),
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
    NozzleTemp(Option<usize>, f32),
    NozzleTempTarget(Option<usize>, f32),
    BedTemp(f32),
    BedTempTarget(f32),
    Progress(f32),
    ProgressLayers(u32, u32),
    Duration(chrono::Duration),
    TimeRemaining(chrono::Duration),
    CurrentFile(String),
    WifiSignal(i32),
    CurrentTool(Option<usize>),
    FanSetting(f32),
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
