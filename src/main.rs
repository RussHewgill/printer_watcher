#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unused_doc_comments)]
#![allow(unused_labels)]
#![allow(unexpected_cfgs)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod auth;
// pub mod camera;
pub mod config;
pub mod conn_manager;
pub mod logging;
pub mod status;
pub mod streaming;
pub mod ui;
// pub mod ui;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use dashmap::DashMap;
use status::GenericPrinterState;

// use iced::Settings;
use std::{env, sync::Arc};
use tokio::sync::RwLock;

use crate::{
    config::{printer_config::PrinterConfig, AppConfig},
    conn_manager::{PrinterConnCmd, PrinterConnMsg},
};
use config::printer_id::PrinterId;
// use ui::model::SavedAppState;

#[cfg(feature = "nope")]
// #[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    logging::init_logs();

    let host = env::var("PRUSA_CONNECT_HOST")?;
    // let host = "https://connect.prusa3d.com".to_string();
    // let connect_key = env::var("PRUSA_CONNECT_KEY")?;
    let token = env::var("PRUSA_CONNECT_TOKEN")?;
    let serial = env::var("PRUSA_SERIAL")?;

    let link_key = env::var("PRUSA_LINK_KEY")?;

    let printer = config::printer_config::PrinterConfigPrusa {
        name: "test_printer".to_string(),
        host: host.clone(),
        key: link_key,
        serial,
        token,
    };

    const URL_VERSION: &'static str = "api/version";
    const URL_INFO: &'static str = "api/v1/info";
    const URL_STATUS: &'static str = "api/v1/status";
    const URL_JOB: &'static str = "api/v1/job";

    // let client = conn_manager::conn_prusa::PrusaClient::new(Arc::new(RwLock::new(printer)))?;
    let client = conn_manager::conn_prusa::prusa_local::PrusaClientLocal::new(Arc::new(
        RwLock::new(printer),
    ))?;

    // let resp: serde_json::Value = client.get_response("api/v1/status").await?;
    // debug!("resp = {}", serde_json::to_string_pretty(&resp).unwrap());

    let resp = client.get_job().await?;
    debug!("resp = {:#?}", resp);

    // // let url = format!("https://{}:443/{}", host, URL_INFO);
    // let url = format!("http://{}/{}", host, URL_STATUS);

    // let client = reqwest::ClientBuilder::new()
    //     .use_rustls_tls()
    //     .danger_accept_invalid_certs(true)
    //     .build()?;
    // let res = client
    //     .get(&url)
    //     // .header("Authorization", &format!("Bearer {}", token.get_token()))
    //     // .header("Content-Type", "application/json")
    //     .header("X-Api-Key", key)
    //     .send()
    //     .await?;

    // let text: serde_json::Value = res.json().await?;
    // // let text = res.text().await?;

    // debug!("text = {:#?}", text);

    Ok(())
}

/// MARK: Main
#[cfg(feature = "nope")]
fn main() -> eframe::Result<()> {
    let _ = dotenvy::dotenv();
    logging::init_logs();

    let mut config = AppConfig::empty();
    // let mut printer_order = std::collections::HashMap::new();

    /// add bambu
    // #[cfg(feature = "nope")]
    {
        let host = env::var("BAMBU_IP").unwrap();
        let access_code = env::var("BAMBU_ACCESS_CODE").unwrap();
        let serial = env::var("BAMBU_IDENT").unwrap();
        let id = env::var("BAMBU_ID").unwrap();
        let id: PrinterId = id.into();

        let printer = config::printer_config::PrinterConfigBambu::from_id(
            serial.clone(),
            "test_bambu_printer".to_string(),
            host,
            access_code,
            id.clone(),
        );
        let printer = PrinterConfig::Bambu(id.clone(), Arc::new(RwLock::new(printer)));

        config.add_printer_blocking(printer.clone()).unwrap();
        // printer_order.insert(ui::model::GridLocation::new(0, 0), id.to_string());
    }

    /// add klipper
    #[cfg(feature = "nope")]
    {
        let host = env::var("KLIPPER_HOST").unwrap();
        let id = env::var("KLIPPER_ID").unwrap();
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

    /// add prusa
    // #[cfg(feature = "nope")]
    {
        let host = env::var("PRUSA_CONNECT_HOST").unwrap();
        let token = env::var("PRUSA_CONNECT_TOKEN").unwrap();
        let serial = env::var("PRUSA_SERIAL").unwrap();

        let id = env::var("PRUSA_ID").unwrap();
        let id: PrinterId = id.into();

        let link_key = env::var("PRUSA_LINK_KEY").unwrap();

        let printer = config::printer_config::PrinterConfigPrusa {
            id: id.clone(),
            name: "test_prusa_printer".to_string(),
            host,
            key: link_key,
            serial,
            token,
        };

        let printer = PrinterConfig::Prusa(id.clone(), Arc::new(RwLock::new(printer)));
        // printer_order.insert(ui::model::GridLocation::new(1, 0), id.to_string());
        config.add_printer_blocking(printer).unwrap();
    }

    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel::<PrinterConnCmd>();
    let (msg_tx, mut msg_rx) = tokio::sync::mpsc::unbounded_channel::<PrinterConnMsg>();
    // let printer_states = Arc::new(dashmap::DashMap::new());

    let printer_states: Arc<DashMap<PrinterId, GenericPrinterState>> = Arc::new(DashMap::new());
    let printer_states2 = printer_states.clone();

    debug!("spawning tokio runtime");
    let configs2 = config.clone();
    let cmd_tx2 = cmd_tx.clone();
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let mut conn = conn_manager::PrinterConnManager::new(
                configs2,
                printer_states2,
                cmd_tx2,
                cmd_rx,
                msg_tx,
            )
            .await;
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

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            // .with_icon(icon)
            .with_inner_size([850.0, 750.0])
            .with_min_inner_size([550.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Printer Watcher",
        native_options,
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Box::new(ui::app::App::new(
                cc,
                config,
                printer_states,
                cmd_tx,
                msg_rx,
                // stream_cmd_tx,
                // handles,
                // graphs,
            ))
        }),
    )
}

/// video widget test
// #[cfg(feature = "nope")]
fn main() -> eframe::Result<()> {
    let _ = dotenvy::dotenv();
    logging::init_logs();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            // .with_icon(icon)
            .with_inner_size([850.0, 750.0])
            .with_min_inner_size([550.0, 400.0]),
        ..Default::default()
    };

    let (stream_tx, stream_rx) = tokio::sync::mpsc::unbounded_channel::<streaming::StreamCmd>();

    debug!("spawning tokio runtime");
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let mut stream_manager = streaming::StreamManager::new(stream_rx);

            debug!("starting stream manager");
            loop {
                if let Err(e) = stream_manager.run().await {
                    error!("error in stream manager: {:?}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    debug!("restarting stream manager");
                }
            }
        });
    });

    eframe::run_native(
        "Printer Watcher",
        native_options,
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Box::new(ui::video_player::test_player::TestVideoApp::new(
                cc, stream_tx,
            ))
        }),
    )
}

/// Retina test
#[cfg(feature = "nope")]
// #[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv()?;
    logging::init_logs();

    let username = env::var("RTSP_USER")?;
    let password = env::var("RTSP_PASS")?;

    let creds: retina::client::Credentials = retina::client::Credentials { username, password };

    let host = env::var("RTSP_URL")?;
    let host = format!("rtsp://{}", host);
    let url = url::Url::parse(&host)?;

    let stop_signal = Box::pin(tokio::signal::ctrl_c());

    let session_group = Arc::new(retina::client::SessionGroup::default());
    let mut session = retina::client::Session::describe(
        url.clone(),
        retina::client::SessionOptions::default()
            .creds(Some(creds))
            .session_group(session_group.clone())
            .user_agent("Retina jpeg example".to_owned()), // .teardown(opts.teardown),
    )
    .await?;

    let video_stream_i = {
        let s = session.streams().iter().position(|s| {
            if s.media() == "video" {
                if s.encoding_name() == "h264" {
                    info!("Using h264 video stream");
                    return true;
                }
                info!(
                    "Ignoring {} video stream because it's unsupported",
                    s.encoding_name(),
                );
            }
            false
        });
        if s.is_none() {
            info!("No suitable video stream found");
        }
        s
    };

    if let Some(i) = video_stream_i {
        session
            .setup(
                i,
                retina::client::SetupOptions::default().transport(retina::client::Transport::Udp(
                    retina::client::UdpTransportOptions::default(),
                )),
            )
            .await?;
    }
    if video_stream_i.is_none() {
        bail!("Exiting because no video or audio stream was selected; see info log messages above");
    }

    debug!("video_stream_i = {:?}", video_stream_i);
    // let result = write_jpeg(session, stop_signal).await;
    let result = streaming::write_frames(session, stop_signal).await;

    // Session has now been dropped, on success or failure. A TEARDOWN should
    // be pending if necessary. session_group.await_teardown() will wait for it.
    if let Err(e) = session_group.await_teardown().await {
        error!("TEARDOWN failed: {}", e);
    }
    // result
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
