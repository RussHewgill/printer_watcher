#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unused_doc_comments)]
#![allow(unused_labels)]
#![allow(unexpected_cfgs)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod auth;
pub mod config;
pub mod conn_manager;
pub mod logging;
pub mod status;

use anyhow::{anyhow, bail, ensure, Context, Result};
use config::printer_id::PrinterId;
use tracing::{debug, error, info, trace, warn};

use std::{env, sync::Arc};
use tokio::sync::RwLock;

use crate::{
    config::{printer_config::PrinterConfig, AppConfig},
    conn_manager::{PrinterConnCmd, PrinterConnMsg},
};

// #[cfg(feature = "nope")]
#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv()?;
    logging::init_logs();

    let host = env::var("BAMBU_IP")?;
    let access_code = env::var("BAMBU_ACCESS_CODE")?;
    let serial = env::var("BAMBU_IDENT")?;
    let id = env::var("BAMBU_ID")?;

    let id: PrinterId = id.into();

    let config = config::printer_config::PrinterConfigBambu::from_id(
        serial.clone(),
        "test_printer".to_string(),
        host,
        access_code,
        id.clone(),
    );
    let config = PrinterConfig::Bambu(id.clone(), Arc::new(RwLock::new(config)));

    let mut configs = AppConfig::empty();

    configs.add_printer(config.clone()).await?;

    let config = configs.get_printer(&id.into()).unwrap();
    debug!("got printer");

    let printer_states = Arc::new(dashmap::DashMap::new());

    let (msg_tx, mut msg_rx) = tokio::sync::mpsc::unbounded_channel::<PrinterConnMsg>();
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel::<PrinterConnCmd>();

    let mut conn = conn_manager::PrinterConnManager::new(
        configs.clone(),
        printer_states,
        cmd_tx,
        cmd_rx,
        msg_tx,
    )
    .await;
    debug!("starting conn manager");

    conn.run().await?;

    Ok(())
}
