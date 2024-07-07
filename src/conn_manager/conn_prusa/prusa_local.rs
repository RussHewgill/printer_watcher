use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use std::sync::Arc;

use reqwest::RequestBuilder;
use serde::de::DeserializeOwned;
use tokio::sync::RwLock;

use crate::{
    config::{printer_config::PrinterConfigPrusa, printer_id::PrinterId},
    conn_manager::{
        conn_prusa::prusa_local_types::PrusaStatus, worker_message::WorkerMsg, WorkerCmd,
    },
    status::{GenericPrinterStateUpdate, PrinterState, PrinterStateUpdate},
};

pub struct PrusaClientLocal {
    pub(super) printer_cfg: Arc<RwLock<PrinterConfigPrusa>>,
    client: reqwest::Client,
    tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, WorkerMsg)>,
    cmd_rx: tokio::sync::mpsc::UnboundedReceiver<WorkerCmd>,
    kill_rx: tokio::sync::oneshot::Receiver<()>,
    update_timer: tokio::time::Interval,
    // thumbnail: Option<(String, Vec<u8>)>,
    octo_client: Option<crate::conn_manager::conn_octoprint::OctoClientLocal>,
}

/// new, run
impl PrusaClientLocal {
    const URL_VERSION: &'static str = "api/version";
    const URL_INFO: &'static str = "api/v1/info";
    const URL_STATUS: &'static str = "api/v1/status";
    const URL_JOB: &'static str = "api/v1/job";

    pub async fn new(
        printer_cfg: Arc<RwLock<PrinterConfigPrusa>>,
        tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, WorkerMsg)>,
        cmd_rx: tokio::sync::mpsc::UnboundedReceiver<WorkerCmd>,
        kill_rx: tokio::sync::oneshot::Receiver<()>,
        interval: Option<std::time::Duration>,
    ) -> Result<Self> {
        let client = reqwest::ClientBuilder::new()
            // .use_rustls_tls()
            // .danger_accept_invalid_certs(true)
            .build()?;

        let update_timer = if let Some(interval) = interval {
            tokio::time::interval(interval)
        } else {
            tokio::time::interval(std::time::Duration::from_secs(1))
        };

        let octo_client = if let Some(octo_cfg) = printer_cfg.read().await.octo.clone() {
            let octo_client = crate::conn_manager::conn_octoprint::OctoClientLocal::new(
                octo_cfg,
                // tx.clone(),
                // cmd_rx.clone(),
                // kill_rx.clone(),
                // None,
            )?;
            Some(octo_client)
        } else {
            None
        };

        Ok(Self {
            printer_cfg,
            client,
            tx,
            cmd_rx,
            kill_rx,
            update_timer,
            // thumbnail: None,
            octo_client,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        loop {
            tokio::select! {
                _ = self.update_timer.tick() => {
                    self.update().await?;
                }
                _ = &mut self.kill_rx => {
                    info!("kill_rx fired, exiting");
                    return Ok(());
                }
                Some(cmd) = self.cmd_rx.recv() => {
                    warn!("unhandled cmd = {:#?}", cmd);
                }
            }
        }
    }

    async fn update(&mut self) -> Result<()> {
        let id = self.printer_cfg.read().await.id.clone();

        let (update, status, job) = self.get_update().await?;
        // debug!("sending update: {:#?}", &update);
        self.tx
            .send((id.clone(), WorkerMsg::StatusUpdate(update)))?;

        self.tx.send((
            id.clone(),
            WorkerMsg::StatusUpdatePrusa(PrusaStatus { status, job }),
        ))?;

        if let Some(octo) = &self.octo_client {
            let update = octo.get_update().await?;
            self.tx
                .send((id.clone(), WorkerMsg::StatusUpdate(update)))?;
        }

        Ok(())
    }
}

/// set_headers
impl PrusaClientLocal {
    async fn set_headers(&self, req: RequestBuilder) -> Result<RequestBuilder> {
        let printer = self.printer_cfg.read().await;

        let timestamp = chrono::Utc::now().timestamp();

        let req = req
            // .header("timestamp", &format!("{}", timestamp))
            // .header("Token", &printer.token)
            .header("X-Api-Key", &printer.key)
            // .header("User-Agent", "printer_watcher")
            // .header("User-Agent-Printer", "")
            // .header("User-Agent-Version", "")
            ;

        Ok(req)
    }
}

/// get_update
impl PrusaClientLocal {
    pub async fn get_update(
        &self,
    ) -> Result<(
        GenericPrinterStateUpdate,
        super::prusa_local_types::Status,
        super::prusa_local_types::Job,
    )> {
        let mut out = vec![];

        let status = self.get_status().await?;

        let state = match status.printer.state.as_ref() {
            "PRINTING" => PrinterState::Printing,
            "BUSY" => PrinterState::Busy,
            "PAUSED" => PrinterState::Paused,
            "ERROR" => PrinterState::Error,
            "ATTENTION" => PrinterState::Error,
            "IDLE" => PrinterState::Idle,
            "FINISHED" => PrinterState::Idle,
            "STOPPED" => PrinterState::Idle,
            "READY" => PrinterState::Idle,
            _ => PrinterState::Disconnected,
        };
        out.push(crate::status::PrinterStateUpdate::State(state.clone()));

        let job = self.get_job().await?;

        // let thumbnail = job.file.refs.thumbnail.clone();
        // debug!("thumbnail = {:#?}", thumbnail);

        let time_printing = match state {
            PrinterState::Printing | PrinterState::Error | PrinterState::Paused => {
                Some(Some(chrono::Duration::seconds(job.time_printing)))
            }
            _ => None,
        };
        let time_remaining = match state {
            PrinterState::Printing | PrinterState::Error | PrinterState::Paused => {
                Some(Some(chrono::Duration::seconds(job.time_remaining)))
            }
            _ => None,
        };

        out.push(PrinterStateUpdate::Progress(status.job.progress as f32));

        out.push(PrinterStateUpdate::NozzleTemp(
            None,
            status.printer.temp_nozzle as f32,
            Some(status.printer.target_nozzle as f32),
        ));
        out.push(PrinterStateUpdate::BedTemp(
            status.printer.temp_bed as f32,
            Some(status.printer.target_bed as f32),
        ));

        out.push(PrinterStateUpdate::CurrentFile(
            job.file.display_name.clone(),
        ));

        Ok((GenericPrinterStateUpdate(out), status, job))
        // Ok(GenericPrinterStateUpdate {
        //     state: Some(state),
        //     nozzle_temp: Some(status.printer.temp_nozzle as f32),
        //     bed_temp: Some(status.printer.temp_bed as f32),
        //     nozzle_temp_target: Some(status.printer.target_nozzle as f32),
        //     bed_temp_target: Some(status.printer.target_bed as f32),
        //     progress: Some(status.job.progress as f32),
        //     time_printing,
        //     time_remaining,
        //     current_file: Some(job.file.display_name),
        //     ..Default::default()
        // })
    }

    pub async fn download_thumbnail(&self) -> Result<Vec<u8>> {
        let job = self.get_job().await?;
        let thumbnail_url = job.file.refs.thumbnail.clone();
        let thumbnail_url = format!(
            "http://{}{}",
            self.printer_cfg.read().await.host,
            thumbnail_url
        );
        let resp = self
            .set_headers(self.client.get(&thumbnail_url))
            .await?
            .send()
            .await?;

        let bytes = resp.bytes().await?;
        Ok(bytes.to_vec())
    }
}

/// getters
impl PrusaClientLocal {
    pub async fn get_response<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        let printer = self.printer_cfg.read().await;

        let url = format!("http://{}:{}/{}", printer.host, 80, url);
        let req = self.client.get(&url);

        let req = self.set_headers(req).await?;
        let resp = req.send().await?;

        if !resp.status().is_success() {
            debug!("status {:#?}", resp.status());
            bail!("Failed to get response, url = {}", url);
        }

        // let j: serde_json::Value = resp.json().await?;
        // debug!("json = {:#?}", j);

        Ok(resp.json().await?)
    }

    pub async fn get_job(&self) -> Result<super::prusa_local_types::Job> {
        self.get_response(Self::URL_JOB).await
    }

    pub async fn get_info(&self) -> Result<super::prusa_local_types::Info> {
        self.get_response(Self::URL_INFO).await
    }

    pub async fn get_status(&self) -> Result<super::prusa_local_types::Status> {
        self.get_response(Self::URL_STATUS).await
    }
}
