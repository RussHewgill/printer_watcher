use serde::{Deserialize, Serialize};

pub mod printer_status {
    use anyhow::{anyhow, bail, ensure, Context, Result};
    use tracing::{debug, error, info, trace, warn};

    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    #[derive(Debug, Default, Clone, Deserialize)]
    pub struct PrinterStatus {
        #[serde(deserialize_with = "deserialize_temperature")]
        pub temperature: Temperature,
        pub sd: Sd,
        pub state: State,
    }

    #[derive(Default, Debug, Clone, PartialEq, Deserialize)]
    pub struct Temperature {
        pub tools: Vec<(usize, Tool)>,
        pub bed: Tool,
        #[serde(default)]
        pub history: Vec<History>,
    }

    fn deserialize_temperature<'de, D>(deserializer: D) -> Result<Temperature, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut out = Temperature::default();

        let map: std::collections::HashMap<String, Tool> =
            std::collections::HashMap::deserialize(deserializer)?;

        if let Some(bed) = map.get("bed").cloned() {
            out.bed = bed;
        }

        let mut tools: Vec<(usize, Tool)> = vec![];

        for (key, value) in map {
            if key.starts_with("tool") {
                // debug!("key: {}, value: {:#?}", key, value);
                let id: usize = key.trim_start_matches("tool").parse().unwrap();
                tools.push((id, value));
            }
        }
        tools.sort_by_key(|tool| tool.0);

        out.tools = tools;

        Ok(out)
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Tool {
        pub actual: f32,
        pub target: Option<f32>,
        pub offset: i64,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct History {
        pub time: i64,
        // pub tool0: Tool02,
        // pub tool1: Tool12,
        // pub bed: Bed2,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Sd {
        pub ready: bool,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct State {
        pub text: String,
        pub flags: Flags,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Flags {
        pub operational: bool,
        pub paused: bool,
        pub printing: bool,
        pub cancelling: bool,
        pub pausing: bool,
        #[serde(rename = "sdReady")]
        pub sd_ready: bool,
        pub error: bool,
        pub ready: bool,
        #[serde(rename = "closedOrError")]
        pub closed_or_error: bool,
    }
}

pub mod job {
    use serde::{Deserialize, Serialize};

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct JobResponse {
        pub job: Job,
        pub progress: Progress,
        pub state: String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Job {
        pub file: File,
        #[serde(rename = "estimatedPrintTime")]
        pub estimated_print_time: i64,
        pub filament: Filament,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct File {
        pub name: String,
        pub origin: String,
        pub size: i64,
        pub date: i64,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Filament {
        pub tool0: Tool0,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Tool0 {
        pub length: i64,
        pub volume: f32,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Progress {
        pub completion: f32,
        pub filepos: i64,
        #[serde(rename = "printTime")]
        pub print_time: i64,
        #[serde(rename = "printTimeLeft")]
        pub print_time_left: i64,
    }
}
