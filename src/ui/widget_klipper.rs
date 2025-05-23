use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui::{Color32, Label, Layout, Response, RichText, Vec2};

use crate::{
    config::printer_config::{PrinterConfigKlipper, PrinterType},
    status::GenericPrinterState,
};

use super::{
    app::App,
    icons::{thumbnail_bed, thumbnail_chamber, thumbnail_nozzle},
    ui_types::GridLocation,
};

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
        let resp = self.printer_widget_header(
            ui,
            &status,
            printer.id.clone(),
            &printer.name,
            pos,
            PrinterType::Klipper,
        );

        let layout = Layout::left_to_right(egui::Align::Center)
            .with_cross_justify(true)
            .with_main_justify(true)
            .with_cross_align(egui::Align::Center);

        let text_size_title = 12.;
        let text_size_eta = 12.;
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
                strip.strip(|mut builder| {
                    let font_size = 12.;

                    if printer.toolchanger {
                        self.klipper_temperatures_toolchanger(&status, layout, font_size, builder);
                    } else {
                        self.klipper_temperatures(&status, layout, font_size, builder);
                    }

                    // let layout = Layout::left_to_right(egui::Align::Center)
                    //     .with_cross_justify(true)
                    //     .with_main_justify(true)
                    //     .with_cross_align(egui::Align::Center);
                });

                /// bed temp, fans
                strip.strip(|mut builder| {
                    self.klipper_temperatures_row2(&status, layout, builder);
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

                            /// Prusa doesn't tell what layer it's on?
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

                /// ETA
                /// TODO: layers?
                #[cfg(feature = "nope")]
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

impl App {
    fn klipper_temperatures_row2(
        &self,

        status: &GenericPrinterState,
        layout: Layout,
        builder: egui_extras::StripBuilder<'_>,
    ) {
        builder
            // .sizes(egui_extras::Size::relative(1. / n as f32), n)
            .size(egui_extras::Size::exact(22.))
            .size(egui_extras::Size::exact(80.))
            // .sizes(egui_extras::Size::exact(55.), n)
            .horizontal(|mut strip| {
                strip.cell(|ui| {
                    ui.add(thumbnail_bed(status.bed_temp_target > 0.));
                });
                strip.cell(|ui| {
                    let text = RichText::new(format!(
                        "{:.1}°C/{:.0}",
                        status.bed_temp, status.bed_temp_target
                    ))
                    .monospace()
                    .size(11.);

                    ui.label(text);
                });
            });
    }

    fn klipper_temperatures_toolchanger(
        &self,
        status: &GenericPrinterState,
        layout: Layout,
        _font_size: f32,
        builder: egui_extras::StripBuilder<'_>,
    ) {
        let n = status.nozzle_temps.len();
        // debug!("Nozzles: {}", n);
        builder
            // .sizes(egui_extras::Size::relative(1. / n as f32), n)
            .size(egui_extras::Size::exact(22.))
            .sizes(egui_extras::Size::exact(55.), n)
            .horizontal(|mut strip| {
                strip.cell(|ui| {
                    ui.add(thumbnail_nozzle(status.nozzle_temp_target > 0.));
                });

                for i in 0..n {
                    strip.cell(|ui| {
                        let color = if Some(i) == status.current_tool {
                            ui.style().visuals.selection.bg_fill
                        } else {
                            // ui.style().visuals.selection.bg_fill
                            ui.style().visuals.widgets.noninteractive.bg_stroke.color
                        };

                        egui::Frame::group(ui.style())
                            .inner_margin(0.)
                            .outer_margin(0.)
                            // .stroke((1., ui.style().visuals.widgets.noninteractive.bg_stroke.color))
                            .stroke((2.0, color))
                            .show(ui, |ui| {
                                let t = status.nozzle_temps.get(&i).unwrap_or(&0.);
                                // debug!("Nozzle {}: {}", i, t);

                                let text =
                                    RichText::new(format!("{:.1}°C", t)).monospace().size(11.);

                                let text = if let Some(target) = status.nozzle_temps_target.get(&i)
                                {
                                    text.strong().color(Color32::from_rgb(251, 149, 20))
                                } else {
                                    text
                                };

                                ui.label(
                                    // RichText::new(format!("{}:{:.1}°C", i, t))
                                    text,
                                );
                                // ui.label(format!("{}", i + 1));
                            });
                    });
                }
            });
    }

    fn klipper_temperatures(
        &self,
        status: &GenericPrinterState,
        layout: Layout,
        font_size: f32,
        builder: egui_extras::StripBuilder<'_>,
    ) {
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
                                    status.nozzle_temp, status.nozzle_temp_target as i64
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
    }
}
