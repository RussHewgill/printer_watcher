use std::sync::Arc;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::{
    config::{printer_id::PrinterId, AppConfig},
    conn_manager::{PrinterConnCmd, PrinterConnMsg},
    status::GenericPrinterState,
};

pub struct AppModel {
    pub current_tab: Tab,

    pub config: AppConfig,

    pub cmd_tx: tokio::sync::mpsc::UnboundedSender<PrinterConnCmd>,

    // pub stream_cmd_tx: Option<tokio::sync::mpsc::UnboundedSender<StreamCmd>>,
    pub msg_rx: tokio::sync::mpsc::UnboundedReceiver<PrinterConnMsg>,

    pub printer_states: Arc<DashMap<PrinterId, GenericPrinterState>>,

    pub app_options: AppOptions,
}

// #[derive(Default)]
pub struct AppFlags {
    pub state: SavedAppState,
    pub config: AppConfig,
    pub msg_rx: tokio::sync::mpsc::UnboundedReceiver<PrinterConnMsg>,
    pub cmd_tx: tokio::sync::mpsc::UnboundedSender<PrinterConnCmd>,
    pub printer_states: Arc<DashMap<PrinterId, GenericPrinterState>>,
}

#[derive(Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct SavedAppState {
    pub current_tab: Tab,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub enum Tab {
    Dashboard,
    Graphs,
    Printers,
    Projects,
    Options,
    // Debugging,
}

impl Default for Tab {
    fn default() -> Self {
        Self::Dashboard
    }
}

pub struct AppOptions {
    pub dark_mode: bool,
    pub dashboard_size: (usize, usize),
}

impl Default for AppOptions {
    fn default() -> Self {
        Self {
            dark_mode: false,
            // dashboard_size: (4, 2),
            dashboard_size: (6, 4),
        }
    }
}
