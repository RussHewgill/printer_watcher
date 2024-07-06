use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::printer_id::PrinterId;

#[derive(Debug, Clone)]
pub enum PrinterConfig {
    // Bambu(PrinterConfigBambu),
    // Klipper(PrinterConfigKlipper),
    Bambu(PrinterId, Arc<RwLock<PrinterConfigBambu>>),
    Klipper(PrinterId, Arc<RwLock<PrinterConfigKlipper>>),
    Prusa(PrinterId, Arc<RwLock<PrinterConfigPrusa>>),
    Octoprint(PrinterId, Arc<RwLock<PrinterConfigOcto>>),
}

/// getters
impl PrinterConfig {
    pub fn id(&self) -> PrinterId {
        match self {
            PrinterConfig::Bambu(id, _) => id.clone(),
            PrinterConfig::Klipper(id, _) => id.clone(),
            PrinterConfig::Prusa(id, _) => id.clone(),
            PrinterConfig::Octoprint(id, _) => id.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrinterConfigBambu {
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

#[derive(Debug, Serialize, Deserialize)]
pub struct PrinterConfigKlipper {
    pub id: PrinterId,
    pub name: String,
    pub host: String,
}

impl PrinterConfigKlipper {
    pub fn new(name: String, host: String) -> Self {
        Self {
            id: PrinterId::generate(),
            name,
            host,
        }
    }

    pub fn from_id(name: String, host: String, id: PrinterId) -> Self {
        Self { id, name, host }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrinterConfigPrusa {
    pub id: PrinterId,
    pub name: String,
    pub host: String,
    pub key: String,
    pub serial: String,
    // pub fingerprint: String,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterConfigOcto {
    pub id: PrinterId,
    pub name: String,
    pub host: String,
    pub token: String,
}
