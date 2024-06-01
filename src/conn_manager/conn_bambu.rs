// pub mod bambu_proto;
mod command;
pub mod message;
mod parse;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

pub struct BambuConn {}
