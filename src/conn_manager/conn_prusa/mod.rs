// pub mod prusa_cloud;
// pub mod prusa_cloud_types;
pub mod prusa_local;
pub mod prusa_local_types;
pub mod telemetry;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use std::sync::Arc;

use reqwest::RequestBuilder;
use tokio::sync::RwLock;

use crate::config::printer_config::PrinterConfigPrusa;
