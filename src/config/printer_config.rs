use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PrinterId(pub Arc<String>);

pub enum PrinterType {
    Bambu,
    Klipper,
    Prusa,
}

#[derive(Debug, Clone)]
pub enum PrinterConfig {
    Bambu(PrinterConfigBambu),
    Klipper(PrinterConfigKlipper),
    // Prusa(PrinterConfigPrusa),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterConfigBambu {
    pub serial: String,
    pub name: String,
    pub host: String,
    pub access_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterConfigKlipper {
    pub host: String,
}
