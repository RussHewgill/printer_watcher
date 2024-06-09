use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::{
    config::{printer_id::PrinterId, AppConfig},
    conn_manager::{PrinterConnCmd, PrinterConnMsg},
};

use super::ui_types::{GridLocation, Tab};

#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
pub struct App {
    pub current_tab: Tab,

    #[serde(skip)]
    pub config: AppConfig,

    #[serde(skip)]
    pub cmd_tx: Option<tokio::sync::mpsc::UnboundedSender<PrinterConnCmd>>,

    // #[serde(skip)]
    // pub stream_cmd_tx: Option<tokio::sync::mpsc::UnboundedSender<StreamCmd>>,
    #[serde(skip)]
    pub msg_rx: Option<tokio::sync::mpsc::UnboundedReceiver<PrinterConnMsg>>,

    // #[serde(skip)]
    // pub printer_states: Arc<DashMap<PrinterId, PrinterStatus>>,
    pub printer_order: HashMap<GridLocation, PrinterId>,
    #[serde(skip)]
    pub unplaced_printers: Vec<PrinterId>,

    #[serde(skip)]
    pub selected_stream: Option<PrinterId>,
    // #[serde(skip)]
    // pub printer_config_page: PrinterConfigPage,

    // pub options: AppOptions,

    // #[serde(skip)]
    // pub login_window: Option<AppLogin>,

    // /// selected printer, show right panel when Some
    // pub selected_printer_controls: Option<PrinterId>,

    // #[serde(skip)]
    // pub printer_textures: Arc<DashMap<PrinterId, WebcamTexture>>,
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        config: AppConfig,
        cmd_tx: tokio::sync::mpsc::UnboundedSender<PrinterConnCmd>,
        msg_rx: tokio::sync::mpsc::UnboundedReceiver<PrinterConnMsg>,
    ) -> Self {
        let mut out = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
            // warn!("using default app state");
            // Self::default()
        } else {
            Self::default()
        };

        // out.printer_states = printer_states;
        out.config = config;

        out.cmd_tx = Some(cmd_tx);
        out.msg_rx = Some(msg_rx);
        // out.stream_cmd_tx = Some(stream_cmd_tx);

        out.unplaced_printers = out.config.printer_ids();
        /// for each printer that isn't in printer_order, queue to add
        for (_, id) in out.printer_order.iter() {
            out.unplaced_printers.retain(|p| p != id);
        }

        // out.printer_textures = printer_textures;

        /// remove printers that were previously placed but are no longer in the config
        {
            let current_printers = out
                .config
                .printer_ids()
                .into_iter()
                // .map(|c| c.serial.clone())
                .collect::<HashSet<_>>();
            out.unplaced_printers
                .retain(|p| current_printers.contains(p));
            out.printer_order
                .retain(|_, v| current_printers.contains(v));
        }

        out
    }
}
