use egui::{Response, Vec2};

use super::{app::App, ui_types::GridLocation};
use crate::config::printer_config::PrinterConfigBambu;

impl App {
    pub fn show_printer_bambu(
        &mut self,
        ui: &mut egui::Ui,
        pos: GridLocation,
        // frame_size: Vec2,
        printer: &PrinterConfigBambu,
    ) -> Response {
        unimplemented!()
    }
}
