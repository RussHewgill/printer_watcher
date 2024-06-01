pub mod conn_bambu;
pub mod conn_klipper;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

/// messages from PrinterConnManager to UI
#[derive(Debug)]
pub enum PrinterConnMsg {
    //
}

/// messages from UI to PrinterConnManager
#[derive(Debug)]
pub enum PrinterConnCmd {
    //
}

pub struct PrinterConnManager {
    //
}
