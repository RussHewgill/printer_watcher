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
pub mod notifications;
pub mod status;
pub mod streaming;
pub mod ui;
// pub mod ui;

use anyhow::{anyhow, bail, ensure, Context, Result};
use streaming::StreamCmd;
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

/// Prusa Test
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
    let id = env::var("PRUSA_ID")?;
    let id: PrinterId = id.into();

    let link_key = env::var("PRUSA_LINK_KEY")?;

    let printer = config::printer_config::PrinterConfigPrusa {
        id,
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

    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel::<conn_manager::WorkerCmd>();
    let (msg_tx, mut msg_rx) = tokio::sync::mpsc::unbounded_channel::<PrinterConnMsg>();

    let (tx, _) = tokio::sync::mpsc::unbounded_channel::<(
        PrinterId,
        conn_manager::worker_message::WorkerMsg,
    )>();

    let (_, kill_rx) = tokio::sync::oneshot::channel::<()>();

    // let client = conn_manager::conn_prusa::PrusaClient::new(Arc::new(RwLock::new(printer)))?;
    let client = conn_manager::conn_prusa::prusa_local::PrusaClientLocal::new(
        Arc::new(RwLock::new(printer)),
        tx,
        cmd_rx,
        kill_rx,
        None,
    )?;

    let resp = client.get_job().await?;
    // debug!("resp = {:#?}", resp);

    // let thumbnail = resp.file.refs.download.clone();
    let thumbnail = resp.file.refs.icon.clone();
    // let thumbnail = resp.file.refs.thumbnail.clone();

    debug!("thumbnail = {:?}", thumbnail);

    let host = env::var("PRUSA_CONNECT_HOST")?;

    let url = format!("http://{}{}", host, thumbnail);

    debug!("url = {:?}", url);

    let client = reqwest::ClientBuilder::new().build()?;

    let key = env::var("PRUSA_LINK_KEY")?;
    let mut resp = client
        .get(&url)
        .header("X-Api-Key", &key)
        // .header("Digest", &key)
        // .header(
        //     "Authorization",
        //     r#"Digest username="maker", realm="Printer API", uri="/thumb/l/usb/BAB~AA94.BGC""#,
        // )
        .send()
        .await?;

    debug!("resp = {:?}", resp);

    let t0 = std::time::Instant::now();
    let bytes = resp.bytes().await?;
    let t1 = std::time::Instant::now();

    let duration = t1 - t0;
    debug!("duration = {:?}", duration);

    // let path = "thumbnail.png";
    let path = "icon.png";
    // let path = "dl.gcode";
    std::fs::write(path, bytes)?;

    Ok(())
}

/// bambu still camera test
#[cfg(feature = "nope")]
// #[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    logging::init_logs();

    let host = env::var("BAMBU_IP").unwrap();
    let access_code = env::var("BAMBU_ACCESS_CODE").unwrap();
    let serial = env::var("BAMBU_SERIAL").unwrap();
    let id = env::var("BAMBU_ID").unwrap();
    let id: PrinterId = id.into();

    let (kill_tx, kill_rx) = tokio::sync::mpsc::channel::<()>(1);
    let (msg_tx, mut msg_rx) = tokio::sync::mpsc::unbounded_channel::<()>();

    let mut ctx = egui::Context::default();

    let image = egui::ColorImage::new([32, 32], egui::Color32::from_gray(0));
    let handle = ctx.load_texture("icon.png", image, Default::default());

    let mut conn = streaming::bambu::bambu_img::JpegStreamViewer::new(
        id,
        serial,
        host,
        access_code,
        handle,
        kill_rx,
        // msg_tx,
    )
    .await
    .unwrap();

    conn.run().await.unwrap();

    Ok(())
}

#[cfg(feature = "nope")]
// #[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    logging::init_logs();

    let host = env::var("KLIPPER_HOST").unwrap();
    let id = env::var("KLIPPER_ID").unwrap();
    let id: PrinterId = id.into();

    #[cfg(feature = "nope")]
    {
        let url = format!("ws://{}:{}/websocket", host, 80);

        let rpc_client = jsonrpsee::ws_client::WsClientBuilder::default()
            .build(&url)
            .await?;

        use jsonrpsee::core::client::ClientT;
        use jsonrpsee::core::client::SubscriptionClientT;

        let mut params = jsonrpsee::core::params::ObjectParams::new();
        params.insert("client_name", "printer_watcher")?;
        params.insert("version", "0.1.0")?;
        params.insert("type", "other")?;
        params.insert("url", "http://github.com/arksine/moontest")?;

        let res: serde_json::Value = rpc_client
            .request("server.connection.identify", params)
            .await?;
        let id = res["connection_id"].as_u64().unwrap();
        debug!("id = {:?}", id);

        // let res: serde_json::Value = rpc_client
        //     .request("printer.objects.list", jsonrpsee::rpc_params![])
        //     .await?;
        // debug!("res = {:?}", res);

        let mut params = jsonrpsee::core::params::ObjectParams::new();
        params.insert(
            "objects",
            serde_json::json!({
                "gcode_move": serde_json::Value::Null,
                "toolhead": ["position", "status"],
            }),
        )?;

        let res: serde_json::Value = rpc_client
            .request("printer.objects.subscribe", params)
            .await?;
        debug!("res = {:?}", res);

        let mut sub: jsonrpsee::core::client::Subscription<serde_json::Value> = rpc_client
            .subscribe_to_method("notify_status_update")
            .await?;

        loop {
            let msg = sub.next().await.unwrap();
            debug!("msg = {:#?}", msg);
        }
    }

    // #[cfg(feature = "nope")]
    {
        let printer = config::printer_config::PrinterConfigKlipper::from_id(
            "test_printer".to_string(),
            host,
            id.clone(),
        );
        let printer = Arc::new(RwLock::new(printer));
        // let printer = PrinterConfig::Klipper(id.clone(), );

        let (tx, _) = tokio::sync::mpsc::unbounded_channel::<(
            PrinterId,
            conn_manager::worker_message::WorkerMsg,
        )>();
        let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel::<conn_manager::WorkerCmd>();
        let (kill_tx, kill_rx) = tokio::sync::oneshot::channel::<()>();

        let mut client =
            conn_manager::conn_klipper::KlipperClient::new(id, printer, tx, cmd_rx, kill_rx)
                .await?;

        debug!("running client");

        client.run().await?;

        debug!("done");
    }

    Ok(())
}

/// MARK: Main
#[allow(unreachable_code)]
// #[cfg(feature = "nope")]
fn main() -> eframe::Result<()> {
    let _ = dotenvy::dotenv();
    logging::init_logs();

    let mut config = AppConfig::load_from_file("config.toml").unwrap_or_default();
    // let mut config = AppConfig::default();

    /// add bambu
    #[cfg(feature = "nope")]
    {
        let host = env::var("BAMBU_IP").unwrap();
        let access_code = env::var("BAMBU_ACCESS_CODE").unwrap();
        let serial = env::var("BAMBU_SERIAL").unwrap();
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

        config.add_printer_blocking(printer.clone()).unwrap();
        // printer_order.insert(ui::model::GridLocation::new(1, 0), id.to_string());
    }

    /// add prusa
    #[cfg(feature = "nope")]
    {
        let host = env::var("PRUSA_CONNECT_HOST").unwrap();
        let token = env::var("PRUSA_CONNECT_TOKEN").unwrap();
        let serial = env::var("PRUSA_SERIAL").unwrap();

        let id = env::var("PRUSA_ID").unwrap();
        let id: PrinterId = id.into();

        let link_key = env::var("PRUSA_LINK_KEY").unwrap();

        let octo = config::printer_config::PrinterConfigOcto {
            host: env::var("OCTO_URL").unwrap(),
            token: env::var("OCTO_TOKEN").unwrap(),
        };

        let printer = config::printer_config::PrinterConfigPrusa {
            id: id.clone(),
            name: "test_prusa_printer".to_string(),
            host,
            key: link_key,
            serial,
            token,
            octo: Some(octo),
        };

        let printer = PrinterConfig::Prusa(id.clone(), Arc::new(RwLock::new(printer)));
        // printer_order.insert(ui::model::GridLocation::new(1, 0), id.to_string());
        config.add_printer_blocking(printer).unwrap();
    }

    // config.save_to_file("config_test.toml").unwrap();
    // return Ok(());

    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel::<PrinterConnCmd>();
    let (msg_tx, mut msg_rx) = tokio::sync::mpsc::unbounded_channel::<PrinterConnMsg>();
    let (stream_tx, stream_rx) = tokio::sync::mpsc::unbounded_channel::<streaming::StreamCmd>();

    let printer_states: Arc<DashMap<PrinterId, GenericPrinterState>> = Arc::new(DashMap::new());
    let printer_states2 = printer_states.clone();

    debug!("spawning tokio runtime");
    let configs2 = config.clone();
    let cmd_tx2 = cmd_tx.clone();
    let stream_tx2 = stream_tx.clone();
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let mut conn = conn_manager::PrinterConnManager::new(
                configs2,
                printer_states2,
                cmd_tx2,
                cmd_rx,
                msg_tx,
                stream_tx2,
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

    // #[cfg(feature = "nope")]
    let stream_tx3 = stream_tx.clone();
    debug!("spawning streaming runtime");
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let mut stream_manager = streaming::StreamManager::new(stream_tx3, stream_rx);

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

            Ok(Box::new(ui::app::App::new(
                cc,
                config,
                printer_states,
                cmd_tx,
                msg_rx,
                stream_tx,
                // handles,
                // graphs,
            )))
        }),
    )
}

/// octo test
#[cfg(feature = "nope")]
// #[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    logging::init_logs();

    let host = env::var("OCTO_URL")?;
    let token = env::var("OCTO_TOKEN")?;
    let id = env::var("OCTO_ID")?;
    let id: PrinterId = id.into();

    let (worker_msg_tx, mut worker_msg_rx) = tokio::sync::mpsc::unbounded_channel::<(
        PrinterId,
        conn_manager::worker_message::WorkerMsg,
    )>();
    let (worker_cmd_tx, worker_cmd_rx) =
        tokio::sync::mpsc::unbounded_channel::<conn_manager::WorkerCmd>();
    let (kill_tx, kill_rx) = tokio::sync::oneshot::channel::<()>();

    let printer = config::printer_config::PrinterConfigOcto {
        id: id.clone(),
        name: "test_octo_printer".to_string(),
        host,
        token,
    };
    let printer = Arc::new(RwLock::new(printer));

    let mut client = conn_manager::conn_octoprint::OctoClientLocal::new(
        printer,
        worker_msg_tx,
        worker_cmd_rx,
        kill_rx,
        None,
    )?;

    // let cmd = conn_manager::conn_octoprint::octo_commands::OctoCmd::Home {
    //     x: true,
    //     y: true,
    //     z: false,
    // };

    // let cmd = conn_manager::conn_octoprint::octo_commands::OctoCmd::ParkTool;

    // let cmd = conn_manager::conn_octoprint::octo_commands::OctoCmd::unload_filament(0);
    let cmd = conn_manager::conn_octoprint::octo_commands::OctoCmd::load_pla(vec![0]);

    let res = client.send_command(&cmd).await?;

    // debug!("cmd = {:?}", cmd.to_json());

    // let v: serde_json::Value = client.get_response("api/version").await?;
    // let v: serde_json::Value = client.get_response("api/printer").await?;
    // let v: serde_json::Value = client.send_command(&cmd).await?;

    // let update = client.get_update().await?;
    // debug!("update = {:#?}", update);

    // let v = std::fs::read_to_string("example_state.json")?;
    // // let v: serde_json::Value = serde_json::from_str(&v)?;
    // let v: conn_manager::conn_octoprint::octo_types::printer_status::PrinterStatus =
    //     serde_json::from_str(&v)?;
    // debug!("v = {:#?}", v);

    Ok(())
}

/// video widget test
#[cfg(feature = "nope")]
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
