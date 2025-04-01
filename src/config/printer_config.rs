use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::printer_id::PrinterId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrinterType {
    Bambu,
    Klipper,
    Prusa,
}

#[derive(Debug, Clone)]
pub enum PrinterConfig {
    // Bambu(PrinterConfigBambu),
    // Klipper(PrinterConfigKlipper),
    Bambu(PrinterId, Arc<RwLock<PrinterConfigBambu>>),
    Klipper(PrinterId, Arc<RwLock<PrinterConfigKlipper>>),
    Prusa(PrinterId, Arc<RwLock<PrinterConfigPrusa>>),
    // Octoprint(PrinterId, Arc<RwLock<PrinterConfigOcto>>),
}

/// getters
impl PrinterConfig {
    pub fn id(&self) -> PrinterId {
        match self {
            PrinterConfig::Bambu(id, _) => id.clone(),
            PrinterConfig::Klipper(id, _) => id.clone(),
            PrinterConfig::Prusa(id, _) => id.clone(),
            // PrinterConfig::Octoprint(id, _) => id.clone(),
        }
    }

    pub async fn name(&self) -> String {
        match self {
            PrinterConfig::Bambu(_, config) => config.read().await.name.clone(),
            PrinterConfig::Klipper(_, config) => config.read().await.name.clone(),
            PrinterConfig::Prusa(_, config) => config.read().await.name.clone(),
            // PrinterConfig::Octoprint(_, config) => &config.read().await.name,
        }
    }

    pub fn name_blocking(&self) -> String {
        match self {
            PrinterConfig::Bambu(_, config) => config.blocking_read().name.clone(),
            PrinterConfig::Klipper(_, config) => config.blocking_read().name.clone(),
            PrinterConfig::Prusa(_, config) => config.blocking_read().name.clone(),
            // PrinterConfig::Octoprint(_, config) => &config.read().await.name,
        }
    }

    pub fn printer_type(&self) -> PrinterType {
        match self {
            PrinterConfig::Bambu(_, _) => PrinterType::Bambu,
            PrinterConfig::Klipper(_, _) => PrinterType::Klipper,
            PrinterConfig::Prusa(_, _) => PrinterType::Prusa,
            // PrinterConfig::Octoprint(_, _) => PrinterType::Octoprint,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterConfigBambu {
    #[serde(default = "PrinterId::empty")]
    pub id: PrinterId,
    pub serial: String,
    pub name: String,
    pub host: String,
    pub access_code: String,
}

impl PrinterConfigBambu {
    pub fn new(serial: String, name: String, host: String, access_code: String) -> Self {
        Self {
            id: PrinterId::generate(),
            serial,
            name,
            host,
            access_code,
        }
    }

    pub fn from_id(
        serial: String,
        name: String,
        host: String,
        access_code: String,
        id: PrinterId,
    ) -> Self {
        Self {
            id,
            serial,
            name,
            host,
            access_code,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterConfigKlipper {
    #[serde(default = "PrinterId::empty")]
    pub id: PrinterId,
    pub name: String,
    pub host: String,
    pub toolchanger: bool,
}

impl PrinterConfigKlipper {
    pub fn new(name: String, host: String) -> Self {
        Self {
            id: PrinterId::generate(),
            name,
            host,
            toolchanger: false,
        }
    }

    pub fn from_id(name: String, host: String, id: PrinterId) -> Self {
        Self {
            id,
            name,
            host,
            toolchanger: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterConfigPrusa {
    #[serde(default = "PrinterId::empty")]
    pub id: PrinterId,
    pub name: String,
    pub host: String,
    pub key: String,
    // pub serial: String,
    // pub fingerprint: String,
    // pub token: String,
    pub octo: Option<PrinterConfigOcto>,
    #[cfg(feature = "rtsp")]
    pub rtsp: Option<crate::streaming::rtsp::RtspCreds>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterConfigOcto {
    // pub id: PrinterId,
    // pub name: String,
    pub host: String,
    pub token: String,
}
