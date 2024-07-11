use anyhow::{anyhow, bail, ensure, Context, Result};
use egui::{Label, Response, RichText, Vec2};
use tracing::{debug, error, info, trace, warn};

use crate::{config::printer_id::PrinterId, status::GenericPrinterState};

use super::{app::App, icons::printer_state_icon, ui_types::GridLocation};

impl App {
    /// MARK: Header
    // #[cfg(feature = "nope")]
    pub fn printer_widget_header(
        &self,
        ui: &mut egui::Ui,
        status: &GenericPrinterState,
        id: PrinterId,
        name: &str,
        pos: GridLocation,
    ) -> Response {
        let icon_size = 24.;

        // let w = ui.available_width();
        // debug!("available_width: {}", w);

        let size = Vec2::new(ui.available_width() - 12., icon_size);
        // let size = Vec2::new(ui.available_size_before_wrap().x, icon_size + 4.);

        let resp = crate::ui::ui_utils::put_ui(ui, size, None, |ui| {
            let resp = ui
                .horizontal(|ui| {
                    // ui.menu_image_button(icon_menu_with_size(icon_size - 4.), |ui| {
                    //     ui.label("Menu");
                    //     // self.printer_menu(ui, status, printer);
                    // });

                    ui.dnd_drag_source(
                        egui::Id::new(format!("{:?}_drag_src_{}_{}", id, pos.col, pos.row)),
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
                                        name,
                                        status.state.to_text()
                                    ))
                                    .strong(),
                                )
                                .truncate(),
                            );
                            ui.allocate_space(Vec2::new(ui.available_width() - icon_size, 0.));
                        },
                    )
                    .response
                })
                .response;

            resp
        });

        resp.context_menu(|ui| {
            ui.label("Context menu");
        });

        #[cfg(feature = "nope")]
        crate::ui::ui_utils::put_ui(ui, size, None, |ui| {
            let layout = Layout::left_to_right(egui::Align::Center)
                .with_cross_justify(true)
                .with_main_justify(true)
                .with_cross_align(egui::Align::Center);

            ui.with_layout(layout, |ui| {
                ui.horizontal(|ui| {
                    ui.menu_image_button(icon_menu_with_size(icon_size - 4.), |ui| {
                        // self.printer_menu(ui, status, printer);
                    });

                    let resp = ui.dnd_drag_source(
                        egui::Id::new(format!("{}_drag_src_{}_{}", printer.id, pos.col, pos.row)),
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
                                .truncate(),
                            );
                            ui.allocate_space(Vec2::new(ui.available_width() - icon_size, 0.));
                        },
                    );

                    resp.response
                })
                .response
            })
            .response
        });

        resp
    }
}
