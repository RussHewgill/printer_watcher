use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui::{Color32, Label, Layout, Pos2, Response, RichText, Sense, Vec2};

use super::{
    app::App,
    icons::{
        icon_menu_with_size, printer_state_icon, thumbnail_bed, thumbnail_chamber, thumbnail_nozzle,
    },
    ui_types::GridLocation,
};
use crate::{
    config::printer_config::{PrinterConfigBambu, PrinterType},
    status::{bambu_status::AmsStatus, GenericPrinterState},
};

impl App {
    pub fn show_printer_bambu_v2(
        &mut self,
        ui: &mut egui::Ui,
        pos: GridLocation,
        // frame_size: Vec2,
        printer: &PrinterConfigBambu,
    ) -> Response {
        /// checked at call site
        let Some(status) = self.printer_states.get(&printer.id) else {
            warn!("Printer not found: {:?}", printer.id);
            panic!();
        };

        /// Name, state, and controls button
        /// Can't be in strip or response can't get passed up
        let resp = self.printer_widget_header(
            ui,
            &status,
            printer.id.clone(),
            &printer.name,
            pos,
            PrinterType::Bambu,
        );

        let layout = Layout::left_to_right(egui::Align::Center)
            .with_cross_justify(true)
            .with_main_justify(true)
            .with_cross_align(egui::Align::Center);

        let text_size_title = 12.;
        let text_size_eta = 12.;

        let thumbnail_width = crate::ui::PRINTER_WIDGET_SIZE.0 - 24.;
        let thumbnail_height = thumbnail_width * 0.5625;

        drop(status);
        ui.spacing_mut().item_spacing.x = 1.;
        egui_extras::StripBuilder::new(ui)
            .clip(true)
            .cell_layout(layout)
            // thumbnail
            .size(egui_extras::Size::exact(thumbnail_height + 6.))
            // .size(egui_extras::Size::exact(26.))
            // temperatures
            .size(egui_extras::Size::exact(26.))
            .size(egui_extras::Size::exact(26.))
            // Title
            .size(egui_extras::Size::exact(text_size_title + 4.))
            // progress bar
            .size(egui_extras::Size::exact(26.))
            // ETA
            .size(egui_extras::Size::exact(text_size_eta + 2.))
            // AMS
            .size(egui_extras::Size::exact(60. + 2.))
            .vertical(|mut strip| {
                let Some(status) = self.printer_states.get(&printer.id) else {
                    warn!("Printer not found: {:?}", printer.id);
                    panic!();
                };
                /// thumbnail/webcam
                strip.cell(|ui| {
                    //
                    ui.label("TODO: Thumbnail");
                });

                #[cfg(feature = "nope")]
                /// temperatures
                strip.strip(|mut builder| {
                    let font_size = 12.;

                    // let layout = Layout::left_to_right(egui::Align::Center)
                    //     .with_cross_justify(true)
                    //     .with_main_justify(true)
                    //     .with_cross_align(egui::Align::Center);

                    builder
                        .size(egui_extras::Size::relative(0.4))
                        .size(egui_extras::Size::relative(0.4))
                        .size(egui_extras::Size::remainder())
                        .cell_layout(layout)
                        .horizontal(|mut strip| {
                            strip.cell(|ui| {
                                ui.horizontal(|ui| {
                                    // ui.ctx().debug_painter().debug_rect(
                                    //     ui.max_rect(),
                                    //     Color32::RED,
                                    //     "",
                                    // );
                                    ui.add(thumbnail_nozzle(status.nozzle_temp_target > 0.));
                                    ui.add(
                                        Label::new(
                                            // RichText::new(format!("{:.1}°C", status.temp_nozzle.unwrap_or(0.)))
                                            RichText::new(format!(
                                                "{:.1}°C / {}",
                                                status.nozzle_temp,
                                                status.nozzle_temp_target as i64
                                            ))
                                            .strong()
                                            .size(font_size),
                                        )
                                        .truncate(),
                                    );
                                });
                            });
                            strip.cell(|ui| {
                                ui.horizontal(|ui| {
                                    ui.add(thumbnail_bed(status.bed_temp_target > 0.));
                                    ui.add(
                                        Label::new(
                                            RichText::new(format!(
                                                "{:.1}°C / {}",
                                                status.bed_temp, status.bed_temp_target as i64
                                            ))
                                            .strong()
                                            .size(font_size),
                                        )
                                        .truncate(),
                                    );
                                });
                            });
                            strip.cell(|ui| {
                                ui.horizontal(|ui| {
                                    ui.add(thumbnail_chamber());
                                    ui.label(
                                        RichText::new(format!(
                                            "--",
                                            // "{}°C",
                                            // status.temp_chamber.unwrap_or(0.) as i64
                                        ))
                                        .strong()
                                        .size(font_size),
                                    );
                                });
                            });
                        });
                });

                /// temperatures 1: nozzles, bed
                strip.strip(|mut builder| {
                    let font_size = 11.5;

                    let Some(bambu) = &status.state_bambu else {
                        error!("Bambu state not found: {:?}", printer.id);
                        panic!();
                    };

                    let Some(extruder) = bambu.device.extruder.as_ref() else {
                        // error!("Extruder not found: {:?}", printer.id);
                        // panic!();
                        return;
                    };
                    let Some(nozzle_state) = extruder.get_state() else {
                        // error!("Nozzle state not found: {:?}", printer.id);
                        // panic!();
                        return;
                    };

                    builder
                        .size(egui_extras::Size::relative(0.4))
                        .size(egui_extras::Size::relative(0.35))
                        .size(egui_extras::Size::remainder())
                        .cell_layout(layout)
                        .horizontal(|mut strip| {
                            let left = &extruder.info[0];
                            let right = &extruder.info[1];

                            let current_nozzle = match nozzle_state {
                                crate::status::bambu_status::H2DNozzleState::Left => "L",
                                crate::status::bambu_status::H2DNozzleState::Right => "R",
                                crate::status::bambu_status::H2DNozzleState::Other(_) => "_",
                            };

                            strip.cell(|ui| {
                                // ui.ctx().debug_painter().debug_rect(
                                //     ui.max_rect(),
                                //     Color32::GREEN,
                                //     "",
                                // );
                                ui.horizontal(|ui| {
                                    ui.add(thumbnail_nozzle(status.nozzle_temp_target > 0.));
                                    ui.add(
                                        Label::new(
                                            // RichText::new(format!("{:.1}°C", status.temp_nozzle.unwrap_or(0.)))
                                            RichText::new(format!(
                                                "[{}] {:.1}°C/{}",
                                                current_nozzle,
                                                status.nozzle_temp,
                                                status.nozzle_temp_target as i64 // 500.0,
                                                                                 // 500.0,
                                            ))
                                            .strong()
                                            .size(font_size),
                                        )
                                        .truncate(),
                                    );
                                });
                            });

                            strip.cell(|ui| {
                                // ui.ctx().debug_painter().debug_rect(
                                //     ui.max_rect(),
                                //     Color32::RED,
                                //     "",
                                // );
                                ui.horizontal(|ui| {
                                    ui.add(thumbnail_bed(status.bed_temp_target > 0.));
                                    ui.add(
                                        Label::new(
                                            RichText::new(format!(
                                                "{:.1}°C/{}",
                                                status.bed_temp,
                                                status.bed_temp_target as i64 // 500.0,
                                                                              // 500,
                                            ))
                                            .strong()
                                            .size(font_size),
                                        )
                                        .truncate(),
                                    );
                                });
                            });

                            strip.cell(|ui| {
                                // ui.ctx().debug_painter().debug_rect(
                                //     ui.max_rect(),
                                //     Color32::BLUE,
                                //     "",
                                // );
                                ui.horizontal(|ui| {
                                    ui.add(thumbnail_chamber());
                                    ui.label(
                                        RichText::new(format!(
                                            // "--",
                                            "{}°C/{}",
                                            status.chamber_temp as i64,
                                            status.chamber_temp_target.unwrap_or(0.) as i64
                                        ))
                                        .strong()
                                        .size(font_size),
                                    );
                                });
                            });

                            //
                        });
                });

                /// temperatures 2: chamber, fans
                strip.cell(|ui| {
                    ui.label("TODO: Temperatures/Fans 2");
                });

                /// Title
                strip.cell(|ui| {
                    // ui.ctx()
                    //     .debug_painter()
                    //     .debug_rect(ui.max_rect(), Color32::GREEN, "");
                    let layout = Layout::left_to_right(egui::Align::Min)
                        .with_cross_justify(true)
                        .with_main_justify(true)
                        .with_cross_align(egui::Align::Min);

                    ui.with_layout(layout, |ui| {
                        ui.add(
                            Label::new(
                                RichText::new(&format!(
                                    "{}",
                                    status
                                        .current_file
                                        .as_ref()
                                        .map(|s| s.as_str())
                                        .unwrap_or("--"),
                                ))
                                .strong()
                                .size(text_size_title),
                            )
                            .truncate(),
                        );
                    });
                });

                /// progress bar
                strip.cell(|ui| {
                    // ui.ctx()
                    //     .debug_painter()
                    //     .debug_rect(ui.max_rect(), Color32::RED, "");
                    let p = status.progress;
                    ui.add(
                        egui::ProgressBar::new(p as f32 / 100.0)
                            .desired_width(ui.available_width() - 0.)
                            .text(format!("{}%", p)),
                    );
                });

                /// ETA
                /// TODO: layers?
                strip.strip(|mut builder| {
                    let Some(remaining) = status.time_remaining else {
                        return;
                    };

                    // let time = eta.time();
                    // // let dt = time - chrono::Local::now().naive_local().time();
                    // let dt = if eta < chrono::Local::now() {
                    //     chrono::TimeDelta::zero()
                    // } else {
                    //     eta - chrono::Local::now()
                    // };

                    let time_finish = chrono::Local::now() + remaining;

                    builder
                        .size(egui_extras::Size::relative(0.3))
                        .size(egui_extras::Size::remainder())
                        .size(egui_extras::Size::relative(0.3))
                        .horizontal(|mut strip| {
                            strip.cell(|ui| {
                                // ui.ctx().debug_painter().debug_rect(
                                //     ui.max_rect(),
                                //     Color32::GREEN,
                                //     "",
                                // );
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

                                if let Some((layer, total)) = status.layer {
                                    ui.add(Label::new(
                                        RichText::new(&format!("{}/{}", layer, total))
                                            .strong()
                                            .size(text_size_eta),
                                    ));
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

                /// AMS
                strip.cell(|ui| {
                    // ui.label("TODO: AMS");
                    self.show_ams_h2d(ui, printer);
                    // ui.ctx()
                    //     .debug_painter()
                    //     .debug_rect(ui.max_rect(), Color32::RED, "");

                    // ui.label()
                });
            });

        ui.spacing_mut().item_spacing.x = 8.;

        resp
    }

    fn show_ams_h2d(&self, ui: &mut egui::Ui, printer: &PrinterConfigBambu) {
        let Some(status) = self.printer_states.get(&printer.id) else {
            warn!("Printer not found: {}", printer.serial);
            panic!();
        };

        let Some(bambu) = &status.state_bambu else {
            error!("Bambu state not found: {:?}", printer.id);
            return;
        };

        // let Some(ams) = &bambu.ams else {
        //     error!("AMS not found: {:?}", printer.id);
        //     return;
        // };

        let size = 62.;

        paint_ams_h2d(ui, size, bambu);
    }
}

/// pretend that the configuration will always be one (external spool or AMS HT) + 1 AMS
// #[cfg(feature = "nope")]
fn paint_ams_h2d(
    ui: &mut egui::Ui,
    size: f32,
    // size: f32,
    // ams: &AmsStatus,
    bambu: &crate::status::bambu_status::PrinterStateBambu,
) {
    let layout = Layout::left_to_right(egui::Align::Center)
        .with_cross_justify(true)
        .with_main_justify(true)
        .with_cross_align(egui::Align::Center);

    // let x = bambu
    //     .ams
    //     .as_ref()
    //     .and_then(|a| a.units.get(&0).cloned())
    //     .and_then(|u| u.info);
    // debug!("AMS unit: {:?}", x);

    let Some(ams) = bambu.ams.as_ref() else {
        // warn!("AMS not found");
        return;
    };

    let external_left = bambu
        .vir_slot
        .as_ref()
        .and_then(|v| v.get(0))
        .map(|v| &v.tray_color);

    let external_left = bambu
        .vir_slot
        .as_ref()
        .and_then(|v| v.get(1))
        .map(|v| &v.tray_color);

    /// info = 1003 = right AMS 2
    /// info = 1103 = left AMS 2
    ///
    /// 1001 = right AMS 1
    /// 1101 = left AMS 1
    egui_extras::StripBuilder::new(ui)
        .clip(true)
        .cell_layout(layout)
        .sizes(egui_extras::Size::relative(0.5), 2)
        .horizontal(|mut strip| {
            strip.cell(|ui| {
                // ui.label("Left");
                // ui.ctx()
                //     .debug_painter()
                //     .debug_rect(ui.max_rect(), Color32::RED, "");

                let size = Vec2::new(ui.available_width(), size);
                let (response, painter) = ui.allocate_painter(size, Sense::hover());

                // _draw_ams_h2d(&painter, bambu);
            });

            strip.cell(|ui| {
                // ui.label("Right");

                // ui.ctx()
                //     .debug_painter()
                //     .debug_rect(ui.max_rect(), Color32::GREEN, "");
            });
        });
}

fn _draw_ams_h2d(
    painter: &egui::Painter,
    bambu: &crate::status::bambu_status::PrinterStateBambu,
    //
) {
    // unimplemented!()
}
