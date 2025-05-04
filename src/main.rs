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
pub mod error_logging;
pub mod logging;
pub mod notifications;
// pub mod profiles;
pub mod fake_printer;
pub mod status;
pub mod streaming;
pub mod ui;
pub mod utils;
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
use streaming::StreamCmd;
// use ui::model::SavedAppState;

/// Prusa Connect Test
#[cfg(feature = "nope")]
// #[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    logging::init_logs();

    let printer = {
        let host = env::var("PRUSA_CONNECT_HOST")?;
        // let host = "https://connect.prusa3d.com".to_string();
        // let connect_key = env::var("PRUSA_CONNECT_KEY")?;
        let token = env::var("PRUSA_CONNECT_TOKEN")?;
        let serial = env::var("PRUSA_SERIAL")?;
        let id = env::var("PRUSA_ID")?;
        let id: PrinterId = id.into();
        let fingerprint = std::env::var("PRUSA_CONNECT_FINGERPRINT")?;

        let link_key = env::var("PRUSA_LINK_KEY")?;

        config::printer_config::PrinterConfigPrusa {
            id,
            name: "test_printer".to_string(),
            host: host.clone(),
            key: link_key,
            serial,
            fingerprint,
            token,
            octo: None,
            rtsp: None,
        }
    };

    #[cfg(feature = "nope")]
    {
        // let url = env::var("PRUSA_CONNECT_TEST_URL")?;
        let url = "https://connect.prusa3d.com/api/version".to_string();

        // debug!("url = {:?}", url);

        let client = reqwest::ClientBuilder::new()
            .use_rustls_tls()
            // .with_root_certificates(client_config.root_store)
            // .tls_built_in_native_certs(true)
            // .tls_built_in_root_certs(true)
            .danger_accept_invalid_certs(true)
            .build()?;

        let req = client.get(&url);
        debug!("sending request");

        let response = req.send().await?;
    }

    let printer = Arc::new(RwLock::new(printer));

    let mut client = conn_manager::conn_prusa::prusa_cloud::PrusaClient::new(printer)?;

    // client.get_info().await?;
    client.register().await?;

    client.get_telemetry().await?;

    Ok(())
}

#[cfg(feature = "nope")]
mod prusa_test {
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
            octo: None,
            rtsp: None,
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
        )
        .await?;

        let url = "api/printer";

        let resp: serde_json::Value = client.get_response(url).await?;

        debug!("resp = {}", serde_json::to_string_pretty(&resp)?);

        #[cfg(feature = "nope")]
        {
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
        }

        Ok(())
    }
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

    // let mut conn = streaming::bambu::bambu_img::JpegStreamViewer::new(
    //     id,
    //     serial,
    //     host,
    //     access_code,
    //     handle,
    //     kill_rx,
    // )
    // .await
    // .unwrap();

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

/// profiles test
#[cfg(feature = "nope")]
#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    logging::init_logs();

    // let appdata = env::var("APPDATA")?;
    // let path = format!(
    //     // "{}\\OrcaSlicer\\system\\Custom\\filament\\fdm_filament_common.json",
    //     "{}\\OrcaSlicer\\system\\Custom\\filament\\fdm_filament_pla.json",
    //     appdata
    // );

    // let f = std::fs::read_to_string(path)?;

    // let f: profiles::FilamentProfile = serde_json::from_str(&f)?;

    // debug!("f = {:#?}", f);

    let _ = profiles::profiles_db::ProfileDb::new().await?;

    Ok(())
}

/// Klipper test
#[tokio::main]
#[cfg(feature = "nope")]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    logging::init_logs();

    let id = PrinterId::from_id("tracerbullet_afsdhasdfhsdh");

    let printer = config::printer_config::PrinterConfigKlipper {
        id: id.clone(),
        host: "192.168.0.245".to_string(),
        name: "Tracer Bullet".to_string(),
        toolchanger: true,
        tools: 4,
    };

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<(
        PrinterId,
        conn_manager::worker_message::WorkerMsg,
    )>();

    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel::<conn_manager::WorkerCmd>();
    let (kill_tx, kill_rx) = tokio::sync::oneshot::channel::<()>();

    let mut client = conn_manager::conn_klipper::KlipperClient::new(
        id,
        Arc::new(RwLock::new(printer)),
        tx,
        cmd_rx,
        kill_rx,
    )
    .await?;

    debug!("Running client");
    client.run().await?;

    Ok(())
}

/// bambu state json test
#[cfg(feature = "nope")]
fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    logging::init_logs();

    let json = {
        let json = r#"{
    "print": {
        "2D": {
            "bs": {
                "bi": [
                    {
                        "est_time": 0,
                        "idx": 0,
                        "print_then": false,
                        "proc_list": [],
                        "step_type": 1,
                        "tool_info": {
                            "color": "",
                            "diameter": 0.0,
                            "id": 8978432
                        },
                        "type": 137
                    }
                ],
                "total_time": 0
            },
            "cond": 15,
            "cur_stage": {
                "idx": 0,
                "left_time": 0,
                "process": 0,
                "state": 0
            },
            "first_confirm": false,
            "makeable": false,
            "material": {
                "cur_id_list": [],
                "state": 0,
                "tar_id": "",
                "tar_name": ""
            }
        },
        "3D": {
            "layer_num": 0,
            "total_layer_num": 0
        },
        "ams": {
            "ams": [
                {
                    "dry_time": 0,
                    "humidity": "2",
                    "humidity_raw": "41",
                    "id": "0",
                    "info": "1103",
                    "temp": "23.1",
                    "tray": [
                        {
                            "id": "0",
                            "state": 9
                        },
                        {
                            "id": "1",
                            "state": 0
                        },
                        {
                            "id": "2",
                            "state": 0
                        },
                        {
                            "id": "3",
                            "state": 0
                        }
                    ]
                }
            ],
            "ams_exist_bits": "1",
            "ams_exist_bits_raw": "1",
            "cali_id": 0,
            "cali_stat": 5,
            "insert_flag": false,
            "power_on_flag": false,
            "tray_exist_bits": "1",
            "tray_is_bbl_bits": "0",
            "tray_now": "255",
            "tray_pre": "255",
            "tray_read_done_bits": "1",
            "tray_reading_bits": "0",
            "tray_tar": "255",
            "unbind_ams_stat": 0,
            "version": 168
        },
        "ams_rfid_status": 0,
        "ams_status": 0,
        "ap_err": 0,
        "aux": "4",
        "aux_part_fan": false,
        "batch_id": 0,
        "bed_target_temper": 0.0,
        "bed_temper": 40.0,
        "big_fan1_speed": "0",
        "big_fan2_speed": "0",
        "cali_version": 0,
        "canvas_id": 0,
        "cfg": "1815DAD8",
        "chamber_temper": 30.0,
        "command": "push_status",
        "cooling_fan_speed": "0",
        "ctt": 0.0,
        "design_id": "",
        "device": {
            "airduct": {
                "modeCur": 0,
                "modeList": [
                    {
                        "ctrl": [
                            16,
                            32,
                            48
                        ],
                        "modeId": 0,
                        "off": [
                            96
                        ]
                    },
                    {
                        "ctrl": [
                            16
                        ],
                        "modeId": 1,
                        "off": [
                            32,
                            48
                        ]
                    },
                    {
                        "ctrl": [
                            48
                        ],
                        "modeId": 2,
                        "off": []
                    }
                ],
                "parts": [
                    {
                        "func": 0,
                        "id": 16,
                        "range": 6553600,
                        "state": 0
                    },
                    {
                        "func": 1,
                        "id": 32,
                        "range": 6553600,
                        "state": 0
                    },
                    {
                        "func": 2,
                        "id": 48,
                        "range": 6553600,
                        "state": 0
                    },
                    {
                        "func": 3,
                        "id": 96,
                        "range": 6553600,
                        "state": 0
                    }
                ]
            },
            "bed_temp": 40,
            "cam": {
                "laser": {
                    "cond": 253,
                    "state": 0
                }
            },
            "cham_temp": 30,
            "ext_tool": {
                "calib": 2,
                "mount": 0,
                "type": ""
            },
            "extruder": {
                "info": [
                    {
                        "filam_bak": [],
                        "hnow": 0,
                        "hpre": 0,
                        "htar": 0,
                        "id": 0,
                        "info": 9,
                        "snow": 65535,
                        "spre": 65535,
                        "star": 65535,
                        "stat": 0,
                        "temp": 39
                    },
                    {
                        "filam_bak": [],
                        "hnow": 1,
                        "hpre": 1,
                        "htar": 1,
                        "id": 1,
                        "info": 8,
                        "snow": 65279,
                        "spre": 65279,
                        "star": 65279,
                        "stat": 0,
                        "temp": 37
                    }
                ],
                "state": 2
            },
            "fan": 0,
            "laser": {
                "power": 0
            },
            "nozzle": {
                "exist": 3,
                "info": [
                    {
                        "diameter": 0.4,
                        "id": 0,
                        "tm": 0,
                        "type": "HS01",
                        "wear": 0
                    },
                    {
                        "diameter": 0.4,
                        "id": 1,
                        "tm": 0,
                        "type": "HS01",
                        "wear": 0
                    }
                ],
                "state": 0
            },
            "plate": {
                "base": 1,
                "cali2d_id": "",
                "cur_id": "",
                "mat": 1,
                "tar_id": ""
            },
            "type": 1
        },
        "err": "0",
        "fail_reason": "0",
        "fan_gear": 0,
        "file": "/usr/etc/print/O1D/new_machine_auto_cali_for_user.gcode",
        "force_upgrade": false,
        "fun": "1AFFF8CFB",
        "gcode_file": "/usr/etc/print/O1D/new_machine_auto_cali_for_user.gcode",
        "gcode_file_prepare_percent": "100",
        "gcode_state": "FINISH",
        "heatbreak_fan_speed": "0",
        "hms": [],
        "home_flag": -1067068400,
        "hw_switch_state": 0,
        "ipcam": {
            "agora_service": "disable",
            "brtc_service": "enable",
            "bs_state": 0,
            "ipcam_dev": "1",
            "ipcam_record": "enable",
            "laser_preview_res": 5,
            "mode_bits": 2,
            "resolution": "1080p",
            "rtsp_url": "disable",
            "timelapse": "disable",
            "tl_store_hpd_type": 2,
            "tl_store_path_type": 2,
            "tutk_server": "enable"
        },
        "job": {
            "cur_stage": {
                "idx": 0,
                "state": 0
            },
            "stage": [
                {
                    "color": [
                        "",
                        ""
                    ],
                    "diameter": [
                        0.0,
                        0.0
                    ],
                    "est_time": 0,
                    "heigh": 0.0,
                    "idx": 0,
                    "platform": "",
                    "print_then": false,
                    "proc_list": [],
                    "tool": [
                        "H000",
                        "H000"
                    ],
                    "type": 2
                }
            ]
        },
        "job_attr": 0,
        "job_id": "0",
        "layer_num": 0,
        "lights_report": [
            {
                "mode": "on",
                "node": "chamber_light"
            },
            {
                "mode": "flashing",
                "node": "work_light"
            },
            {
                "mode": "on",
                "node": "chamber_light2"
            }
        ],
        "maintain": 3,
        "mapping": [
            65280,
            65280,
            65280,
            65280,
            65280,
            65280,
            65280,
            65280,
            65280,
            65280,
            65280,
            65280,
            65280,
            65280,
            65280,
            65280,
            65280,
            65280,
            65280,
            65280,
            65280,
            65280,
            65280,
            65280,
            65280,
            65280,
            65280,
            65280,
            65280,
            65280,
            65280,
            65280
        ],
        "mc_action": 255,
        "mc_err": 0,
        "mc_percent": 100,
        "mc_print_error_code": "0",
        "mc_print_stage": "1",
        "mc_print_sub_stage": 0,
        "mc_remaining_time": 0,
        "mc_stage": 1,
        "model_id": "",
        "net": {
            "conf": 16,
            "info": [
                {
                    "ip": 385919168,
                    "mask": 16777215
                },
                {
                    "ip": 0,
                    "mask": 0
                }
            ]
        },
        "nozzle_diameter": "0.4",
        "nozzle_target_temper": 0.0,
        "nozzle_temper": 39.0,
        "nozzle_type": "stainless_steel",
        "online": {
            "ahb": true,
            "ext": true,
            "version": 7
        },
        "percent": 100,
        "plate_cnt": 0,
        "plate_id": 0,
        "plate_idx": 0,
        "prepare_per": 100,
        "print_error": 0,
        "print_gcode_action": 255,
        "print_real_action": 0,
        "print_type": "system",
        "profile_id": "",
        "project_id": "0",
        "queue": 0,
        "queue_est": 0,
        "queue_number": 0,
        "queue_sts": 0,
        "queue_total": 0,
        "remain_time": 0,
        "s_obj": [],
        "sdcard": false,
        "sequence_id": "2021",
        "spd_lvl": 2,
        "spd_mag": 100,
        "stat": "6208000",
        "state": 6,
        "stg": [
            13,
            50,
            47,
            25,
            3,
            48,
            39
        ],
        "stg_cur": -1,
        "subtask_id": "0",
        "subtask_name": "new_machine_auto_cali_for_user.gcode",
        "task_id": "4915",
        "total_layer_num": 0,
        "upgrade_state": {
            "ahb_new_version_number": "",
            "ams_new_version_number": "02.00.19.47",
            "consistency_request": false,
            "dis_state": 1,
            "err_code": 0,
            "ext_new_version_number": "",
            "force_upgrade": false,
            "idx": 7,
            "idx2": 1205931494,
            "lower_limit": "00.00.00.00",
            "message": "",
            "module": "",
            "new_version_state": 1,
            "ota_new_version_number": "01.01.01.00",
            "progress": "0",
            "sequence_id": 0,
            "sn": "0948AD532000009",
            "status": "IDLE"
        },
        "upload": {
            "file_size": 0,
            "finish_size": 0,
            "message": "Good",
            "oss_url": "",
            "progress": 0,
            "sequence_id": "0903",
            "speed": 0,
            "status": "idle",
            "task_id": "",
            "time_remaining": 0,
            "trouble_id": ""
        },
        "ver": "18",
        "vir_slot": [
            {
                "bed_temp": "0",
                "bed_temp_type": "0",
                "cali_idx": -1,
                "cols": [
                    "00000000"
                ],
                "ctype": 0,
                "drying_temp": "0",
                "drying_time": "0",
                "id": "254",
                "nozzle_temp_max": "0",
                "nozzle_temp_min": "0",
                "remain": 0,
                "tag_uid": "0000000000000000",
                "total_len": 330000,
                "tray_color": "00000000",
                "tray_diameter": "1.75",
                "tray_id_name": "",
                "tray_info_idx": "",
                "tray_sub_brands": "",
                "tray_type": "",
                "tray_uuid": "00000000000000000000000000000000",
                "tray_weight": "0",
                "xcam_info": "000000000000000000000000"
            },
            {
                "bed_temp": "0",
                "bed_temp_type": "0",
                "cali_idx": -1,
                "cols": [
                    "00000000"
                ],
                "ctype": 0,
                "drying_temp": "0",
                "drying_time": "0",
                "id": "255",
                "nozzle_temp_max": "0",
                "nozzle_temp_min": "0",
                "remain": 0,
                "tag_uid": "0000000000000000",
                "total_len": 330000,
                "tray_color": "00000000",
                "tray_diameter": "1.75",
                "tray_id_name": "",
                "tray_info_idx": "",
                "tray_sub_brands": "",
                "tray_type": "",
                "tray_uuid": "00000000000000000000000000000000",
                "tray_weight": "0",
                "xcam_info": "000000000000000000000000"
            }
        ],
        "vt_tray": {
            "bed_temp": "0",
            "bed_temp_type": "0",
            "cali_idx": -1,
            "cols": [
                "00000000"
            ],
            "ctype": 0,
            "drying_temp": "0",
            "drying_time": "0",
            "id": "255",
            "nozzle_temp_max": "0",
            "nozzle_temp_min": "0",
            "remain": 0,
            "tag_uid": "0000000000000000",
            "total_len": 330000,
            "tray_color": "00000000",
            "tray_diameter": "1.75",
            "tray_id_name": "",
            "tray_info_idx": "",
            "tray_sub_brands": "",
            "tray_type": "",
            "tray_uuid": "00000000000000000000000000000000",
            "tray_weight": "0",
            "xcam_info": "000000000000000000000000"
        },
        "wifi_signal": "-41dBm",
        "xcam": {
            "allow_skip_parts": false,
            "buildplate_marker_detector": true,
            "first_layer_inspector": true,
            "halt_print_sensitivity": "medium",
            "print_halt": true,
            "printing_monitor": true,
            "spaghetti_detector": true
        },
        "xcam_status": "0"
    }
}"#;
        json
    };

    // let json: crate::status::bambu_status::PrintStatus = serde_json::from_str(json)?;

    // debug!("json = {:#?}", json);

    Ok(())
}

/// MARK: Main
// #[allow(unreachable_code)]
#[cfg(feature = "nope")]
fn main() -> eframe::Result<()> {
    let _ = dotenvy::dotenv();
    logging::init_logs();

    let mut config = AppConfig::load_from_file("config.toml").unwrap_or_default();
    // let mut config = AppConfig::default();
    // debug!("loaded config from file");

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
    // warn!("Skipping streaming runtime");
    // #[cfg(feature = "nope")]
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
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "Printer Watcher",
        native_options,
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);

            /// repaint at least once per second
            let ctx2 = cc.egui_ctx.clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
                ctx2.request_repaint();
            });

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

/// GStreamer test
#[cfg(feature = "nope")]
fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    logging::init_logs();

    // // Create a GLib Main Context and Main Loop for this thread
    // let main_context = glib::MainContext::new();
    // let main_loop = glib::MainLoop::new(Some(&main_context), false);

    // crate::streaming::gstreamer_bambu::test_gstreamer().unwrap();

    /// use image crate to load an image with an unknown format from a file
    let path = "test.dat";

    // 1680x1080 Format: Bgr

    // load Vec<u8> from file
    let mut buffer_vec = Vec::new();
    let mut file = std::fs::File::open(path).unwrap();
    std::io::Read::read_to_end(&mut file, &mut buffer_vec).unwrap();

    let width = 1680;
    let height = 1080;

    // /// convert buffer from BGR to RGB
    // let mut rgb_buffer = Vec::new();

    // for i in (0..buffer_vec.len()).step_by(3) {
    //     rgb_buffer.push(buffer_vec[i + 2]);
    //     rgb_buffer.push(buffer_vec[i + 1]);
    //     rgb_buffer.push(buffer_vec[i]);
    // }

    let img_buffer: Option<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>> =
        image::ImageBuffer::from_raw(width, height, buffer_vec);

    // let img = image::open(path).unwrap();

    // debug!("img = {:#?}", img_buffer);

    // save to file

    img_buffer.unwrap().save("test.png").unwrap();

    Ok(())
}

/// Retina test
#[cfg(feature = "nope")]
// #[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    logging::init_logs();

    let url = url::Url::parse("rtsps://192.168.0.23/streaming/live/1").unwrap();

    debug!("url = {:?}", url);

    let session_group = Arc::new(retina::client::SessionGroup::default());

    let creds = retina::client::Credentials {
        username: "bblp".to_string(),
        password: env::var("RTSP_PASS").unwrap(),
    };

    let mut session = retina::client::Session::describe(
        url,
        retina::client::SessionOptions::default()
            .creds(Some(creds))
            .session_group(session_group.clone())
            // .teardown(opts.teardown)
            .user_agent("Retina jpeg example".to_owned()),
    )
    .await?;

    debug!("got session");

    for stream in session.streams().iter() {
        debug!("stream = {:?}", stream);
    }

    Ok(())
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
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    // let (stream_tx, stream_rx) = tokio::sync::mpsc::unbounded_channel::<streaming::StreamCmd>();
    // let stream_tx2 = stream_tx.clone();
    // // let stream_rx2 = stream_rx.clone();

    let (stream_tx, stream_rx) = crossbeam_channel::unbounded::<streaming::StreamCmd>();

    let stream_rx2 = stream_rx.clone();

    std::thread::spawn(|| {
        let pass = env::var("RTSP_PASS").unwrap();
        let player = streaming::gstreamer_bambu::GStreamerPlayer::new(
            // "bblp",
            &pass,
            "192.168.0.23",
            322,
            stream_rx2,
        );
        player.init().unwrap();
    });

    // debug!("spawning tokio runtime");
    #[cfg(feature = "nope")]
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let mut stream_manager = streaming::StreamManager::new(stream_tx2, stream_rx);

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

            Ok(Box::new(ui::video_player::test_player::TestVideoApp::new(
                cc, stream_tx,
            )))
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
