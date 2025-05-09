use std::sync::atomic::Ordering;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui::{Color32, CornerRadius, Label, Layout, Pos2, Response, RichText, Sense, Vec2};

use super::{
    app::App,
    icons::{
        icon_menu_with_size, printer_state_icon, thumbnail_bed, thumbnail_chamber, thumbnail_nozzle,
    },
    ui_types::GridLocation,
};
use crate::{
    config::printer_config::{PrinterConfigBambu, PrinterType},
    status::{
        bambu_status::{h2d_extruder::ExtruderSwitchState, AmsStatus, BambuPrinterType},
        GenericPrinterState,
    },
};

impl App {
    pub fn show_printer_bambu_v2(
        &mut self,
        ui: &mut egui::Ui,
        pos: GridLocation,
        // frame_size: Vec2,
        printer: &PrinterConfigBambu,
        bambu_type: Option<BambuPrinterType>,
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
            .size(egui_extras::Size::exact(thumbnail_height + 4.))
            // .size(egui_extras::Size::exact(26.))
            // temperatures
            .size(egui_extras::Size::exact(20.))
            .size(egui_extras::Size::exact(20.))
            // Title
            .size(egui_extras::Size::exact(text_size_title + 4.))
            // progress bar
            .size(egui_extras::Size::exact(26.))
            // ETA
            .size(egui_extras::Size::exact(text_size_eta + 2.))
            // AMS
            .size(egui_extras::Size::exact(48.))
            .vertical(|mut strip| {
                let Some(status) = self.printer_states.get(&printer.id) else {
                    warn!("Printer not found: {:?}", printer.id);
                    panic!();
                };
                /// thumbnail/webcam
                if bambu_type == Some(BambuPrinterType::H2D) {
                    #[cfg(not(feature = "gstreamer"))]
                    strip.cell(|ui| {
                        ui.label("Webcam disabled");
                    });

                    #[cfg(feature = "gstreamer")]
                    strip.cell(|ui| {
                        let mut entry = self
                            .webcam_textures
                            .entry(printer.id.clone())
                            .or_insert_with(|| {
                                let image = egui::ColorImage::new(
                                    [1680, 1080],
                                    egui::Color32::from_gray(220),
                                );
                                let texture = ui.ctx().load_texture(
                                    format!("{:?}_texture", printer.id),
                                    image,
                                    Default::default(),
                                );
                                super::ui_types::WebcamTexture::new(texture)
                            });

                        let size = Vec2::new(thumbnail_width, thumbnail_height);

                        if entry.enabled.load(Ordering::SeqCst) {
                            let img = egui::Image::from_texture((entry.texture.id(), size))
                                .fit_to_exact_size(size)
                                .max_size(size)
                                .corner_radius(CornerRadius::same(4))
                                .sense(Sense::click());

                            let img_resp = ui.add(img);

                            if img_resp.hovered() {
                                ui.ctx().request_repaint();
                            }

                            if img_resp.clicked_by(egui::PointerButton::Primary) {
                                // debug!("webcam clicked");
                                self.selected_stream = Some(printer.id.clone());
                            } else if img_resp.clicked_by(egui::PointerButton::Secondary) {
                                self.stream_cmd_tx
                                    .as_ref()
                                    .unwrap()
                                    .send(crate::streaming::StreamCmd::StopStream(
                                        printer.id.clone(),
                                    ))
                                    .unwrap();
                                // entry.enabled = false;
                                entry.enabled.store(false, Ordering::SeqCst);
                            }
                        } else if self.options.auto_start_streams && entry.first_start {
                            self.stream_cmd_tx
                                .as_ref()
                                .unwrap()
                                .send(crate::streaming::StreamCmd::StartRtsp {
                                    ctx: ui.ctx().clone(),
                                    id: printer.id.clone(),
                                    host: printer.host.clone(),
                                    access_code: printer.access_code.clone(),
                                    serial: printer.serial.clone(),
                                    texture: entry.texture.clone(),
                                    enabled: entry.enabled.clone(),
                                })
                                .unwrap();
                            // entry.enabled = true;
                            entry.first_start = false;
                        } else {
                            // debug!("webcam not enabled: {:?}", printer.id);
                            let img = egui::Image::from_texture((entry.texture.id(), size))
                                .fit_to_exact_size(size)
                                .max_size(size)
                                .corner_radius(CornerRadius::same(4))
                                // .bg_fill(Color32::RED)
                                .sense(Sense::click());
                            let img_resp = ui.add(img);
                            super::ui_utils::draw_pause_overlay(ui, &img_resp);

                            if img_resp.clicked_by(egui::PointerButton::Secondary) {
                                debug!("restarting webcam stream: {:?}", printer.id);
                                self.stream_cmd_tx
                                    .as_ref()
                                    .unwrap()
                                    .send(crate::streaming::StreamCmd::StartRtsp {
                                        ctx: ui.ctx().clone(),
                                        id: printer.id.clone(),
                                        host: printer.host.clone(),
                                        access_code: printer.access_code.clone(),
                                        serial: printer.serial.clone(),
                                        texture: entry.texture.clone(),
                                        enabled: entry.enabled.clone(),
                                    })
                                    .unwrap();
                                // entry.enabled = true;
                                entry.enabled.store(true, Ordering::SeqCst);
                            }
                        }

                        //
                    });
                } else {
                    /// thumbnail/webcam
                    strip.cell(|ui| {
                        // ui.label("Webcam: TODO");

                        let mut entry = self
                            .webcam_textures
                            .entry(printer.id.clone())
                            .or_insert_with(|| {
                                let image = egui::ColorImage::new(
                                    [1920, 1080],
                                    egui::Color32::from_gray(220),
                                );
                                let texture = ui.ctx().load_texture(
                                    format!("{:?}_texture", printer.id),
                                    image,
                                    Default::default(),
                                );
                                super::ui_types::WebcamTexture::new(texture)
                            });

                        let size = Vec2::new(thumbnail_width, thumbnail_height);

                        if entry.enabled.load(Ordering::SeqCst) {
                            let img = egui::Image::from_texture((entry.texture.id(), size))
                                .fit_to_exact_size(size)
                                .max_size(size)
                                .corner_radius(CornerRadius::same(4))
                                .sense(Sense::click());

                            let resp = ui.add(img);

                            if resp.clicked_by(egui::PointerButton::Primary) {
                                // debug!("webcam clicked");
                                self.selected_stream = Some(printer.id.clone());
                            }
                        } else if self.options.auto_start_streams {
                            self.stream_cmd_tx
                                .as_ref()
                                .unwrap()
                                .send(crate::streaming::StreamCmd::StartBambuStills {
                                    id: printer.id.clone(),
                                    host: printer.host.clone(),
                                    access_code: printer.access_code.clone(),
                                    serial: printer.serial.clone(),
                                    texture: entry.texture.clone(),
                                })
                                .unwrap();
                            // entry.enabled = true;
                            entry.enabled.store(true, Ordering::SeqCst);
                        } else {
                            if ui.button("Enable webcam").clicked() {
                                self.stream_cmd_tx
                                    .as_ref()
                                    .unwrap()
                                    .send(crate::streaming::StreamCmd::StartBambuStills {
                                        id: printer.id.clone(),
                                        host: printer.host.clone(),
                                        access_code: printer.access_code.clone(),
                                        serial: printer.serial.clone(),
                                        texture: entry.texture.clone(),
                                    })
                                    .unwrap();
                                // entry.enabled = true;
                                entry.enabled.store(true, Ordering::SeqCst);
                            }
                        }
                    });
                }

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

                /// temperatures: nozzles, bed, chamber
                if bambu_type == Some(BambuPrinterType::H2D) {
                    strip.strip(|mut builder| {
                        let font_size = 11.5;

                        let Some(bambu) = &status.state_bambu else {
                            error!("Bambu state not found: {:?}", printer.id);
                            panic!();
                        };

                        let Some(extruder) = bambu.device.extruder.as_ref() else {
                            return;
                        };

                        builder
                            .size(egui_extras::Size::relative(0.4))
                            .size(egui_extras::Size::relative(0.35))
                            .size(egui_extras::Size::remainder())
                            .cell_layout(layout)
                            .horizontal(|mut strip| {
                                let current_nozzle = match extruder.switch_state {
                                    ExtruderSwitchState::Idle => {
                                        match extruder.current_extruder() {
                                            0 => "R",
                                            1 => "L",
                                            _ => "??",
                                        }
                                    }
                                    ExtruderSwitchState::Busy => "B",
                                    ExtruderSwitchState::Switching => "S",
                                    ExtruderSwitchState::Failed => "F",
                                    ExtruderSwitchState::Other(_) => "?",
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
                                                    "[{}] {}°C/{}",
                                                    current_nozzle,
                                                    extruder
                                                        .get_current()
                                                        .map(|e| e.temp)
                                                        .unwrap_or(0),
                                                    extruder
                                                        .get_current()
                                                        .map(|e| e.target_temp)
                                                        .unwrap_or(0),
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
                } else {
                    strip.strip(|mut builder| {
                        let font_size = 11.5;

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
                }

                /// temperatures 2: other nozzle, fans
                // ui.label("TODO: Temperatures/Fans 2");
                if bambu_type == Some(BambuPrinterType::H2D) {
                    strip.strip(|mut builder| {
                        let font_size = 11.5;

                        let Some(bambu) = &status.state_bambu else {
                            error!("Bambu state not found: {:?}", printer.id);
                            panic!();
                        };

                        let Some(extruder) = bambu.device.extruder.as_ref() else {
                            return;
                        };

                        builder
                            .size(egui_extras::Size::relative(0.28))
                            .sizes(egui_extras::Size::relative(0.2), 3)
                            .size(egui_extras::Size::remainder())
                            .cell_layout(layout)
                            .horizontal(|mut strip| {
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
                                                    "{}°C/{}",
                                                    extruder
                                                        .get_other()
                                                        .map(|e| e.temp)
                                                        .unwrap_or(0),
                                                    extruder
                                                        .get_other()
                                                        .map(|e| e.target_temp)
                                                        .unwrap_or(0),
                                                    // 500,
                                                    // 500,
                                                ))
                                                .strong()
                                                .size(font_size),
                                            )
                                            .truncate(),
                                        );
                                    });
                                });

                                let airduct = bambu.device.airduct.as_ref().unwrap();

                                strip.cell(|ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "Part: {:>3}",
                                            // bambu.cooling_fan_speed.unwrap_or_default()
                                            airduct.parts[0].state as i64
                                        ))
                                        .strong()
                                        .size(font_size),
                                    );
                                });

                                strip.cell(|ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "Aux: {:>3}",
                                            // bambu.aux_fan_speed.unwrap_or_default()
                                            airduct.parts[1].state as i64
                                        ))
                                        .strong()
                                        .size(font_size),
                                    );
                                });

                                strip.cell(|ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "Cham: {:>3}",
                                            // bambu.chamber_fan_speed.unwrap_or_default()
                                            airduct.parts[2].state as i64
                                        ))
                                        .strong()
                                        .size(font_size),
                                    );
                                });

                                // parts[3] = heater?

                                #[cfg(feature = "nope")]
                                strip.cell(|ui| {
                                    ui.horizontal(|ui| {
                                        // ui.add(thumbnail_fan());

                                        // debug!("cooling: {:?}", bambu.cooling_fan_speed);
                                        // debug!("big_fan1: {:?}", bambu.aux_fan_speed);
                                        // debug!("big_fan2: {:?}", bambu.chamber_fan_speed);

                                        // let airduct = bambu.device.airduct.as_ref().unwrap();

                                        // ui.label("")

                                        // for (i, fan) in airduct.parts.iter().enumerate() {
                                        //     debug!(
                                        //         "Fan: {} ({}): {}, {}",
                                        //         fan.air_type, fan.id, fan.func, fan.state,
                                        //     );
                                        // }
                                    });
                                });

                                strip.cell(|ui| {
                                    // ui.label("TODO");
                                });
                            });
                    });
                } else {
                    strip.cell(|ui| {
                        ui.label("TODO: Temperatures/Fans 2");
                    });
                }

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

                    let Some(status) = self.printer_states.get(&printer.id) else {
                        warn!("Printer not found: {}", printer.serial);
                        panic!();
                    };

                    let Some(bambu) = &status.state_bambu else {
                        error!("Bambu state not found: {:?}", printer.id);
                        return;
                    };

                    let height = 44.;

                    if bambu_type == Some(BambuPrinterType::H2D) {
                        super::ams::paint_ams_h2d(ui, height, bambu);
                    } else {
                        // self.show_ams(ui, printer);

                        let Some(ams) = &bambu.ams else {
                            error!("AMS not found: {:?}", printer.id);
                            return;
                        };

                        super::widget_bambu::paint_ams(ui, height, ams);
                    }
                    // ui.ctx()
                    //     .debug_painter()
                    //     .debug_rect(ui.max_rect(), Color32::RED, "");

                    // ui.label()
                });
            });

        ui.spacing_mut().item_spacing.x = 8.;

        resp
    }
}
