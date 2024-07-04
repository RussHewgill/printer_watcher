use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use serde_json::Value;

/// pick tool:
///     G27 P0 Z5
///     T[tool] S1 L0 D0

pub enum OctoCmd {
    ParkTool,
    PickupTool(usize),
    Jog {
        x: Option<f64>,
        y: Option<f64>,
        z: Option<f64>,
        absolute: bool,
        /// mm/min
        speed: u64,
    },
    Home {
        x: bool,
        y: bool,
        z: bool,
    },
    SetFeedrate(u64),
}

impl OctoCmd {
    pub fn to_json(&self) -> Value {
        match self {
            OctoCmd::ParkTool => serde_json::json!({
                "commands": [
                    "G27 P0 Z5",
                    "T5 S1 L0 D0",
                ],
            }),
            OctoCmd::PickupTool(t) => {
                let c = format!("T{} S1 L0 D0", t);
                serde_json::json!({
                    "commands": [
                        "G27 P0 Z5",
                        c
                    ],
                })
            }
            OctoCmd::Jog {
                x,
                y,
                z,
                absolute,
                speed,
            } => todo!(),
            OctoCmd::Home { x, y, z } => {
                let mut active_axes = Vec::new();
                if *x {
                    active_axes.push("x");
                }
                if *y {
                    active_axes.push("y");
                }
                if *z {
                    active_axes.push("z");
                }
                serde_json::json!({
                    "command": "home",
                    "axes": active_axes,
                })
            }
            OctoCmd::SetFeedrate(f) => todo!(),
        }
    }
}
