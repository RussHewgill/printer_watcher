use serde::{Deserialize, Serialize};

/// MARK: Version
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Version {
    pub api: String,
    pub server: String,
    pub nozzle_diameter: f64,
    pub text: String,
    pub hostname: String,
    pub capabilities: Capabilities,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Capabilities {
    #[serde(rename = "upload-by-put")]
    pub upload_by_put: bool,
}

/// MARK: Info
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Info {
    pub nozzle_diameter: f64,
    pub mmu: bool,
    pub serial: String,
    pub hostname: String,
    pub min_extrusion_temp: i64,
}

/// MARK: Status
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Status {
    pub job: JobStatus,
    pub storage: Storage,
    pub printer: Printer,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobStatus {
    pub id: i64,
    pub progress: f64,
    pub time_remaining: i64,
    pub time_printing: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Storage {
    pub path: String,
    pub name: String,
    pub read_only: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Printer {
    pub state: String,
    pub temp_bed: f64,
    pub target_bed: f64,
    pub temp_nozzle: f64,
    pub target_nozzle: f64,
    pub axis_z: f64,
    pub flow: i64,
    pub speed: i64,
    pub fan_hotend: i64,
    pub fan_print: i64,
}

/// MARK: Job
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Job {
    pub file: File,
    pub id: i64,
    pub progress: f64,
    pub state: String,
    pub time_printing: i64,
    pub time_remaining: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct File {
    pub display_name: String,
    pub m_timestamp: i64,
    pub name: String,
    pub path: String,
    pub refs: Refs,
    pub size: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Refs {
    pub download: String,
    pub icon: String,
    pub thumbnail: String,
}
