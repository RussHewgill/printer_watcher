use serde::{Deserialize, Serialize};
use serde_json::Value;

// #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
// pub struct Rpc {
//     pub jsonrpc: String,
//     pub method: String,
//     pub params: Vec<serde_json::Value>,
// }

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusUpdateResponse {
    pub eventtime: f64,
    pub status: Status,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Status {
    pub extruder: Extruder,
    pub heater_bed: HeaterBed,
    pub print_stats: PrintStats,
    pub virtual_sdcard: VirtualSdcard,
    pub webhooks: Webhooks,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Extruder {
    pub target: f64,
    pub temperature: f64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeaterBed {
    pub target: f64,
    pub temperature: f64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrintStats {
    pub filament_used: f64,
    pub filename: String,
    pub info: Info,
    pub message: String,
    pub print_duration: f64,
    pub state: String,
    pub total_duration: f64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Info {
    pub current_layer: Option<i64>,
    pub total_layer: Option<i64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VirtualSdcard {
    pub file_path: Option<String>,
    pub file_position: i64,
    pub file_size: i64,
    pub is_active: bool,
    pub progress: f64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Webhooks {
    pub state: String,
    pub state_message: String,
}

pub mod metadata {
    use serde::Deserialize;
    use serde::Serialize;

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct KlipperMetadata {
        pub chamber_temp: f64,
        pub estimated_time: i64,
        pub filament_name: String,
        pub filament_total: Option<f64>,
        pub filament_type: String,
        pub filament_weight_total: f64,
        pub filename: String,
        pub first_layer_bed_temp: f64,
        pub first_layer_extr_temp: f64,
        pub first_layer_height: f64,
        pub gcode_end_byte: i64,
        pub gcode_start_byte: i64,
        pub job_id: String,
        pub layer_count: i64,
        pub layer_height: f64,
        pub modified: f64,
        pub nozzle_diameter: f64,
        pub object_height: f64,
        pub print_start_time: f64,
        pub size: i64,
        pub slicer: String,
        pub slicer_version: String,
        pub thumbnails: Vec<Thumbnail>,
        pub uuid: String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Thumbnail {
        pub height: i64,
        pub relative_path: String,
        pub size: i64,
        pub width: i64,
    }
}
