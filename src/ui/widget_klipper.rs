use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui::{Label, Layout, Response, RichText, Vec2};

use crate::{config::printer_config::PrinterConfigKlipper, status::GenericPrinterState};

use super::{app::App, ui_types::GridLocation};

impl App {
    pub fn show_printer_klipper(
        &mut self,
        ui: &mut egui::Ui,
        pos: GridLocation,
        // frame_size: Vec2,
        printer: &PrinterConfigKlipper,
    ) -> Response {
        /// checked at call site
        let Some(status) = self.printer_states.get(&printer.id) else {
            warn!("Printer not found: {:?}", printer.id);
            panic!();
        };

        /// Name, state, and controls button
        /// Can't be in strip or response can't get passed up
        let resp = self.printer_widget_header(ui, &status, printer.id.clone(), &printer.name, pos);

        unimplemented!()
    }
}
