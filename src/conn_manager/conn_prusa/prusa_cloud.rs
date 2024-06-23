use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use std::sync::Arc;

use reqwest::RequestBuilder;
use tokio::sync::RwLock;

use crate::config::printer_config::PrinterConfigPrusa;

pub struct PrusaClient {
    pub(super) printer_cfg: Arc<RwLock<PrinterConfigPrusa>>,
    client: reqwest::Client,
}

/// consts, new
impl PrusaClient {
    // /api/v1/info:
    // /api/v1/status:
    // /api/v1/job:
    // /api/v1/job/{id}:
    // /api/v1/job/{id}/pause:
    // /api/v1/job/{id}/resume:

    const URL_VERSION: &'static str = "api/version";
    const URL_INFO: &'static str = "api/v1/info";
    const URL_STATUS: &'static str = "api/v1/status";
    const URL_JOB: &'static str = "api/v1/job";

    pub fn new(printer_cfg: Arc<RwLock<PrinterConfigPrusa>>) -> Result<Self> {
        // let mut root_cert_store = rustls::RootCertStore::empty();
        // root_cert_store.add_parsable_certificates(
        //     rustls_native_certs::load_native_certs().expect("could not load platform certs"),
        // );

        // let client_config = rustls::ClientConfig::builder()
        //     .with_root_certificates(root_cert_store)
        //     .with_no_client_auth();

        let client = reqwest::ClientBuilder::new()
            .use_rustls_tls()
            // .with_root_certificates(client_config.root_store)
            // .tls_built_in_native_certs(true)
            // .tls_built_in_root_certs(true)
            .danger_accept_invalid_certs(true)
            .build()?;

        // let client = reqwest::ClientBuilder::new().build()?;
        Ok(Self {
            printer_cfg,
            client,
        })
    }
}

impl PrusaClient {
    async fn set_headers(&self, req: RequestBuilder) -> Result<RequestBuilder> {
        let printer = self.printer_cfg.read().await;

        let timestamp = chrono::Utc::now().timestamp();

        let req = req
            // .header("timestamp", &format!("{}", timestamp))
            // .header("Token", &printer.token)
            // .header("X-Api-Key", &printer.key)
            // .header("User-Agent", "printer_watcher")
            // .header("User-Agent-Printer", "")
            // .header("User-Agent-Version", "")
            ;

        Ok(req)
    }
}

impl PrusaClient {
    pub async fn get_info(&self) -> Result<()> {
        let url = format!(
            "https://{}/{}",
            self.printer_cfg.read().await.host,
            Self::URL_VERSION
        );

        let req = self.client.get(&url);

        let req = self.set_headers(req).await?;

        let response = req.send().await?;

        if !response.status().is_success() {
            bail!("failed to get info: {:?}", response);
        }

        let text = response.text().await?;

        debug!("text = {:#?}", text);

        unimplemented!()
    }
}
