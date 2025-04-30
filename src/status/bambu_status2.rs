use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct PrinterStateBambu {
    pub printer_type: BambuPrinterType,
    pub status: PrintStatus,
}

impl PrinterStateBambu {
    pub fn update(&mut self, status: PrintStatus) -> Result<()> {
        self.status = status;
        Ok(())
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

#[derive(Debug, Default, Clone, Deserialize)]
pub struct PrintStatus {
    pub print: PrintData,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct PrintData {
    #[serde(rename = "2D")]
    pub print_2d: Option<Print2D>,
    #[serde(rename = "3D")]
    pub print_3d: Option<Print3D>,
    pub ams: Option<Ams>,
    #[serde(rename = "ams_rfid_status")]
    pub ams_rfid_status: Option<i64>,
    pub ams_status: Option<i64>,
    pub ap_err: Option<i64>,
    pub aux: Option<String>,
    pub aux_part_fan: Option<bool>,
    pub batch_id: Option<i64>,
    pub bed_target_temper: Option<f64>,
    pub bed_temper: Option<f64>,
    pub big_fan1_speed: Option<String>,
    pub big_fan2_speed: Option<String>,
    pub cali_version: Option<i64>,
    pub canvas_id: Option<i64>,
    pub cfg: Option<String>,
    pub chamber_temper: Option<f64>,
    pub command: Option<String>,
    pub cooling_fan_speed: Option<String>,
    pub ctt: Option<f64>,
    pub design_id: Option<String>,
    pub device: Option<Device>,
    pub err: Option<String>, // Often a number, but represented as string
    pub fail_reason: Option<String>, // Often a number, but represented as string
    pub fan_gear: Option<i64>,
    pub file: Option<String>,
    pub force_upgrade: Option<bool>,
    pub fun: Option<String>,
    pub gcode_file: Option<String>,
    pub gcode_file_prepare_percent: Option<String>,
    pub gcode_state: Option<String>,
    pub heatbreak_fan_speed: Option<String>,
    pub hms: Option<Vec<HmsEntry>>, // Assuming HmsEntry structure if known, else use serde_json::Value
    pub home_flag: Option<i64>,
    pub hw_switch_state: Option<i64>,
    pub ipcam: Option<Ipcam>,
    pub job: Option<Job>,
    pub job_attr: Option<i64>,
    pub job_id: Option<String>,
    pub layer_num: Option<i64>,
    pub lights_report: Option<Vec<LightReport>>,
    pub maintain: Option<i64>,
    pub mapping: Option<Vec<i64>>,
    pub mc_action: Option<i64>,
    pub mc_err: Option<i64>,
    pub mc_percent: Option<i64>,
    pub mc_print_error_code: Option<String>,
    pub mc_print_stage: Option<String>, // Often a number, but represented as string
    pub mc_print_sub_stage: Option<i64>,
    pub mc_remaining_time: Option<i64>,
    pub mc_stage: Option<i64>,
    pub model_id: Option<String>,
    pub net: Option<Net>,
    pub nozzle_diameter: Option<String>,
    pub nozzle_target_temper: Option<f64>,
    pub nozzle_temper: Option<f64>,
    pub nozzle_type: Option<String>,
    pub online: Option<Online>,
    pub percent: Option<i64>,
    pub plate_cnt: Option<i64>,
    pub plate_id: Option<i64>,
    pub plate_idx: Option<i64>,
    pub prepare_per: Option<i64>,
    pub print_error: Option<i64>,
    pub print_gcode_action: Option<i64>,
    pub print_real_action: Option<i64>,
    pub print_type: Option<String>,
    pub profile_id: Option<String>,
    pub project_id: Option<String>,
    pub queue: Option<i64>,
    pub queue_est: Option<i64>,
    pub queue_number: Option<i64>,
    pub queue_sts: Option<i64>,
    pub queue_total: Option<i64>,
    pub remain_time: Option<i64>,
    pub s_obj: Option<Vec<serde_json::Value>>, // Use specific struct if structure is known
    pub sdcard: Option<bool>,
    pub sequence_id: Option<String>,
    pub spd_lvl: Option<i64>,
    pub spd_mag: Option<i64>,
    pub stat: Option<String>,
    pub state: Option<i64>,
    pub stg: Option<Vec<i64>>,
    pub stg_cur: Option<i64>,
    pub subtask_id: Option<String>,
    pub subtask_name: Option<String>,
    pub task_id: Option<String>,
    pub total_layer_num: Option<i64>,
    pub upgrade_state: Option<UpgradeState>,
    pub upload: Option<Upload>,
    pub ver: Option<String>, // Often a number, but represented as string
    pub vir_slot: Option<Vec<VirtualTray>>,
    pub vt_tray: Option<VirtualTray>,
    pub wifi_signal: Option<String>,
    pub xcam: Option<Xcam>,
    pub xcam_status: Option<String>, // Often a number, but represented as string
}

#[derive(Debug, Clone, Deserialize)]
pub struct Print2D {
    pub bs: Option<Bs>,
    pub cond: Option<i64>,
    pub cur_stage: Option<CurStage2D>,
    pub first_confirm: Option<bool>,
    pub makeable: Option<bool>,
    pub material: Option<Material>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Bs {
    pub bi: Option<Vec<Bi>>,
    pub total_time: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Bi {
    pub est_time: Option<i64>,
    pub idx: Option<i64>,
    pub print_then: Option<bool>,
    pub proc_list: Option<Vec<serde_json::Value>>, // Use specific struct if structure is known
    pub step_type: Option<i64>,
    pub tool_info: Option<ToolInfo>,
    #[serde(rename = "type")]
    pub type_field: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolInfo {
    pub color: Option<String>,
    pub diameter: Option<f64>,
    pub id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CurStage2D {
    pub idx: Option<i64>,
    pub left_time: Option<i64>,
    pub process: Option<i64>,
    pub state: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Material {
    pub cur_id_list: Option<Vec<serde_json::Value>>, // Use specific struct if structure is known
    pub state: Option<i64>,
    pub tar_id: Option<String>,
    pub tar_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Print3D {
    pub layer_num: Option<i64>,
    pub total_layer_num: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Ams {
    pub ams: Vec<AmsUnit>,
    pub ams_exist_bits: Option<String>,
    pub ams_exist_bits_raw: Option<String>,
    pub cali_id: Option<i64>,
    pub cali_stat: Option<i64>,
    pub insert_flag: Option<bool>,
    pub power_on_flag: Option<bool>,
    pub tray_exist_bits: Option<String>,
    pub tray_is_bbl_bits: Option<String>,
    pub tray_now: Option<String>,
    pub tray_pre: Option<String>,
    pub tray_read_done_bits: Option<String>,
    pub tray_reading_bits: Option<String>,
    pub tray_tar: Option<String>,
    pub unbind_ams_stat: Option<i64>,
    pub version: Option<i64>,
}

impl Ams {
    pub fn is_ams_unload(&self) -> bool {
        self.tray_tar.as_ref().map(|s| s.as_str()) == Some("255")
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AmsUnit {
    pub dry_time: Option<i64>,
    pub humidity: Option<String>,
    pub humidity_raw: Option<String>,
    pub id: Option<String>,
    pub info: Option<String>,
    pub temp: Option<String>, // Often a float, but represented as string
    pub tray: Option<Vec<AmsTray>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AmsTray {
    pub id: Option<String>,
    pub state: Option<i64>,
    // These fields appear in the standard PrintData.ams.ams.tray but not in the H2D example
    pub k: Option<f64>,
    pub n: Option<String>,
    pub tray_color: Option<String>,
    pub tray_diameter: Option<String>,
    pub tray_id_name: Option<String>,
    pub tray_info_idx: Option<String>,
    pub tray_sub_brands: Option<String>,
    pub tray_type: Option<String>,
    pub tray_uuid: Option<String>,
    pub tray_weight: Option<String>,
    pub xcam_info: Option<String>,
    pub cols: Option<Vec<String>>,
    pub ctype: Option<i64>,
    pub nozzle_temp_max: Option<String>,
    pub nozzle_temp_min: Option<String>,
    pub remain: Option<i64>,
    pub tag_uid: Option<String>,
    pub bed_temp: Option<String>,
    pub bed_temp_type: Option<String>,
    pub drying_temp: Option<String>,
    pub drying_time: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Device {
    pub airduct: Option<Airduct>,
    pub bed_temp: Option<i64>,
    pub cam: Option<Cam>,
    pub cham_temp: Option<i64>,
    pub ext_tool: Option<ExtTool>,
    pub extruder: Option<Extruder>,
    pub fan: Option<i64>,
    pub laser: Option<LaserPower>,
    pub nozzle: Option<Nozzle>,
    pub plate: Option<Plate>,
    #[serde(rename = "type")]
    pub type_field: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Airduct {
    #[serde(rename = "modeCur")]
    pub mode_cur: Option<i64>,
    #[serde(rename = "modeList")]
    pub mode_list: Option<Vec<AirductMode>>,
    pub parts: Option<Vec<AirductPart>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AirductMode {
    pub ctrl: Option<Vec<i64>>,
    #[serde(rename = "modeId")]
    pub mode_id: Option<i64>,
    pub off: Option<Vec<i64>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AirductPart {
    pub func: Option<i64>,
    pub id: Option<i64>,
    pub range: Option<i64>,
    pub state: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Cam {
    pub laser: Option<LaserStatus>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LaserStatus {
    pub cond: Option<i64>,
    pub state: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExtTool {
    pub calib: Option<i64>,
    pub mount: Option<i64>,
    #[serde(rename = "type")]
    pub type_field: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Extruder {
    pub info: Option<Vec<ExtruderInfo>>,
    pub state: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExtruderInfo {
    pub filam_bak: Option<Vec<serde_json::Value>>, // Use specific struct if structure is known
    pub hnow: Option<i64>,
    pub hpre: Option<i64>,
    pub htar: Option<i64>,
    pub id: Option<i64>,
    pub info: Option<i64>,
    pub snow: Option<i64>,
    pub spre: Option<i64>,
    pub star: Option<i64>,
    pub stat: Option<i64>,
    pub temp: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LaserPower {
    pub power: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Nozzle {
    pub exist: Option<i64>,
    pub info: Option<Vec<NozzleInfo>>,
    pub state: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NozzleInfo {
    pub diameter: Option<f64>,
    pub id: Option<i64>,
    pub tm: Option<i64>,
    #[serde(rename = "type")]
    pub type_field: Option<String>,
    pub wear: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Plate {
    pub base: Option<i64>,
    pub cali2d_id: Option<String>,
    pub cur_id: Option<String>,
    pub mat: Option<i64>,
    pub tar_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HmsEntry {
    // Define fields based on actual HMS data structure
    // Example:
    // pub attr: Option<i64>,
    // pub code: Option<i64>,
    // ...
    // Using Value as a placeholder if structure is unknown/variable
    #[serde(flatten)]
    pub data: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Ipcam {
    pub agora_service: Option<String>,
    pub brtc_service: Option<String>,
    pub bs_state: Option<i64>,
    pub ipcam_dev: Option<String>,
    pub ipcam_record: Option<String>,
    pub laser_preview_res: Option<i64>,
    pub mode_bits: Option<i64>,
    pub resolution: Option<String>,
    pub rtsp_url: Option<String>,
    pub timelapse: Option<String>,
    pub tl_store_hpd_type: Option<i64>,
    pub tl_store_path_type: Option<i64>,
    pub tutk_server: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Job {
    pub cur_stage: Option<JobCurStage>,
    pub stage: Option<Vec<JobStage>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JobCurStage {
    pub idx: Option<i64>,
    pub state: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JobStage {
    pub color: Option<Vec<String>>,
    pub diameter: Option<Vec<f64>>,
    pub est_time: Option<i64>,
    pub heigh: Option<f64>,
    pub idx: Option<i64>,
    pub platform: Option<String>,
    pub print_then: Option<bool>,
    pub proc_list: Option<Vec<serde_json::Value>>, // Use specific struct if structure is known
    pub tool: Option<Vec<String>>,
    #[serde(rename = "type")]
    pub type_field: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LightReport {
    pub mode: Option<String>,
    pub node: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Net {
    pub conf: Option<i64>,
    pub info: Option<Vec<NetInfo>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NetInfo {
    pub ip: Option<i64>,   // IP address as integer
    pub mask: Option<i64>, // Subnet mask as integer
}

#[derive(Debug, Clone, Deserialize)]
pub struct Online {
    pub ahb: Option<bool>,
    pub ext: Option<bool>,
    pub version: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpgradeState {
    pub ahb_new_version_number: Option<String>,
    pub ams_new_version_number: Option<String>,
    pub consistency_request: Option<bool>,
    pub dis_state: Option<i64>,
    pub err_code: Option<i64>,
    pub ext_new_version_number: Option<String>,
    pub force_upgrade: Option<bool>,
    pub idx: Option<i64>,
    pub idx2: Option<i64>,
    pub lower_limit: Option<String>,
    pub message: Option<String>,
    pub module: Option<String>,
    pub new_version_state: Option<i64>,
    pub ota_new_version_number: Option<String>,
    pub progress: Option<String>,
    pub sequence_id: Option<i64>,
    pub sn: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Upload {
    pub file_size: Option<i64>,
    pub finish_size: Option<i64>,
    pub message: Option<String>,
    pub oss_url: Option<String>,
    pub progress: Option<i64>,
    pub sequence_id: Option<String>,
    pub speed: Option<i64>,
    pub status: Option<String>,
    pub task_id: Option<String>,
    pub time_remaining: Option<i64>,
    pub trouble_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VirtualTray {
    pub bed_temp: Option<String>,
    pub bed_temp_type: Option<String>,
    pub cali_idx: Option<i64>,
    pub cols: Option<Vec<String>>,
    pub ctype: Option<i64>,
    pub drying_temp: Option<String>,
    pub drying_time: Option<String>,
    pub id: Option<String>,
    pub nozzle_temp_max: Option<String>,
    pub nozzle_temp_min: Option<String>,
    pub remain: Option<i64>,
    pub tag_uid: Option<String>,
    pub total_len: Option<i64>,
    pub tray_color: Option<String>,
    pub tray_diameter: Option<String>,
    pub tray_id_name: Option<String>,
    pub tray_info_idx: Option<String>,
    pub tray_sub_brands: Option<String>,
    pub tray_type: Option<String>,
    pub tray_uuid: Option<String>,
    pub tray_weight: Option<String>,
    pub xcam_info: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Xcam {
    pub allow_skip_parts: Option<bool>,
    pub buildplate_marker_detector: Option<bool>,
    pub first_layer_inspector: Option<bool>,
    pub halt_print_sensitivity: Option<String>,
    pub print_halt: Option<bool>,
    pub printing_monitor: Option<bool>,
    pub spaghetti_detector: Option<bool>,
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
