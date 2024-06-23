use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use std::sync::Arc;

use reqwest::RequestBuilder;
use tokio::sync::RwLock;

use crate::config::printer_config::PrinterConfigPrusa;

pub struct PrusaClientLocal {
    pub(super) printer_cfg: Arc<RwLock<PrinterConfigPrusa>>,
    client: reqwest::Client,
}

impl PrusaClientLocal {
    const URL_VERSION: &'static str = "api/version";
    const URL_INFO: &'static str = "api/v1/info";
    const URL_STATUS: &'static str = "api/v1/status";
    const URL_JOB: &'static str = "api/v1/job";

    pub fn new(printer_cfg: Arc<RwLock<PrinterConfigPrusa>>) -> Result<Self> {
        let client = reqwest::ClientBuilder::new()
            // .use_rustls_tls()
            // .danger_accept_invalid_certs(true)
            .build()?;

        Ok(Self {
            printer_cfg,
            client,
        })
    }
}

impl PrusaClientLocal {
    pub async fn get_info(&self) -> Result<()> {
        let printer = self.printer_cfg.read().await;

        let url = format!("http://{}:{}/{}", printer.host, 80, Self::URL_INFO);
        debug!("url: {}", url);
        let req = self.client.get(&url);

        // let req = self.set_headers(req).await?;
        let resp = req.send().await?;

        let status = resp.status();
        if !status.is_success() {
            bail!("Request failed: {}", status);
        }

        let body = resp.text().await?;
        info!("Response: {}", body);

        Ok(())
    }
}
