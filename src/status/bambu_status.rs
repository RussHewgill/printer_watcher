use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use std::{collections::HashMap, time::Instant};

use serde::{de, Deserialize, Serialize};

use crate::conn_manager::conn_bambu::message::{PrintAms, PrintData, PrintVtTray, VirtualSlotItem};

use super::PrinterState;

mod helpers {
    /// Extracts a specified number of bits from an integer, interpreting it based on the provided base.
    ///
    /// # Arguments
    ///
    /// * `num` - The number containing the bits.
    /// * `start` - The starting bit position (0-indexed, from the right).
    /// * `count` - The number of bits to extract.
    /// * `base` - The base to interpret `num` (10 or 16). If 16, `num` is converted
    ///           to a string and then parsed as hexadecimal (mimicking the C++ behavior).
    ///
    /// # Returns
    ///
    /// The extracted bits as an `i64`, or 0 if the base is unsupported, parsing fails, or an error occurs.
    // fn get_flag_bits_from_int(num: i64, start: u64, count: u64, base: u64) -> Option<i64>
    pub(super) fn get_flag_bits_from_int(num: i64, start: u64, count: u64) -> Option<i64> {
        let base = 10;

        if count == 0 || count > 64 || start >= 64 {
            return None; // Avoid invalid shifts or mask creation
        }

        let value_res: Result<u64, std::num::ParseIntError> = match base {
            10 => Ok(num as u64),
            16 => {
                // Note: This mimics the C++ logic of converting the integer to a string
                // first, then parsing that string as hex. This might not be the
                // intended behavior in all scenarios. Consider if `num` should
                // already represent the hex value directly.
                u64::from_str_radix(&num.to_string(), 16)
            }
            _ => return None, // Unsupported base
        };

        Some(value_res.map_or(0, |value| {
            let mask = if count == 64 {
                u64::MAX // Special case for 64 bits
            } else {
                (1u64 << count) - 1
            };
            // Perform the shift and mask
            ((value >> start) & mask) as i64
        }))
    }
}

#[derive(Default, Debug, Clone)]
pub struct PrinterStateBambu {
    /// X1, P1, A1, etc
    pub printer_type: Option<BambuPrinterType>,

    pub state: PrinterState,
    // pub stage: Option<PrintStage>,
    pub stage: Option<i64>,
    pub sub_stage: Option<i64>,

    pub device: Device,

    pub stg: Vec<i64>,
    pub stg_cur: i64,

    // pub last_report: Option<PrinterStatusReport>,
    pub last_report: Option<Instant>,

    pub ams: Option<AmsStatus>,
    pub ams_status: Option<i64>,

    pub vt_tray: Option<PrintVtTray>,
    pub vir_slot: Option<Vec<VirtualSlotItem>>,

    pub current_file: Option<String>,
    pub subtask_id: Option<String>,
    pub current_task_thumbnail_url: Option<String>,
    // pub gcode_state: Option<GcodeState>,
    pub print_error: Option<PrintError>,
    pub print_percent: Option<i64>,
    pub eta: Option<chrono::DateTime<chrono::Local>>,
    pub is_sdcard_printing: Option<bool>,

    pub wifi_signal: Option<String>,
    pub spd_lvl: Option<i64>,
    // pub print_line_number: Option<String>,
    pub layer_num: Option<i64>,
    pub total_layer_num: Option<i64>,
    pub line_number: Option<i64>,

    pub chamber_light: Option<bool>,

    pub temp_nozzle: Option<f64>,
    pub temp_tgt_nozzle: Option<f64>,
    pub temp_bed: Option<f64>,
    pub temp_tgt_bed: Option<f64>,
    pub temp_chamber: Option<f64>,

    pub fan_gear: Option<i64>,
    pub heatbreak_fan_speed: Option<i64>,
    pub cooling_fan_speed: Option<i64>,
    pub aux_fan_speed: Option<i64>,
    pub chamber_fan_speed: Option<i64>,
}

impl PrinterStateBambu {
    // pub fn update(&mut self, printer: &PrinterConfig, report: &PrintData) -> Result<()> {
    pub fn update(&mut self, report: &PrintData) -> Result<()> {
        self.last_report = Some(Instant::now());

        if let Some(f) = report.gcode_file.as_ref() {
            self.current_file = Some(f.clone());
        }

        if let Some(s) = Self::get_state(report) {
            // if self.state != s && s == PrinterState::Finished {
            //     let _ = notify_rust::Notification::new()
            //         .summary(&format!("Print Complete on {}", printer.name))
            //         .body(&format!(
            //             "{}",
            //             self.current_file
            //                 .as_ref()
            //                 .unwrap_or(&"Unknown File".to_string())
            //         ))
            //         // .icon("thunderbird")
            //         .appname("Bambu Watcher")
            //         .timeout(0)
            //         .show();
            // }
            self.state = s;
        }

        if let Some(s) = report.mc_print_stage.as_ref() {
            // self.stage = Some(s.clone());
            if let Some(s) = s.parse::<i64>().ok() {
                self.stage = Some(s);
            } else {
                warn!("Failed to parse stage: {:?}", s);
            }
        }

        if let Some(s) = report.mc_print_sub_stage {
            self.sub_stage = Some(s);
        }

        if let Some(s) = report.stg.as_ref() {
            self.stg = s.clone();
        }
        if let Some(s) = report.stg_cur {
            self.stg_cur = s;
        }

        // if let Some(s) = report.gcode_state.as_ref() {
        //     self.gcode_state = Some(GcodeState::from_str(s));
        // }

        if let Some(id) = report.subtask_id.as_ref() {
            // debug!("printer name = {:?}", printer.name);
            // debug!("subtask_id = {:?}", id);
            self.subtask_id = Some(id.clone());
        }

        if let Some(p) = report.mc_percent {
            self.print_percent = Some(p);
        }

        if let Some(e) = report.print_error {
            self.print_error = Some(PrintError::from_code(e));
        }

        if let Some(t) = report.mc_remaining_time {
            self.eta = Some(
                chrono::Local::now()
                    + chrono::TimeDelta::new(t as i64 * 60, 0)
                        .context(format!("time delta: {:?}", t))?,
            );
        }

        if let Some(w) = report.wifi_signal.as_ref() {
            self.wifi_signal = Some(w.clone());
        }

        if let Some(s) = report.spd_lvl {
            self.spd_lvl = Some(s);
        }

        if let Some(l) = report.layer_num {
            self.layer_num = Some(l);
        }

        if let Some(t) = report.total_layer_num {
            self.total_layer_num = Some(t);
        }

        if let Some(l) = report.mc_print_line_number.as_ref() {
            if let Some(l) = l.parse::<i64>().ok() {
                self.line_number = Some(l);
            }
        }

        if let Some(lights) = report.lights_report.as_ref() {
            for light in lights.iter() {
                if light.node == "chamber_light" {
                    self.chamber_light = Some(light.mode == "on");
                }
            }
        }

        if let Some(t) = report.nozzle_temper {
            self.temp_nozzle = Some(t);
        }
        if let Some(t) = report.nozzle_target_temper {
            self.temp_tgt_nozzle = Some(t as f64);
        }

        if let Some(t) = report.bed_temper {
            self.temp_bed = Some(t);
        }
        if let Some(t) = report.bed_target_temper {
            self.temp_tgt_bed = Some(t as f64);
        }

        if let Some(t) = report.chamber_temper {
            self.temp_chamber = Some(t);
        }

        // if let Some(t) = report.heatbreak_fan_speed {
        //     self.heatbreak_fan_speed = Some(t);
        // }

        if let Some(t) = self.heatbreak_fan_speed.as_ref() {
            let t = (*t as f32 / 1.5).round() as i64 * 10;
            self.heatbreak_fan_speed = Some(t);
        }

        if let Some(t) = report.cooling_fan_speed.as_ref() {
            if let Some(t) = t.parse::<i64>().ok() {
                // debug!("raw fan speed: {}", t);
                let t = (t as f32 / 1.5).round() as i64 * 10;
                // round(floor(cooling_fan_speed / float(1.5)) * float(25.5));

                // let t = ((t as f32 / 1.5).floor() * 25.5).round() as i64;
                // self.cooling_fan_speed = Some(t);
            }
        }

        if let Some(t) = report.big_fan1_speed.as_ref() {
            if let Some(t) = t.parse::<i64>().ok() {
                // let t = (t as f32 / 1.5).round() as i64 * 10;
                self.aux_fan_speed = Some(t);
            }
        }

        if let Some(t) = report.big_fan2_speed.as_ref() {
            if let Some(t) = t.parse::<i64>().ok() {
                // let t = (t as f32 / 1.5).round() as i64 * 10;
                self.chamber_fan_speed = Some(t);
            }
        }

        #[cfg(feature = "nope")]
        if let Some(gear) = report.fan_gear {
            self.fan_gear = Some(gear);

            self.cooling_fan_speed = Some((gear & 0x00FF0000) >> 16);
            self.aux_fan_speed = Some((gear & 0x0000FF00) >> 8);
            self.chamber_fan_speed = Some((gear & 0x000000FF) >> 0);
        }

        if let Some(s) = report.ams_status {
            self.ams_status = Some(s);
        }

        if let Some(d) = report.device.as_ref() {
            // debug!("device = {:#?}", d);
            self.device = d.clone();
        }

        if let Some(ams) = report.ams.as_ref() {
            // debug!("ams = {:#?}", ams);
            self.ams = Some(self.update_ams(ams, self.ams_status)?);
        }

        if let Some(v) = report.vir_slot.as_ref() {
            // debug!("vir_slot = {:#?}", v);
            self.vir_slot = Some(v.clone());
        }

        if let Some(v) = report.vt_tray.as_ref() {
            // debug!("vt_tray = {:#?}", v);
            self.vt_tray = Some(v.clone());
        }

        Ok(())
    }

    fn update_ams(&mut self, ams: &PrintAms, status_code: Option<i64>) -> Result<AmsStatus> {
        let mut out = self.ams.take().unwrap_or_default();

        // debug!("ams = {:#?}", ams);

        /// 254 if external spool / vt_tray,
        /// otherwise is ((ams_id * 4) + tray_id) for current tray
        /// (ams 2 tray 2 would be (1*4)+1 = 5)
        if let Some(current) = ams.tray_now.as_ref().and_then(|t| t.parse::<u64>().ok()) {
            out.current_tray = if current == 254 {
                Some(AmsCurrentSlot::ExternalSpool)
            } else {
                Some(AmsCurrentSlot::Tray {
                    ams_id: current / 4,
                    tray_id: current % 4,
                })
            };
        } else {
            // out.current_tray = None;
        }

        if let Some(units) = ams.ams.as_ref() {
            for unit in units.iter() {
                let mut slots: [Option<AmsSlot>; 4] = Default::default();

                for i in 0..4 {
                    // let slot = &unit.tray[i];
                    let Some(slot) = unit.tray.get(i) else {
                        continue;
                    };

                    let Some(col) = slot.tray_color.clone() else {
                        slots[i] = None;
                        continue;
                    };
                    let color = egui::Color32::from_hex(&format!("#{}", col))
                        .unwrap_or(egui::Color32::from_rgb(255, 0, 255));

                    slots[i] = Some(AmsSlot {
                        material: slot.tray_type.clone().unwrap_or("Unknown".to_string()),
                        k: slot.k.unwrap_or(0.),
                        color,
                    });
                }

                let id = unit.id.parse::<i64>()?;

                let info = unit.info.as_ref().and_then(|i| i.parse::<i64>().ok());

                // out.units.push(AmsUnit {
                //     id,
                //     humidity: unit.humidity.parse().unwrap_or(0),
                //     temp: unit.temp.parse().unwrap_or(0.),
                //     slots,
                // });
                out.units.insert(
                    id,
                    AmsUnit {
                        id,
                        info,
                        humidity: unit.humidity.parse().unwrap_or(0),
                        temp: unit.temp.parse().unwrap_or(0.),
                        slots,
                    },
                );
            }
        }

        if let Some(bits) = ams.ams_exist_bits.as_ref() {
            out.ams_exist_bits = Some(bits.clone());
        }

        if let Some(bits) = ams.tray_exist_bits.as_ref() {
            out.tray_exist_bits = Some(bits.clone());
        }

        if let Some(now) = ams.tray_now.as_ref() {
            out.tray_now = Some(now.clone());
        }
        if let Some(pre) = ams.tray_pre.as_ref() {
            out.tray_pre = Some(pre.clone());
        }
        if let Some(tar) = ams.tray_tar.as_ref() {
            out.tray_tar = Some(tar.clone());
        }

        if let Some(v) = ams.version {
            out.version = Some(v);
        }

        if let Some(status_code) = status_code {
            if status_code == 768 {
                out.state = None;
            } else {
                let state = crate::utils::parse_ams_status(&out, status_code);

                // match state {
                //     crate::ui::ui_types::AmsState::FilamentChange(_) => {
                //         // out.current_tray
                //     },
                //     _ => {}
                // }

                out.state = Some(state);
            }
        }

        if let Some(ams0) = ams.ams.as_ref().and_then(|a| a.get(0)) {
            out.humidity = Some(ams0.humidity.clone());
        }

        Ok(out)
    }
}

impl PrinterStateBambu {
    pub fn is_error(&self) -> bool {
        matches!(self.state, PrinterState::Error(_))
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    fn get_state(
        report: &crate::conn_manager::conn_bambu::message::PrintData,
    ) -> Option<PrinterState> {
        if let Some(s) = report.gcode_state.as_ref() {
            match s.as_str() {
                "IDLE" => Some(PrinterState::Idle),
                "READY" => Some(PrinterState::Idle),
                "FINISH" => Some(PrinterState::Idle),
                "CREATED" => Some(PrinterState::Printing),
                "RUNNING" => Some(PrinterState::Printing),
                "PREPARE" => Some(PrinterState::Printing),
                "PAUSE" => {
                    if let Some(e) = report.print_error {
                        // Some(PrinterState::Error(format!("Error: {}", e)))
                        Some(PrinterState::Error(Some(format!("{}", e))))
                    } else {
                        Some(PrinterState::Paused)
                    }
                }
                "FAILED" => Some(PrinterState::Error(Some("Failed".to_string()))),
                // s => panic!("Unknown gcode state: {}", s),
                s => Some(PrinterState::Unknown(s.to_string())),
            }
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BambuPrinterType {
    X1C,
    X1E,
    P1P,
    P1S,
    A1,
    A1m,
    H2D,
    Unknown,
}

#[derive(Debug, Clone)]
pub enum PrintError {
    None,
    Unknown(i64),
}

/// https://e.bambulab.com/query.php?
impl PrintError {
    pub fn from_code(code: i64) -> Self {
        match code {
            0 => PrintError::None,
            // 83935249 => PrintError::None,
            _ => PrintError::Unknown(code),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct AmsStatus {
    pub units: HashMap<i64, AmsUnit>,
    pub current_tray: Option<AmsCurrentSlot>,
    // pub id: Option<i64>,
    // pub humidity: Option<i64>,
    // pub temp: Option<i64>,
    // pub slots: [Option<AmsSlot>; 4],
    // pub current_slot: Option<u64>,
    pub ams_exist_bits: Option<String>,
    pub tray_exist_bits: Option<String>,
    pub tray_now: Option<String>,
    pub tray_pre: Option<String>,
    pub tray_tar: Option<String>,
    pub version: Option<i64>,
    pub state: Option<AmsState>,
    pub humidity: Option<String>,
}

impl AmsStatus {
    pub fn is_ams_unload(&self) -> bool {
        self.tray_tar.as_ref().map(|s| s.as_str()) == Some("255")
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AmsCurrentSlot {
    ExternalSpool,
    Tray { ams_id: u64, tray_id: u64 },
}

impl AmsCurrentSlot {
    pub fn is_slot(&self, ams_id: u64, tray_id: u64) -> bool {
        match self {
            AmsCurrentSlot::Tray {
                ams_id: a,
                tray_id: t,
            } => *a == ams_id && *t == tray_id,
            _ => false,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct AmsUnit {
    pub id: i64,
    pub humidity: i64,
    pub temp: f64,
    pub info: Option<i64>,
    pub slots: [Option<AmsSlot>; 4],
}

#[derive(Debug, Default, Clone)]
pub struct AmsSlot {
    pub material: String,
    pub k: f64,
    // pub color: [u8; 3],
    pub color: egui::Color32,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub enum PrintStage {
    Printing = 0,
    AutoBedLeveling = 1,
    HeatbedPreheating = 2,
    SweepingXyMechMode = 3,
    ChangingFilament = 4,
    M400Pause = 5,
    PausedDueToFilamentRunout = 6,
    HeatingHotend = 7,
    CalibratingExtrusion = 8,
    ScanningBedSurface = 9,
    InspectingFirstLayer = 10,
    IdentifyingBuildPlateType = 11,
    CalibratingMicroLidar = 12,
    HomingToolhead = 13,
    CleaningNozzleTip = 14,
    CheckingExtruderTemperature = 15,
    PrintingWasPausedByTheUser = 16,
    PauseOfFrontCoverFalling = 17,
    CalibratingTheMicroLida = 18,
    CalibratingExtrusionFlow = 19,
    PausedDueToNozzleTemperatureMalfunction = 20,
    PausedDueToHeatBedTemperatureMalfunction = 21,
    FilamentUnloading = 22,
    SkipStepPause = 23,
    FilamentLoading = 24,
    MotorNoiseCalibration = 25,
    PausedDueToAmsLost = 26,
    PausedDueToLowSpeedOfTheHeatBreakFan = 27,
    PausedDueToChamberTemperatureControlError = 28,
    CoolingChamber = 29,
    PausedByTheGcodeInsertedByUser = 30,
    MotorNoiseShowoff = 31,
    NozzleFilamentCoveredDetectedPause = 32,
    CutterErrorPause = 33,
    FirstLayerErrorPause = 34,
    NozzleClogPause = 35,
}

impl PrintStage {
    pub fn to_string(&self) -> &'static str {
        match self {
            PrintStage::Printing => "Printing",
            PrintStage::AutoBedLeveling => "Auto Bed Leveling",
            PrintStage::HeatbedPreheating => "Heatbed Preheating",
            PrintStage::SweepingXyMechMode => "Sweeping XY Mech Mode",
            PrintStage::ChangingFilament => "Changing Filament",
            PrintStage::M400Pause => "M400 Pause",
            PrintStage::PausedDueToFilamentRunout => "Paused Due To Filament Runout",
            PrintStage::HeatingHotend => "Heating Hotend",
            PrintStage::CalibratingExtrusion => "Calibrating Extrusion",
            PrintStage::ScanningBedSurface => "Scanning Bed Surface",
            PrintStage::InspectingFirstLayer => "Inspecting First Layer",
            PrintStage::IdentifyingBuildPlateType => "Identifying Build Plate Type",
            PrintStage::CalibratingMicroLidar => "Calibrating Micro Lidar",
            PrintStage::HomingToolhead => "Homing Toolhead",
            PrintStage::CleaningNozzleTip => "Cleaning Nozzle Tip",
            PrintStage::CheckingExtruderTemperature => "Checking Extruder Temperature",
            PrintStage::PrintingWasPausedByTheUser => "Printing Was Paused By The User",
            PrintStage::PauseOfFrontCoverFalling => "Pause Of Front Cover Falling",
            PrintStage::CalibratingTheMicroLida => "Calibrating The Micro Lidar",
            PrintStage::CalibratingExtrusionFlow => "Calibrating Extrusion Flow",
            PrintStage::PausedDueToNozzleTemperatureMalfunction => {
                "Paused Due To Nozzle Temperature Malfunction"
            }
            PrintStage::PausedDueToHeatBedTemperatureMalfunction => {
                "Paused Due To Heat Bed Temperature Malfunction"
            }
            PrintStage::FilamentUnloading => "Filament Unloading",
            PrintStage::SkipStepPause => "Skip Step Pause",
            PrintStage::FilamentLoading => "Filament Loading",
            PrintStage::MotorNoiseCalibration => "Motor Noise Calibration",
            PrintStage::PausedDueToAmsLost => "Paused Due To Ams Lost",
            PrintStage::PausedDueToLowSpeedOfTheHeatBreakFan => {
                "Paused Due To Low Speed Of The Heat Break Fan"
            }
            PrintStage::PausedDueToChamberTemperatureControlError => {
                "Paused Due To Chamber Temperature Control"
            }
            PrintStage::CoolingChamber => "Cooling Chamber",
            PrintStage::PausedByTheGcodeInsertedByUser => "Paused By The Gcode Inserted By User",
            PrintStage::MotorNoiseShowoff => "Motor Noise Showoff",
            PrintStage::NozzleFilamentCoveredDetectedPause => {
                "Nozzle Filament Covered Detected Pause"
            }
            PrintStage::CutterErrorPause => "Cutter Error Pause",
            PrintStage::FirstLayerErrorPause => "First Layer Error Pause",
            PrintStage::NozzleClogPause => "Nozzle Clog Pause",
        }
    }

    pub fn new(layer_num: Option<i64>, code: i64) -> Self {
        let layer_num = layer_num.unwrap_or(0);
        if layer_num > 0 {
            Self::Printing
        } else {
            Self::_new(code)
        }
    }

    fn _new(code: i64) -> Self {
        match code {
            0 => Self::Printing,
            1 => Self::AutoBedLeveling,
            2 => Self::HeatbedPreheating,
            3 => Self::SweepingXyMechMode,
            4 => Self::ChangingFilament,
            5 => Self::M400Pause,
            6 => Self::PausedDueToFilamentRunout,
            7 => Self::HeatingHotend,
            8 => Self::CalibratingExtrusion,
            9 => Self::ScanningBedSurface,
            10 => Self::InspectingFirstLayer,
            11 => Self::IdentifyingBuildPlateType,
            12 => Self::CalibratingMicroLidar,
            13 => Self::HomingToolhead,
            14 => Self::CleaningNozzleTip,
            15 => Self::CheckingExtruderTemperature,
            16 => Self::PrintingWasPausedByTheUser,
            17 => Self::PauseOfFrontCoverFalling,
            18 => Self::CalibratingTheMicroLida,
            19 => Self::CalibratingExtrusionFlow,
            20 => Self::PausedDueToNozzleTemperatureMalfunction,
            21 => Self::PausedDueToHeatBedTemperatureMalfunction,
            22 => Self::FilamentUnloading,
            23 => Self::SkipStepPause,
            24 => Self::FilamentLoading,
            25 => Self::MotorNoiseCalibration,
            26 => Self::PausedDueToAmsLost,
            27 => Self::PausedDueToLowSpeedOfTheHeatBreakFan,
            28 => Self::PausedDueToChamberTemperatureControlError,
            29 => Self::CoolingChamber,
            30 => Self::PausedByTheGcodeInsertedByUser,
            31 => Self::MotorNoiseShowoff,
            32 => Self::NozzleFilamentCoveredDetectedPause,
            33 => Self::CutterErrorPause,
            34 => Self::FirstLayerErrorPause,
            35 => Self::NozzleClogPause,
            _ => Self::Printing,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub enum AmsState {
    /// 0
    Idle,
    /// 1
    FilamentChange(FilamentSwapStep),
    /// 2
    RfidIdentifying,
    /// 3
    Assist,
    /// 4
    Calibration,
    /// 0x10
    SelfCheck,
    /// 0x20
    Debug,
    /// 0xFF
    Unknown,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct Device {
    pub airduct: Option<h2d_airduct::H2DAirDuct>,
    // pub bed_temp: Option<i64>,
    // pub cam: Option<Cam>,
    // pub cham_temp: Option<i64>,
    // pub ext_tool: Option<ExtTool>,
    #[serde(default)]
    pub extruder: Option<h2d_extruder::H2DExtruder>,
    // pub fan: Option<i64>,
    // pub laser: Option<LaserPower>,
    // pub nozzle: Option<Nozzle>,
    // pub plate: Option<Plate>,
    // #[serde(rename = "type")]
    // pub type_field: Option<i64>,
}

pub mod h2d_airduct {
    use anyhow::{anyhow, bail, ensure, Context, Result};
    use tracing::{debug, error, info, trace, warn};

    use serde::{Deserialize, Deserializer};

    use super::helpers::get_flag_bits_from_int;

    #[derive(Debug, Clone)]
    pub struct H2DAirDuct {
        current_mode: i64,
        // modes: Vec<AirMode>,
        pub parts: Vec<AirPart>,
    }

    #[derive(Debug, Clone)]
    pub struct AirPart {
        pub air_type: i64,
        pub id: i64,
        pub func: i64,
        pub state: i64,
        pub range_start: i64,
        pub range_end: i64,
    }

    impl<'de> serde::Deserialize<'de> for H2DAirDuct {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            let s: serde_json::Value = Deserialize::deserialize(d)?;

            let mut parts = vec![];

            let Some(ps) = s.get("parts").and_then(|p| p.as_array()) else {
                warn!("Missing parts in airduct: {:#?}", s);
                return Err(serde::de::Error::custom("Missing parts in airduct"));
            };

            for (i, part) in ps.iter().enumerate() {
                let range = part["range"].as_i64().unwrap_or(0);
                let part = AirPart {
                    air_type: part["type"].as_i64().unwrap_or(0),
                    id: part["id"].as_i64().unwrap_or(0),
                    func: part["func"].as_i64().unwrap_or(0),
                    state: part["state"].as_i64().unwrap_or(0),
                    range_start: get_flag_bits_from_int(range, 0, 16).unwrap_or(0),
                    range_end: get_flag_bits_from_int(range, 16, 16).unwrap_or(0),
                };
                parts.push(part);
            }

            Ok(Self {
                current_mode: s["modeCur"].as_i64().unwrap(),
                parts,
            })
        }
    }
}

pub mod h2d_extruder {
    use anyhow::{anyhow, bail, ensure, Context, Result};
    use tracing::{debug, error, info, trace, warn};

    use serde::{Deserialize, Deserializer};

    use super::helpers::get_flag_bits_from_int;

    #[derive(Debug, Clone)]
    pub struct H2DExtruder {
        /// 0 = Right
        /// 1 = Left
        current_extruder: i64,
        pub switch_state: ExtruderSwitchState,

        pub left: ExtruderInfo,
        pub right: ExtruderInfo,
    }

    impl<'de> serde::Deserialize<'de> for H2DExtruder {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            let s: serde_json::Value = Deserialize::deserialize(d)?;
            // debug!("extruder = {}", serde_json::to_string_pretty(&s).unwrap());

            let right = {
                let Some(right) = s.pointer("/info/0") else {
                    warn!(
                        "Missing right extruder info, s = {}",
                        serde_json::to_string_pretty(&s).unwrap()
                    );
                    return Err(serde::de::Error::custom("Missing right extruder info"));
                };
                // let right = s.pointer("/extruder/info/0").unwrap();
                let info = right["info"].as_i64().unwrap_or(0);
                let temp = right["temp"].as_i64().unwrap_or(0);

                let spre = right["spre"].as_i64().unwrap_or(0);
                let snow = right["snow"].as_i64().unwrap_or(0);
                let star = right["star"].as_i64().unwrap_or(0);

                ExtruderInfo {
                    id: right["id"].as_i64().unwrap(),
                    has_filament: get_flag_bits_from_int(info, 1, 1) == Some(1),
                    buffer_has_filament: get_flag_bits_from_int(info, 2, 1) == Some(1),
                    temp: get_flag_bits_from_int(temp, 0, 16).unwrap(),
                    target_temp: get_flag_bits_from_int(temp, 16, 16).unwrap(),

                    ams_slot_pre: (
                        get_flag_bits_from_int(spre, 0, 8).unwrap(),
                        get_flag_bits_from_int(spre, 8, 8).unwrap(),
                    ),
                    ams_slot_now: (
                        get_flag_bits_from_int(snow, 0, 8).unwrap(),
                        get_flag_bits_from_int(snow, 8, 8).unwrap(),
                    ),
                    ams_slot_tar: (
                        get_flag_bits_from_int(star, 0, 8).unwrap(),
                        get_flag_bits_from_int(star, 8, 8).unwrap(),
                    ),

                    nozzle_id: right["hnow"].as_i64().unwrap_or(0),
                    target_nozzle_id: right["htar"].as_i64().unwrap_or(0),
                }
            };

            let left = {
                let left = s.pointer("/info/1").unwrap_or_default();
                let info = left["info"].as_i64().unwrap_or(0);
                let temp = left["temp"].as_i64().unwrap_or(0);

                let spre = left["spre"].as_i64().unwrap_or(0);
                let snow = left["snow"].as_i64().unwrap_or(0);
                let star = left["star"].as_i64().unwrap_or(0);

                ExtruderInfo {
                    id: left["id"].as_i64().unwrap(),
                    has_filament: get_flag_bits_from_int(info, 1, 1) == Some(1),
                    buffer_has_filament: get_flag_bits_from_int(info, 2, 1) == Some(1),
                    temp: get_flag_bits_from_int(temp, 0, 16).unwrap(),
                    target_temp: get_flag_bits_from_int(temp, 16, 16).unwrap(),

                    ams_slot_pre: (
                        get_flag_bits_from_int(spre, 0, 8).unwrap(),
                        get_flag_bits_from_int(spre, 8, 8).unwrap(),
                    ),
                    ams_slot_now: (
                        get_flag_bits_from_int(snow, 0, 8).unwrap(),
                        get_flag_bits_from_int(snow, 8, 8).unwrap(),
                    ),
                    ams_slot_tar: (
                        get_flag_bits_from_int(star, 0, 8).unwrap(),
                        get_flag_bits_from_int(star, 8, 8).unwrap(),
                    ),

                    nozzle_id: left["hnow"].as_i64().unwrap_or(0),
                    target_nozzle_id: left["htar"].as_i64().unwrap_or(0),
                }
            };

            let state = s.pointer("/state").unwrap().as_i64().unwrap();

            let total_extruder_count = get_flag_bits_from_int(state, 0, 4).unwrap();
            let current_extruder = get_flag_bits_from_int(state, 4, 4).unwrap();
            let target_extruder = get_flag_bits_from_int(state, 8, 4).unwrap();

            let switch_state = get_flag_bits_from_int(state, 12, 3).unwrap();

            Ok(Self {
                current_extruder,
                switch_state: ExtruderSwitchState::from_code(switch_state),

                left,
                right,
            })
        }
    }

    impl H2DExtruder {
        pub fn get_current(&self) -> Option<&ExtruderInfo> {
            match self.current_extruder {
                0 => Some(&self.right),
                1 => Some(&self.left),
                _ => None,
                // _ => panic!("Invalid current extruder: {}", self.current_extruder),
            }
        }

        pub fn get_other(&self) -> Option<&ExtruderInfo> {
            match self.current_extruder {
                0 => Some(&self.left),
                1 => Some(&self.right),
                _ => None,
                // _ => panic!("Invalid current extruder: {}", self.current_extruder),
            }
        }

        pub fn current_extruder(&self) -> i64 {
            self.current_extruder
        }
    }

    #[derive(Debug, Clone)]
    pub struct ExtruderInfo {
        pub id: i64,
        pub has_filament: bool,
        pub buffer_has_filament: bool,
        // pub nozzle_exist: bool,
        pub temp: i64,
        pub target_temp: i64,
        pub ams_slot_pre: (i64, i64),
        pub ams_slot_now: (i64, i64),
        pub ams_slot_tar: (i64, i64),
        pub nozzle_id: i64,
        pub target_nozzle_id: i64,
        // ams_stat: (),
    }

    #[derive(Debug, Clone, Copy)]
    pub enum ExtruderSwitchState {
        Idle,
        Busy,
        Switching,
        Failed,
        Other(i64),
    }

    impl ExtruderSwitchState {
        pub fn from_code(code: i64) -> Self {
            match code {
                0 => Self::Idle,
                1 => Self::Busy,
                2 => Self::Switching,
                3 => Self::Failed,
                other => Self::Other(other),
            }
        }
    }

    #[cfg(feature = "nope")]
    /// 0 = Right
    /// 1 = Left
    #[derive(Debug, Clone, Deserialize)]
    pub struct Extruder {
        pub info: Vec<ExtruderInfo>,
        state: Option<i64>,
    }

    #[cfg(feature = "nope")]
    impl H2DExtruder {
        pub fn get_state(&self) -> Option<H2DNozzleState> {
            // seen:
            // Left:
            // 2        0b0000_0000_0000_0010 (?)
            // 33042    0b1000_0001_0001_0010 (AMS slot 1)
            // Right:
            // 274      0b0000_0001_0001_0010

            match self.state {
                Some(2) => Some(H2DNozzleState::Left),
                Some(274) => Some(H2DNozzleState::Right),
                Some(s) => Some(H2DNozzleState::Other(s)),
                None => None,
            }
        }
    }

    // #[derive(Debug, Clone, Copy)]
    // pub enum H2DNozzleState {
    //     Left,
    //     Right,
    //     Other(i64),
    // }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub enum FilamentSwapStep {
    Idling,
    HeatNozzle,
    CutFilament,
    PullBackCurrentFilament,
    PushNewFilament,
    PurgeOldFilament,
    FeedFilament,
    ConfirmExtruded,
    CheckFilamentPosition,
}
