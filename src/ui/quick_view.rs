use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui::{Color32, Pos2, Response, Sense, Stroke, UiBuilder, Vec2};

use crate::{
    config::printer_id::PrinterId,
    status::PrinterState,
    ui::{
        app::App,
        ui_types::{GridLocation, StatusColors},
    },
};

impl App {
    pub fn show_quick_view(&mut self, ui: &mut egui::Ui) {
        for y in 0..self.options.dashboard_size.1 {
            for x in 0..self.options.dashboard_size.0 {
                let pos = GridLocation { col: x, row: y };

                let id = if let Some(id) = self.printer_order.get(&pos) {
                    id.clone()
                } else {
                    continue;
                };

                let Some(printer) = self.config.get_printer(&id) else {
                    // warn!("Printer not found: {:?}", id);
                    // return (None, None);
                    // unimplemented!()
                    continue;
                };

                let color = if let Some(status) = self.printer_states.get(&id) {
                    match &status.state {
                        // PrinterState::Paused => Color32::from_rgb(173, 125, 90),
                        // PrinterState::Printing => Color32::from_rgb(121, 173, 116),
                        // PrinterState::Error(_) => Color32::from_rgb(173, 125, 90),
                        // PrinterState::Idle | PrinterState::Finished => Color32::from_rgb(158, 44, 150),
                        // PrinterState::Busy => Color32::from_rgb(73, 84, 218),
                        // // PrinterState::Disconnected => Color32::from_rgb(191, 0, 5),
                        // PrinterState::Disconnected => Color32::from_rgb(0, 0, 0),
                        // // _ => Color32::from_gray(127),
                        // // _ => Color32::GREEN,
                        // PrinterState::Unknown(_) => Color32::YELLOW,
                        PrinterState::Paused => StatusColors::PAUSED,
                        PrinterState::Printing => StatusColors::PRINTING,
                        PrinterState::Error(_) => StatusColors::ERROR,
                        PrinterState::Idle | PrinterState::Finished => StatusColors::IDLE,
                        PrinterState::Busy => StatusColors::BUSY,
                        PrinterState::Disconnected => StatusColors::DISCONNECTED,
                        PrinterState::Unknown(_) => StatusColors::UNKNOWN,
                    }
                } else {
                    // debug!("no state");
                    // Color32::from_gray(127)
                    Color32::RED
                };

                let width = 18.;
                let size = Vec2::new(width, width);

                let (response, painter) = ui.allocate_painter(size, Sense::hover());
                painter.circle_filled(response.rect.center(), width / 2. - 2., color);

                // let name = if let Some(printer) = self.config.get_printer(&id) {
                //     printer.name_blocking()
                // } else {
                //     format!("Unknown Printer {:?}", id)
                // };

                // response.on_hover_text(format!("{}: {}", name, status.unwrap_or("Unknown Status")))
                response.on_hover_text(format!("{}", printer.name_blocking()));
            }
        }
    }

    fn show_quick_view_printer(
        &self,
        ui: &mut egui::Ui,
        id: &PrinterId,
        color: &Color32,
        //
    ) -> Response {
        let width = 18.;
        let size = Vec2::new(width, width);

        let (color, status) = if let Some(status) = self.printer_states.get(&id) {
            let color = match status.state {
                PrinterState::Paused => StatusColors::PAUSED,
                PrinterState::Printing => StatusColors::PRINTING,
                PrinterState::Error(_) => StatusColors::ERROR,
                PrinterState::Idle | PrinterState::Finished => StatusColors::IDLE,
                PrinterState::Busy => StatusColors::BUSY,
                PrinterState::Disconnected => StatusColors::DISCONNECTED,
                PrinterState::Unknown(_) => StatusColors::UNKNOWN,
            };
            (color, Some(status.state.to_text()))
        } else {
            (Color32::from_rgba_unmultiplied(127, 127, 127, 64), None)
        };

        let (response, painter) = ui.allocate_painter(size, Sense::hover());
        painter.circle_filled(response.rect.center(), width / 2. - 2., Color32::GREEN);

        let name = if let Some(printer) = self.config.get_printer(id) {
            printer.name_blocking()
        } else {
            format!("Unknown Printer {:?}", id)
        };

        response.on_hover_text(format!("{}: {}", name, status.unwrap_or("Unknown Status")))
    }
}
