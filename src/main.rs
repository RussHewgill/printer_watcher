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
pub mod ui;

use anyhow::{anyhow, bail, ensure, Context, Result};
use config::printer_id::PrinterId;
use iced::Settings;
use tracing::{debug, error, info, trace, warn};
use ui::model::SavedAppState;

use std::{env, sync::Arc};
use tokio::sync::RwLock;

use crate::{
    config::{printer_config::PrinterConfig, AppConfig},
    conn_manager::{PrinterConnCmd, PrinterConnMsg},
};

fn main() -> Result<()> {
    dotenvy::dotenv()?;
    logging::init_logs();

    let mut config = AppConfig::empty();
    let mut printer_order = std::collections::HashMap::new();

    /// add bambu
    // #[cfg(feature = "nope")]
    {
        let host = env::var("BAMBU_IP")?;
        let access_code = env::var("BAMBU_ACCESS_CODE")?;
        let serial = env::var("BAMBU_IDENT")?;
        let id = env::var("BAMBU_ID")?;
        let id: PrinterId = id.into();

        let printer = config::printer_config::PrinterConfigBambu::from_id(
            serial.clone(),
            "test_printer".to_string(),
            host,
            access_code,
            id.clone(),
        );
        let printer = PrinterConfig::Bambu(id.clone(), Arc::new(RwLock::new(printer)));

        config.add_printer_blocking(printer.clone())?;
        printer_order.insert(ui::model::GridLocation::new(0, 0), id.to_string());
    }

    /// add klipper
    #[cfg(feature = "nope")]
    {
        let host = env::var("KLIPPER_HOST")?;
        let id = env::var("KLIPPER_ID")?;
        let id: PrinterId = id.into();

        let printer = config::printer_config::PrinterConfigKlipper::from_id(
            "test_printer".to_string(),
            host,
            id.clone(),
        );
        let printer = PrinterConfig::Klipper(id.clone(), Arc::new(RwLock::new(printer)));

        config.add_printer_blocking(printer.clone())?;
        printer_order.insert(ui::model::GridLocation::new(1, 0), id.to_string());
    }

    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel::<PrinterConnCmd>();
    let (msg_tx, mut msg_rx) = tokio::sync::mpsc::unbounded_channel::<PrinterConnMsg>();
    // let printer_states = Arc::new(dashmap::DashMap::new());

    let configs2 = config.clone();
    let cmd_tx2 = cmd_tx.clone();
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let mut conn =
                conn_manager::PrinterConnManager::new(configs2, cmd_tx2, cmd_rx, msg_tx).await;
            debug!("starting conn manager");

            conn.init().await.unwrap();
            loop {
                if let Err(e) = conn.run().await {
                    error!("error in conn manager: {:?}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    debug!("restarting conn manager");
                }
            }
        });
    });

    /// start UI
    // #[cfg(feature = "nope")]
    {
        let state = SavedAppState {
            current_tab: Default::default(),
            printer_order,
        };

        let flags = ui::model::AppFlags {
            state,
            config,
            cmd_tx,
            msg_rx,
            // printer_states,
        };

        let flags = iced::Settings::with_flags(flags);

        <crate::ui::model::AppModel as iced::Application>::run(flags)?;
    }

    // let rt = tokio::runtime::Runtime::new().unwrap();
    // rt.block_on(async move {
    //     loop {
    //         if let Some(msg) = msg_rx.recv().await {
    //             debug!("msg: {:?}", msg);
    //             //
    //         } else {
    //             panic!("msg_rx closed");
    //         }
    //     }
    // });

    Ok(())
}

/// klipper test
#[cfg(feature = "nope")]
// #[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv()?;
    logging::init_logs();

    /// proper test
    // #[cfg(feature = "nope")]
    {
        let host = env::var("KLIPPER_HOST")?;
        let id = env::var("KLIPPER_ID")?;
        let id: PrinterId = id.into();

        let config = config::printer_config::PrinterConfigKlipper::from_id(
            "test_printer".to_string(),
            host,
            id.clone(),
        );
        let config = PrinterConfig::Klipper(id.clone(), Arc::new(RwLock::new(config)));

        let mut configs = AppConfig::empty();

        configs.add_printer(config.clone()).await?;

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
    }

    /// websocket test
    #[cfg(feature = "nope")]
    {
        let host = env::var("KLIPPER_HOST")?;

        let url = url::Url::parse(&format!("ws://{}:{}/websocket", host, 80))?;

        debug!("url = {:?}", url);

        let (ws_stream, s) = tokio_tungstenite::connect_async(url)
            .await
            .expect("Failed to connect");
        debug!("connected");

        let (write, read) = futures::StreamExt::split(ws_stream);

        let read_future = futures::StreamExt::for_each(read, |message| async {
            debug!("receiving...");
            let data = message.unwrap().into_data();
            let d = String::from_utf8(data).unwrap();
            debug!("received: {}", d);
            // tokio::io::stdout().write(&data).await.unwrap();
            debug!("received...");
        });

        read_future.await;

        //
    }

    Ok(())
}

/// bambu test
#[cfg(feature = "nope")]
// #[tokio::main]
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
