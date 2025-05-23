use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui::{CornerRadius, Label, Layout, Response, RichText, Sense, Vec2};

use super::{
    app::App,
    icons::*,
    ui_types::{GridLocation, ThumbnailMap},
};
use crate::{
    config::{
        printer_config::{PrinterConfigPrusa, PrinterType},
        printer_id::PrinterId,
    },
    status::GenericPrinterState,
};

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
            warn!("Printer not found: {:?}", printer.id);
            panic!();
        };

        // let size_available = ui.available_size();

        /// Name, state, and controls button
        /// Can't be in strip or response can't get passed up
        let resp = self.printer_widget_header(
            ui,
            &status,
            printer.id.clone(),
            &printer.name,
            pos,
            PrinterType::Prusa,
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

        drop(status);

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
            // Tools (Placeholder)
            .size(egui_extras::Size::exact(text_size_temps + 6.))
            // .size(egui_extras::Size::exact(text_size_temps + 2.))
            .size(egui_extras::Size::exact(text_size_temps + 6.))
            .vertical(|mut strip| {
                /// thumbnail/webcam
                strip.cell(|ui| {
                    self.prusa_preview(ui, &printer, (thumbnail_width, thumbnail_height));
                });

                let Some(status) = self.printer_states.get(&printer.id) else {
                    warn!("Printer not found: {:?}", printer.id);
                    panic!();
                };

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

                /// Tool Temperatures
                strip.strip(|mut builder| {
                    builder
                        .sizes(egui_extras::Size::relative(0.2), 5)
                        .horizontal(|mut strip| {
                            for idx in 0..5 {
                                strip.cell(|ui| {
                                    ui.add(
                                        Label::new(
                                            RichText::new(format!(
                                                "{:.1}°C",
                                                status.nozzle_temps.get(&idx).unwrap_or(&0.),
                                            ))
                                            .strong()
                                            .size(text_size_temps),
                                        )
                                        .truncate(),
                                    );
                                });
                            }
                        });
                });

                /// Tool Target Temperatures
                strip.strip(|mut builder| {
                    builder
                        .sizes(egui_extras::Size::relative(0.2), 5)
                        .horizontal(|mut strip| {
                            for idx in 0..5 {
                                strip.cell(|ui| {
                                    ui.add(
                                        Label::new(
                                            RichText::new(format!(
                                                "{:.0}°C",
                                                status
                                                    .nozzle_temps_target
                                                    .get(&idx)
                                                    .unwrap_or(&0.)
                                                    .round(),
                                            ))
                                            .strong()
                                            .size(text_size_temps),
                                        )
                                        .truncate(),
                                    );
                                });
                            }
                        });
                });

                //
            });

        ui.spacing_mut().item_spacing.x = 8.;

        resp
    }
}

impl App {
    fn prusa_preview(
        &mut self,
        ui: &mut egui::Ui,
        printer: &PrinterConfigPrusa,
        (thumbnail_width, thumbnail_height): (f32, f32),
    ) {
        let Some(status) = self.printer_states.get(&printer.id) else {
            warn!("Printer not found: {:?}", printer.id);
            panic!();
        };

        let preview_type = self
            .preview_setting
            .entry(printer.id.clone())
            .or_insert_with(|| crate::ui::ui_types::PreviewType::Thumbnail);

        match preview_type {
            crate::ui::ui_types::PreviewType::None => {}
            crate::ui::ui_types::PreviewType::Thumbnail => {
                let Some(thumbnail_path) = status
                    .state_prusa
                    .as_ref()
                    .and_then(|s| Some(s.job.file.refs.thumbnail.as_str()))
                else {
                    // warn!("No thumbnail path found for printer: {:?}", printer.id);
                    return;
                };

                match self.thumbnails.get(&printer.id) {
                    Some((file, img)) => {
                        // warn!("thumbnail found: {:?}", file);
                        if file != thumbnail_path {
                            // debug!("bad thumbnail");
                            self.thumbnails.remove(&printer.id);
                            self.thumbnails.set_in_progress(printer.id.clone(), false);
                        } else {
                            // ui.label("Thumbnail");
                            let img =
                                egui::Image::from_bytes(format!("bytes://{}", file), img.clone())
                                    .fit_to_exact_size(Vec2::new(thumbnail_width, thumbnail_height))
                                    .sense(Sense::click());

                            let resp = ui.add(img);
                            if resp.clicked_by(egui::PointerButton::Primary) {
                                self.selected_stream = Some(printer.id.clone());
                            } else if resp.clicked_by(egui::PointerButton::Secondary) {
                                preview_type.toggle_type();
                            }
                        }
                    }
                    None => {
                        if !self.thumbnails.is_in_progress(&printer.id) {
                            debug!("sending thumbnail fetch request");
                            self.thumbnails.set_in_progress(printer.id.clone(), true);
                            self.cmd_tx
                                .as_ref()
                                .unwrap()
                                .send(crate::conn_manager::PrinterConnCmd::FetchThumbnail(
                                    printer.id.clone(),
                                    thumbnail_path.to_string(),
                                ))
                                .unwrap();
                        }
                    }
                }
            }
            crate::ui::ui_types::PreviewType::Webcam => {
                let mut entry = self
                    .webcam_textures
                    .entry(printer.id.clone())
                    .or_insert_with(|| {
                        let image =
                            egui::ColorImage::new([1920, 1080], egui::Color32::from_gray(220));
                        let texture = ui.ctx().load_texture(
                            format!("{:?}_rtsp_texture", printer.id),
                            image,
                            Default::default(),
                        );
                        super::ui_types::WebcamTexture::new(texture)
                    });

                let size = Vec2::new(thumbnail_width, thumbnail_height);

                let mut start = false;
                if entry.enabled.load(std::sync::atomic::Ordering::SeqCst) {
                    // debug!("rtsp webcam enabled");
                    let img = egui::Image::from_texture((entry.texture.id(), size))
                        .fit_to_exact_size(size)
                        .max_size(size)
                        .corner_radius(CornerRadius::same(4))
                        .sense(Sense::click());

                    let resp = ui.add(img);

                    if resp.clicked_by(egui::PointerButton::Primary) {
                        // debug!("webcam clicked");
                        self.selected_stream = Some(printer.id.clone());
                    } else if resp.clicked_by(egui::PointerButton::Secondary) {
                        #[cfg(feature = "rtsp")]
                        preview_type.toggle_type();
                        #[cfg(feature = "rtsp")]
                        self.send_stream_cmd(crate::streaming::StreamCmd::StopStream(
                            printer.id.clone(),
                        ))
                        .unwrap();
                    } else if resp.clicked_by(egui::PointerButton::Middle) {
                        #[cfg(feature = "rtsp")]
                        debug!("toggle skip frames");
                        #[cfg(feature = "rtsp")]
                        self.send_stream_cmd(crate::streaming::StreamCmd::SendRtspCommand(
                            printer.id.clone(),
                            crate::streaming::SubStreamCmd::Rtsp(
                                crate::streaming::rtsp::RtspCommand::ToggleSkipFrames,
                            ),
                        ))
                        .unwrap();
                    }
                } else if self.options.auto_start_streams {
                    // start = true;
                } else {
                    if ui.button("Enable webcam").clicked() {
                        // start = true;
                    }
                }

                #[cfg(feature = "rtsp")]
                if start {
                    if let Some(creds) = printer.rtsp.as_ref() {
                        self.stream_cmd_tx
                            .as_ref()
                            .unwrap()
                            .send(crate::streaming::StreamCmd::StartRtsp(
                                printer.id.clone(),
                                entry.texture.clone(),
                                creds.clone(),
                                ui.ctx().clone(),
                            ))
                            .unwrap();
                        entry.enabled = true;
                    }
                }
            }
        }
    }
}
