use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui::{Label, Layout, Response, RichText, Vec2};

use crate::config::printer_config::PrinterConfigOcto;

use super::{app::App, ui_types::GridLocation};

// impl App {
//     pub fn show_printer_octo(
//         &mut self,
//         ui: &mut egui::Ui,
//         pos: GridLocation,
//         // frame_size: Vec2,
//         printer: &PrinterConfigOcto,
//     ) -> Response {
//         // /// checked at call site
//         // let Some(status) = self.printer_states.get(&printer.id) else {
//         //     warn!("Printer not found: {}", printer.serial);
//         //     panic!();
//         // };

//         unimplemented!()
//     }
// }
