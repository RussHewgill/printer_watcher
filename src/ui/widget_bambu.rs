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
        bambu_status::{AmsCurrentSlot, AmsSlot, AmsStatus},
        GenericPrinterState,
    },
};

impl App {
    pub fn show_printer_bambu(
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
            // temperatures
            .size(egui_extras::Size::exact(26.))
            // Title
            .size(egui_extras::Size::exact(text_size_title + 4.))
            // progress bar
            .size(egui_extras::Size::exact(26.))
            // ETA
            .size(egui_extras::Size::exact(text_size_eta + 2.))
            // AMS
            .size(egui_extras::Size::exact(60. + 2.))
            // .size(egui_extras::Size::initial(10.))
            .vertical(|mut strip| {
                let Some(status) = self.printer_states.get(&printer.id) else {
                    warn!("Printer not found: {:?}", printer.id);
                    panic!();
                };

                /// thumbnail/webcam
                strip.cell(|ui| {
                    // ui.label("Webcam: TODO");

                    let mut entry = self
                        .webcam_textures
                        .entry(printer.id.clone())
                        .or_insert_with(|| {
                            let image =
                                egui::ColorImage::new([1920, 1080], egui::Color32::from_gray(220));
                            let texture = ui.ctx().load_texture(
                                format!("{:?}_texture", printer.id),
                                image,
                                Default::default(),
                            );
                            super::ui_types::WebcamTexture::new(texture)
                        });

                    let size = Vec2::new(thumbnail_width, thumbnail_height);

                    if entry.enabled {
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
                        entry.enabled = true;
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
                            entry.enabled = true;
                        }
                    }

                    //
                });

                /// thumbnail/webcam
                #[cfg(feature = "nope")]
                strip.cell(|ui| {
                    // ui.ctx()
                    //     .debug_painter()
                    //     .debug_rect(ui.max_rect(), Color32::GREEN, "");
                    let layout = Layout::left_to_right(egui::Align::Center)
                        // .with_cross_justify(true)
                        .with_main_justify(true)
                        .with_cross_align(egui::Align::Center);

                    ui.with_layout(layout, |ui| {
                        // debug!("width = {}, height = {}", thumbnail_width, thumbnail_height);

                        let mut use_webcam = false;
                        if let Some(entry) = self.printer_textures.get(&printer.id) {
                            // debug!("got printer texture");
                            if entry.enabled {
                                // debug!("webcam image enabled");
                                let handle = entry.handle.clone();
                                use_webcam = true;
                                /// webcam
                                let size = Vec2::new(thumbnail_width, thumbnail_height);
                                let img = egui::Image::from_texture((handle.id(), size))
                                    .fit_to_exact_size(size)
                                    .max_size(size)
                                    .rounding(Rounding::same(4.))
                                    .sense(Sense::click());
                                let resp = ui.add(img);
                                if resp.clicked_by(egui::PointerButton::Primary) {
                                    // debug!("webcam clicked");
                                    self.selected_stream = Some(printer.id.clone());
                                } else if resp.clicked_by(egui::PointerButton::Secondary) {
                                    self.stream_cmd_tx
                                        .as_ref()
                                        .unwrap()
                                        .send(crate::cloud::streaming::StreamCmd::ToggleStream(
                                            printer.id.clone(),
                                        ))
                                        .unwrap();
                                }
                            }
                        }

                        if !use_webcam {
                            if let Some(url) = status.current_task_thumbnail_url.as_ref() {
                                /// current print job thumbnail
                                let img = egui::Image::new(url)
                                    .bg_fill(if ui.visuals().dark_mode {
                                        Color32::from_gray(128)
                                    } else {
                                        Color32::from_gray(210)
                                    })
                                    .max_width(thumbnail_width)
                                    .rounding(Rounding::same(4.))
                                    .sense(Sense::click());

                                if ui.add(img).clicked_by(egui::PointerButton::Secondary) {
                                    self.stream_cmd_tx
                                        .as_ref()
                                        .unwrap()
                                        .send(crate::cloud::streaming::StreamCmd::ToggleStream(
                                            printer.id.clone(),
                                        ))
                                        .unwrap();
                                }
                            } else if let Some(t) = status.printer_type {
                                /// printer icon
                                let resp = ui.add(
                                    thumbnail_printer(&printer, &t, ui.ctx())
                                        .fit_to_exact_size(Vec2::new(
                                            thumbnail_width,
                                            thumbnail_height,
                                        ))
                                        .rounding(Rounding::same(4.))
                                        .sense(Sense::click()),
                                );
                                if resp.clicked_by(egui::PointerButton::Secondary) {
                                    self.stream_cmd_tx
                                        .as_ref()
                                        .unwrap()
                                        .send(crate::cloud::streaming::StreamCmd::ToggleStream(
                                            printer.id.clone(),
                                        ))
                                        .unwrap();
                                }
                            }
                        }
                    });
                });

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
                    self.show_ams(ui, printer);
                    // ui.ctx()
                    //     .debug_painter()
                    //     .debug_rect(ui.max_rect(), Color32::RED, "");

                    // ui.label()
                });

                //
            });
        ui.spacing_mut().item_spacing.x = 8.;

        let Some(status) = self.printer_states.get(&printer.id) else {
            warn!("Printer not found: {:?}", printer.id);
            panic!();
        };

        let ams_status = status.state_bambu.as_ref().unwrap().ams_status.as_ref();
        ui.label(format!("ams_status: {:?}", ams_status));

        // let ams = status.state_bambu.as_ref().unwrap().ams.as_ref().unwrap();

        // debug!("ams: {:#?}", ams);

        // let state = &status.state;
        // let bstate = status.state_bambu.as_ref().unwrap();
        // ui.label(format!("State: {:?}, {:?}", state, bstate.state));
        // // ui.label(format!("Bambu State: {:?}", bstate.state));

        resp
    }

    fn show_ams(
        &self,
        ui: &mut egui::Ui,
        printer: &PrinterConfigBambu,
        // printer: &PrinterConfigBambu,
    ) {
        let Some(status) = self.printer_states.get(&printer.id) else {
            warn!("Printer not found: {}", printer.serial);
            panic!();
        };

        let Some(bambu) = &status.state_bambu else {
            error!("Bambu state not found: {:?}", printer.id);
            return;
        };

        let Some(ams) = &bambu.ams else {
            error!("AMS not found: {:?}", printer.id);
            return;
        };

        let size = 62.;

        // ui.label(format!("Humidity: {:?}", ams.humidity));

        // crate::ui::icons::paint_ams(ui, size, ams);
        paint_ams(ui, size, ams);

        //
    }
}

/// Circle for each slot
/// Line going down for active slot
fn paint_ams(
    ui: &mut egui::Ui,
    size: f32,
    // size: f32,
    ams: &AmsStatus,
) {
    #[cfg(feature = "nope")]
    ui.vertical(|ui| {
        ui.label(&format!("current tray: {:?}", ams.current_tray));
        ui.label(&format!("tray_now: {:?}", ams.tray_now));
        ui.label(&format!("tray_pre: {:?}", ams.tray_pre));
        ui.label(&format!("tray_tar: {:?}", ams.tray_tar));

        ui.label(&format!("state: {:?}", ams.state));
    });

    let num_units = ams.units.len();

    // debug!("size = {:#?}", size);

    let size = Vec2::new(ui.available_width(), size);
    let (response, painter) = ui.allocate_painter(size, Sense::hover());

    let rect = response.rect;
    let c = rect.center();
    // let r = rect.width() / 2.0 - 1.0;
    // let r = size.x / 2.0 - 1.0;

    // /// 234 x 62
    // // debug!("rect: {:#?}", rect);
    // debug!("rect.width(): {:#?}", rect.width());
    // debug!("rect.height(): {:#?}", rect.height());

    // let mut rect2 = rect;
    // rect2.set_width(rect.width() - 0.);
    // rect2.set_height(rect.height() - 0.0);
    // rect2.set_center(c);
    // painter.rect_stroke(
    //     rect2,
    //     2.,
    //     egui::Stroke::new(3.0, Color32::from_rgba_premultiplied(255, 0, 0, 64)),
    // );

    let p0 = rect.left_top();

    let y = 18.;

    let c = rect.center_top() + Vec2::new(0., y);

    let small_circle_r = 12.;
    let small_spacing = 5.;
    let small_center_spacing = 4.;

    let circle_stroke = 2.;
    let circle_stroke_color = Color32::from_gray(120);
    let y2_height = small_circle_r * 2. + circle_stroke * 2. + 2.;

    if num_units == 0 {
        error!("No units found in ams status");
        return;
    } else if num_units == 1 {
        // let unit = &ams.units[&0];
        // let Some(unit) = ams.units.get(&0) else {
        //     error!("Unit 0 not found in ams status");
        //     for (k, v) in ams.units.iter() {
        //         debug!("unit: {}", k);
        //     }
        //     return;
        // };

        let Some((k, unit)) = ams.units.iter().next() else {
            error!("No AMS units found?");
            return;
        };

        let edge_padding = rect.width() / 8.0;

        let circle_r = 14.;
        let spacing = (rect.width() - edge_padding * 2.) / 3.0;

        for slot_idx in 0..4 {
            let x = slot_idx as f32 * spacing + edge_padding;
            let c = p0 + Vec2::new(x, y);
            // debug!("c: {:#?}", c);

            match &unit.slots[slot_idx] {
                Some(slot) => {
                    painter.circle(
                        c,
                        circle_r,
                        slot.color,
                        egui::Stroke::new(2., circle_stroke_color),
                    );

                    if let Some(AmsCurrentSlot::Tray { ams_id, tray_id }) = ams.current_tray {
                        if ams_id == 0 && slot_idx as u64 == tray_id {
                            draw_ams_current(&painter, circle_r, circle_stroke, c, slot);
                        }
                    }
                }
                None => {
                    painter.circle_stroke(
                        c,
                        circle_r,
                        egui::Stroke::new(circle_stroke, circle_stroke_color),
                    );
                }
            }
            // let color = unit.slots[slot].as_ref
            // painter.circle_filled(c, r, Color32::RED);
        }
    } else if num_units >= 2 {
        let y1 = c.y;
        let y2 = c.y + y2_height;
        for unit in 0..4 {
            if ams.units.get(&unit).is_none() {
                continue;
            }
            let y = if unit < 2 { y1 } else { y2 };

            let d = if unit % 2 == 0 { -1. } else { 1. };
            for slot_idx in 0..4 {
                let x = c.x
                    + (small_center_spacing + small_circle_r) * d
                    + (small_spacing + small_circle_r * 2.) * slot_idx as f32 * d;

                let c = Pos2::new(x, y);

                match &ams.units[&unit].slots[slot_idx] {
                    Some(slot) => {
                        // painter.circle_filled(c, circle_r, slot.color);
                        painter.circle(
                            c,
                            small_circle_r,
                            slot.color,
                            egui::Stroke::new(circle_stroke, circle_stroke_color),
                        );

                        if let Some(AmsCurrentSlot::Tray { ams_id, tray_id }) = ams.current_tray {
                            if ams_id == unit as u64 && slot_idx as u64 == tray_id {
                                draw_ams_current(&painter, small_circle_r, circle_stroke, c, slot);
                            }
                        }
                    }
                    None => {
                        painter.circle_stroke(
                            c,
                            small_circle_r,
                            egui::Stroke::new(circle_stroke, circle_stroke_color),
                        );
                    }
                }

                //
            }
        }

        let c0 = rect.center_top();
        let c1 = rect.center_top() + Vec2::new(0., small_circle_r * 2. + 2.);
        painter.line_segment([c0, c1], egui::Stroke::new(1.0, Color32::from_gray(180)));

        if num_units > 2 {
            // let top = y2 - small_circle_r;
            // let c0 = rect.center_top() + Vec2::new(0., top);
            // let c1 = rect.center_top() + Vec2::new(0., top + small_circle_r * 1. + 2.);

            // let top = Pos2::new(c.x, c.y + y2_height);

            // painter.circle_filled(top, 10., Color32::RED);

            let c0 = Pos2::new(c.x, c.y + y2_height - small_circle_r - 2.);
            let c1 = Pos2::new(c.x, c.y + y2_height + small_circle_r + 2.);

            // let c0 = rect.center_top() + Vec2::new(0., top);
            // let c1 = rect.center_top() + Vec2::new(0., top + small_circle_r * 1. + 2.);

            painter.line_segment([c0, c1], egui::Stroke::new(1.0, Color32::from_gray(180)));
            // painter.line_segment([c0, c1], egui::Stroke::new(1.0, Color32::RED));
        }
    } else {
        debug!("ams.units.len() = {:#?}", ams.units.len());
    }

    //
}

fn draw_ams_current(
    painter: &egui::Painter,
    circle_r: f32,
    circle_stroke: f32,
    c: Pos2,
    slot: &AmsSlot,
) {
    // let s = (circle_r + circle_stroke) * 2. + 2.;
    let s = (circle_r + circle_stroke) * 2.;
    let rect2 = egui::Rect::from_center_size(c, Vec2::splat(s));
    painter.rect_stroke(
        rect2,
        3.,
        egui::Stroke::new(circle_stroke, slot.color),
        egui::StrokeKind::Middle,
    );
}

#[cfg(feature = "nope")]
impl App {
    /// MARK: Header
    fn bambu_printer_header(
        &self,
        ui: &mut egui::Ui,
        status: &GenericPrinterState,
        printer: &PrinterConfigBambu,
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
                        egui::Id::new(format!("{:?}_drag_src_{}_{}", printer.id, pos.col, pos.row)),
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
                        egui::Id::new(format!("{:?}_drag_src_{}_{}", printer.id, pos.col, pos.row)),
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
