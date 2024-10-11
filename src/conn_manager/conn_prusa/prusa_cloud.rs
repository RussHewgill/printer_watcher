use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use std::sync::Arc;

use reqwest::RequestBuilder;
use tokio::sync::RwLock;

use crate::config::printer_config::PrinterConfigPrusa;

pub struct PrusaClient {
    pub(super) printer_cfg: Arc<RwLock<PrinterConfigPrusa>>,
    client: reqwest::Client,
    code: Option<String>,
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
            code: None,
        })
    }
}

impl PrusaClient {
    // #[cfg(feature = "nope")]
    pub async fn register(&mut self) -> Result<()> {
        let url = format!("https://{}/p/register", self.printer_cfg.read().await.host);

        debug!("url = {:#?}", url);

        let req = self.client.post(&url);
        let req = self.set_headers(req).await?;
        let req = req.json(&serde_json::json!({
            "sn": self.printer_cfg.read().await.serial,
            "fingerprint": self.printer_cfg.read().await.fingerprint,
            // "printer_type": "XL5IS", // type = 3.1.0 ??
            "printer_type": "3.1.0", // type = 3.1.0 ??
            "firmware": "6.0.4+14924",
        }));

        debug!("sending request");
        let response = req.send().await?;
        // debug!("response = {:#?}", response);
        debug!("status = {:?}", response.status());

        let headers = response.headers();
        // debug!("headers = {:#?}", headers);

        if let Some(code) = headers.get("code") {
            debug!("got code: {:?}", code);
            let code = code.to_str()?.to_string();
            self.code = Some(code);
        }

        // let text = response.text().await?;
        // debug!("text = {:#?}", text);

        // let json: serde_json::Value = response.json().await?;
        // debug!("json = {:#?}", json);

        Ok(())
        // unimplemented!()
    }

    async fn set_headers(&self, req: RequestBuilder) -> Result<RequestBuilder> {
        let printer = self.printer_cfg.read().await;

        let timestamp = chrono::Utc::now().timestamp();
        let timestamp = format!("{}", timestamp);
        // debug!("timestamp = {}", timestamp);

        let req = req
            .header("Fingerprint", &self.printer_cfg.read().await.fingerprint)
            .header("timestamp", &timestamp)
            // .header("X-Api-Key", &printer.key)
            .header("User-Agent", "printer_watcher")
            .header("Token", &printer.token)
            .header("User-Agent-Printer", "XL5IS")
            // .header("User-Agent-Version", "0.0.1")
            ;

        // let req = if let Some(code) = self.code.as_ref() {
        //     req.header("Token", code)
        // } else {
        //     req
        // };

        Ok(req)
    }
}

impl PrusaClient {
    pub async fn get_telemetry(&self) -> Result<()> {
        let timestamp = chrono::Utc::now().timestamp() - 60;
        let timestamp = format!("{}", timestamp);

        let url = format!(
            "https://{}/app/printers/{}/telemetry?from={}&granularity=15",
            self.printer_cfg.read().await.host,
            self.printer_cfg.read().await.fingerprint,
            timestamp,
        );

        debug!("url = {:#?}", url);

        let req = self.client.get(&url);

        let req = self.set_headers(req).await?;

        // let cookie = std::env::var("PRUSA_CONNECT_TEST_COOKIE")?;
        // let req = req.header("Cookie", cookie);

        // let req = req.header("")

        // let req = req.json("");

        debug!("sending request");
        let response = req.send().await?;

        debug!("response = {:#?}", response);
        debug!("status = {:?}", response.status());

        let text: serde_json::Value = response.json().await?;
        debug!("text = {:#?}", text);

        Ok(())
    }

    pub async fn get_info(&self) -> Result<()> {
        // let url = format!(
        //     "https://{}/{}",
        //     self.printer_cfg.read().await.host,
        //     Self::URL_VERSION
        // );

        // let payload = serde_json::json!({
        //     "command": "SEND_INFO",
        // });

        // if !response.status().is_success() {
        //     bail!("failed to get info: {:?}", response);
        // }

        // let text = response.json().await?;
        // debug!("text = {:#?}", text);

        unimplemented!()
    }
}
