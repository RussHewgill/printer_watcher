use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use serde_json::Value;

/// pick tool:
///     G27 P0 Z5
///     T[tool] S1 L0 D0
/// change filaments:
///     Load PLA in T0:
///         G27 P0 Z40
///         M701 S"PLA" T0 W2
///     Unload T5:
///         G27 P0 Z40
///         M702 T4 W2
///     Unload T0, then load PLA:
///         G27 P0 Z40
///         M1600 S"PLA" T0 R
/// cooldown:
/// stealth:
///     disable: M9140
///     enable: M9150
///     

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
    ChangeFilament([Option<ChangeFilament>; 5]),
    Cooldown,
    SetStealth(bool),
}

impl OctoCmd {
    pub fn unload_filament(tool: usize) -> Self {
        let mut out = [None; 5];
        out[tool] = Some(ChangeFilament::Unload(tool));
        Self::ChangeFilament(out)
    }

    pub fn load_pla(tools: impl IntoIterator<Item = usize>) -> Self {
        let mut out = [None; 5];
        for t in tools.into_iter() {
            if t >= 5 {
                continue;
            }
            out[t] = Some(ChangeFilament::Load(t, FilamentType::PLA));
        }
        Self::ChangeFilament(out)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChangeFilament {
    Unload(usize),
    Load(usize, FilamentType),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilamentType {
    PLA,
    PETG,
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
            OctoCmd::ChangeFilament(changes) => {
                let mut cs = vec![];

                for (i, c) in changes.into_iter().enumerate() {
                    let Some(c) = c else {
                        continue;
                    };

                    match c {
                        ChangeFilament::Unload(tool) => {
                            cs.push(format!("M702 T{} W2", tool));
                        }
                        ChangeFilament::Load(tool, filament) => {
                            let f = match filament {
                                FilamentType::PLA => "PLA",
                                FilamentType::PETG => "PETG",
                            };
                            cs.push(format!("M701 S\"{}\" T{} W2", f, tool));
                        }
                    }
                }

                serde_json::json!({
                    "commands": [
                        cs
                    ]
                })
            }
            _ => todo!(),
        }
    }
}
