pub mod octo_commands;
pub mod octo_types;

use anyhow::{anyhow, bail, ensure, Context, Result};
use octo_commands::OctoCmd;
use tracing::{debug, error, info, trace, warn};

use serde::de::DeserializeOwned;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{
    config::{printer_config::PrinterConfigOcto, printer_id::PrinterId},
    status::{GenericPrinterStateUpdate, PrinterState, PrinterStateUpdate},
};

use super::{worker_message::WorkerMsg, WorkerCmd};

pub struct OctoClientLocal {
    pub(super) printer_cfg: Arc<RwLock<PrinterConfigOcto>>,
    client: reqwest::Client,
    tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, WorkerMsg)>,
    cmd_rx: tokio::sync::mpsc::UnboundedReceiver<WorkerCmd>,
    kill_rx: tokio::sync::oneshot::Receiver<()>,
    update_timer: tokio::time::Interval,
}

/// new
impl OctoClientLocal {
    pub fn new(
        printer_cfg: Arc<RwLock<PrinterConfigOcto>>,
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

        Ok(Self {
            printer_cfg,
            client,
            tx,
            cmd_rx,
            kill_rx,
            update_timer,
        })
    }
}

/// get_response, get_update
impl OctoClientLocal {
    pub async fn get_response<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        let printer = self.printer_cfg.read().await;
        let token = printer.token.clone();
        let host = printer.host.clone();
        drop(printer);

        let url = format!("http://{}:5000/{}", host, url);

        let res = self
            .client
            .get(&url)
            .header("X-Api-Key", token)
            .send()
            .await?;

        if !res.status().is_success() {
            warn!("status {:#?}", res.status());
            bail!("Failed to get response, url = {}", url);
        }

        Ok(res.json().await?)
    }

    pub async fn get_update(&self) -> Result<GenericPrinterStateUpdate> {
        let mut out = vec![];

        let job_info = self.get_job_info().await?;

        let printer_state = self.get_printer_state().await?;

        out.push(PrinterStateUpdate::BedTemp(
            printer_state.temperature.bed.actual,
            printer_state.temperature.bed.target,
        ));

        for (id, tool) in printer_state.temperature.tools {
            out.push(PrinterStateUpdate::NozzleTemp(id, tool.actual, tool.target));
        }

        let flags = printer_state.state.flags;
        let state = if flags.printing {
            PrinterState::Printing
        } else if flags.paused {
            PrinterState::Paused
        } else if flags.error {
            PrinterState::Error
        } else {
            PrinterState::Idle
        };
        out.push(PrinterStateUpdate::State(state));

        Ok(GenericPrinterStateUpdate(out))

        // Ok(GenericPrinterStateUpdate {
        //     // state: Some(state),
        //     // nozzle_temp: Some(status.printer.temp_nozzle as f32),
        //     // bed_temp: Some(status.printer.temp_bed as f32),
        //     // nozzle_temp_target: Some(status.printer.target_nozzle as f32),
        //     // bed_temp_target: Some(status.printer.target_bed as f32),
        //     // progress: Some(status.job.progress as f32),
        //     // time_printing,
        //     // time_remaining,
        //     // current_file: Some(job.file.display_name),
        //     ..Default::default()
        // })
    }
}

/// send commands
impl OctoClientLocal {
    // pub async fn send_command<T: DeserializeOwned>(&self, cmd: &OctoCmd) -> Result<T>
    pub async fn send_command(&self, cmd: &OctoCmd) -> Result<reqwest::Response> {
        let printer = self.printer_cfg.read().await;
        let token = printer.token.clone();
        let host = printer.host.clone();
        drop(printer);

        let url = match cmd {
            // OctoCmd::ParkTool => todo!(),
            OctoCmd::Jog { .. } | OctoCmd::Home { .. } | OctoCmd::SetFeedrate(..) => {
                "api/printer/printhead"
            }
            OctoCmd::PickupTool(_) 
            | OctoCmd::ParkTool 
            | OctoCmd::ChangeFilament(_) 
            // | OctoCmd::ChangeFilament(_) 
            => "api/printer/command",
            _ => unimplemented!(),
        };

        let url = format!("http://{}:5000/{}", host, url);

        let cmd = cmd.to_json();

        debug!("sending command: {:#?}", cmd);

        let res = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-Api-Key", token)
            .json(&cmd)
            .send()
            .await?;

        if !res.status().is_success() {
            warn!("status {:#?}", res.status());
            bail!("Failed to get response, url = {}", url);
        }

        // let json: serde_json::Value = res.json().await?;

        // Ok(res.json().await?)
        // Ok(())
        Ok(res)
    }
}

/// get info
impl OctoClientLocal {
    pub async fn get_job_info(&self) -> Result<()> {
        let v: serde_json::Value = self.get_response("api/job").await?;

        debug!("job info: {:#?}", v);

        unimplemented!()
    }

    pub async fn get_printer_state(&self) -> Result<octo_types::printer_status::PrinterStatus> {
        let v: octo_types::printer_status::PrinterStatus = self.get_response("api/printer").await?;
        Ok(v)
    }
}
