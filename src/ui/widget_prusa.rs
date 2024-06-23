use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui::{Label, Layout, Response, RichText, Vec2};

use super::{app::App, icons::*, ui_types::GridLocation};
use crate::{config::printer_config::PrinterConfigPrusa, status::GenericPrinterState};

impl App {
    pub fn show_printer_prusa(
        &mut self,
        ui: &mut egui::Ui,
        pos: GridLocation,
        // frame_size: Vec2,
        printer: &PrinterConfigPrusa,
    ) -> Response {
        /// checked at call site
        let Some(status) = self.printer_states.get(&printer.id) else {
            warn!("Printer not found: {}", printer.serial);
            panic!();
        };

        /// Name, state, and controls button
        /// Can't be in strip or response can't get passed up
        let resp = ui
            .horizontal(|ui| {
                // let selected = self
                //     .selected_printer_controls
                //     .as_ref()
                //     .map(|s| s == &printer.serial)
                //     .unwrap_or(false);

                let resp = self.prusa_printer_header(ui, &status, &printer, pos);

                resp
            })
            .response;

        // unimplemented!()
        // ui.label("Prusa")

        resp
    }
}

impl App {
    /// MARK: Header
    fn prusa_printer_header(
        &self,
        ui: &mut egui::Ui,
        status: &GenericPrinterState,
        printer: &PrinterConfigPrusa,
        pos: GridLocation,
    ) -> Response {
        let icon_size = 24.;

        let size = Vec2::new(ui.available_width() - 12., icon_size);
        // let size = Vec2::new(ui.available_size_before_wrap().x, icon_size + 4.);

        crate::ui::ui_utils::put_ui(ui, size, |ui| {
            let layout = Layout::left_to_right(egui::Align::Center)
                .with_cross_justify(true)
                .with_main_justify(true)
                .with_cross_align(egui::Align::Center);

            ui.with_layout(layout, |ui| {
                ui.horizontal(|ui| {
                    let resp = ui.dnd_drag_source(
                        egui::Id::new(format!(
                            "{}_drag_src_{}_{}",
                            printer.serial, pos.col, pos.row
                        )),
                        GridLocation {
                            col: pos.col,
                            row: pos.row,
                        },
                        |ui| {
                            printer_state_icon(ui, icon_size, &status.state);
                            ui.add(
                                Label::new(
                                    RichText::new(&format!(
                                        "{} ({})",
                                        printer.name,
                                        status.state.to_text()
                                    ))
                                    .strong(),
                                )
                                .truncate(true),
                            );
                            ui.allocate_space(Vec2::new(ui.available_width() - icon_size, 0.));
                        },
                    );
                    ui.menu_image_button(icon_menu_with_size(icon_size - 4.), |ui| {
                        // self.printer_menu(ui, status, printer);
                    });

                    resp.response
                })
                .response
            })
            .response
        })
    }
}
