use std::{collections::HashMap, time::Instant};

use serde::{Deserialize, Serialize};

use super::PrinterState;

#[derive(Default, Debug, Clone)]
pub struct PrinterStateBambu {
    /// X1, P1, A1, etc
    pub printer_type: Option<BambuPrinterType>,

    pub state: PrinterState,
    // pub stage: Option<PrintStage>,
    pub stage: Option<i64>,
    pub sub_stage: Option<i64>,

    pub stg: Vec<i64>,
    pub stg_cur: i64,

    // pub last_report: Option<PrinterStatusReport>,
    pub last_report: Option<Instant>,

    pub ams: Option<AmsStatus>,
    pub ams_status: Option<i64>,

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
