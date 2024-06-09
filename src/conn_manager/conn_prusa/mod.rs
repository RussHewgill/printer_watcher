use std::sync::Arc;

use tokio::sync::RwLock;

use crate::config::printer_config::PrinterConfigPrusa;

pub struct PrusaClient {
    pub(super) printer_cfg: Arc<RwLock<PrinterConfigPrusa>>,
    client: reqwest::Client,
}

impl PrusaClient {
    // /api/v1/info:
    // /api/v1/status:
    // /api/v1/job:
    // /api/v1/job/{id}:
    // /api/v1/job/{id}/pause:
    // /api/v1/job/{id}/resume:

    const URL_VERSION: &'static str = "/api/version";
    const URL_INFO: &'static str = "/api/v1/info";
    const URL_STATUS: &'static str = "/api/v1/status";
    const URL_JOB: &'static str = "/api/v1/job";

    pub fn new(printer_cfg: Arc<RwLock<PrinterConfigPrusa>>) -> Self {
        let client = reqwest::Client::new();
        Self {
            printer_cfg,
            client,
        }
    }
}
