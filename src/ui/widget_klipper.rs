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

        let layout = Layout::left_to_right(egui::Align::Center)
            .with_cross_justify(true)
            .with_main_justify(true)
            .with_cross_align(egui::Align::Center);

        let text_size_title = 12.;
        let text_size_eta = 11.;
        let text_size_temps = 12.;

        let thumbnail_width = crate::ui::PRINTER_WIDGET_SIZE.0 - 24.;
        let thumbnail_height = thumbnail_width * 0.5625;

        ui.spacing_mut().item_spacing.x = 1.;
        // #[cfg(feature = "nope")]
        egui_extras::StripBuilder::new(ui)
            .clip(true)
            .cell_layout(layout)
            // thumbnail
            .size(egui_extras::Size::exact(thumbnail_height + 6.))
            // temperatures
            .size(egui_extras::Size::exact(26.))
            // Title
            .size(egui_extras::Size::exact(text_size_title + 4.))
            // progress bar
            .size(egui_extras::Size::exact(26.))
            // ETA
            .size(egui_extras::Size::exact(text_size_eta + 2.))
            // AMS
            .size(egui_extras::Size::exact(text_size_temps + 2.))
            .size(egui_extras::Size::exact(text_size_temps + 2.))
            // .size(egui_extras::Size::initial(10.))
            .vertical(|mut strip| {
                let Some(status) = self.printer_states.get(&printer.id) else {
                    warn!("Printer not found: {:?}", printer.id);
                    panic!();
                };

                /// thumbnail/webcam
                strip.cell(|ui| {
                    ui.label("Thumbnail");
                });

                /// temperatures
                strip.cell(|ui| {
                    ui.label("temps");
                });

                /// Title
                strip.cell(|ui| {
                    ui.label("Current Print");
                });

                /// progress bar
                strip.cell(|ui| {
                    ui.label("Progress Bar");
                });

                /// ETA
                /// TODO: layers?
                strip.strip(|mut builder| {
                    let Some(remaining) = status.time_remaining else {
                        return;
                    };

                    let time_finish = chrono::Local::now() + remaining;

                    builder
                        .size(egui_extras::Size::relative(0.3))
                        .size(egui_extras::Size::remainder())
                        .size(egui_extras::Size::relative(0.3))
                        .horizontal(|mut strip| {
                            strip.cell(|ui| {
                                ui.add(Label::new(
                                    RichText::new(&time_finish.format("%-I:%M %p").to_string())
                                        .strong()
                                        // .text_style(Text)
                                        .size(text_size_eta),
                                ));
                            });

                            strip.cell(|ui| {
                                /// TODO: status instead of layers during prepare
                                #[cfg(feature = "nope")]
                                if let Some(stage) = status.stage {
                                    let state =
                                        crate::status::PrintStage::new(status.layer_num, stage);

                                    let idle = matches!(status.state, PrinterState::Idle)
                                        || matches!(status.state, PrinterState::Finished);
                                    if !idle
                                        && !matches!(state, crate::status::PrintStage::Printing)
                                    {
                                        ui.add(Label::new(
                                            RichText::new(state.to_string())
                                                .size(text_size_eta - 2.),
                                        ));

                                        return;
                                    }
                                }

                                #[cfg(feature = "nope")]
                                if let (Some(layer), Some(max)) =
                                    (status.layer_num, status.total_layer_num)
                                {
                                    ui.add(Label::new(
                                        RichText::new(&format!("{}/{}", layer, max))
                                            .strong()
                                            .size(text_size_eta),
                                    ));
                                }
                            });

                            strip.cell(|ui| {
                                ui.add(Label::new(
                                    RichText::new(&format!(
                                        "-{:02}:{:02}",
                                        remaining.num_hours(),
                                        remaining.num_minutes() % 60
                                    ))
                                    .strong()
                                    .size(text_size_eta),
                                ));
                            });
                        });
                });

                //
            });

        resp
    }
}
