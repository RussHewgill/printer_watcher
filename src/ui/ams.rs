use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui::{response, Color32, Layout, Pos2, Rect, RichText, Sense, Stroke, Vec2};

use crate::status::bambu_status::{AmsCurrentSlot, AmsUnit};

/// pretend that the configuration will always be one (external spool or AMS HT) + 1 AMS
// #[cfg(feature = "nope")]
pub(super) fn paint_ams_h2d(
    ui: &mut egui::Ui,
    // size: f32,
    // size: f32,
    height: f32,
    // ams: &AmsStatus,
    bambu: &crate::status::bambu_status::PrinterStateBambu,
) {
    let layout = Layout::left_to_right(egui::Align::Center)
        .with_cross_justify(true)
        .with_main_justify(true)
        .with_cross_align(egui::Align::Center);

    // // let height = 62.;
    // let height = 44.;

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

    let external_right = bambu
        .vir_slot
        .as_ref()
        .and_then(|v| v.get(1))
        .map(|v| &v.tray_color);

    let left_ams: Option<(&i64, &AmsUnit)> = {
        let mut out = None;
        for (i, unit) in ams.units.iter() {
            if let Some(info) = unit.info {
                if info / 100 == 11 {
                    out = Some((i, unit));
                }
            }
        }
        out
    };

    let right_ams: Option<(&i64, &AmsUnit)> = {
        let mut out = None;
        for (i, unit) in ams.units.iter() {
            if let Some(info) = unit.info {
                if info / 100 == 10 {
                    out = Some((i, unit));
                }
            }
        }
        out
    };

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

                egui::Frame::group(&ui.style())
                    .inner_margin(0.)
                    .outer_margin(0.)
                    .show(ui, |ui| {
                        let size = Vec2::new(ui.available_width(), height);
                        let (response, painter) = ui.allocate_painter(size, Sense::hover());

                        match left_ams {
                            Some((_, unit)) => {
                                // debug!("Left AMS: {:?}", unit);
                                _draw_ams_h2d(
                                    ui,
                                    &response,
                                    &painter,
                                    bambu,
                                    unit,
                                    ams.current_tray,
                                    bambu.device.extruder.as_ref(),
                                );
                            }
                            None => {
                                match external_left {
                                    Some(color) => {
                                        _draw_external_spool_h2d(
                                            ui, &response, &painter, bambu, color,
                                        );
                                    }
                                    None => {
                                        // debug!("No external spool found");
                                    }
                                }
                            }
                        }
                    });

                // _draw_ams_2_h2d(&painter, bambu);
            });

            strip.cell(|ui| {
                // ui.label("Right");
                egui::Frame::group(&ui.style())
                    .inner_margin(0.)
                    .outer_margin(0.)
                    .show(ui, |ui| {
                        // ui.ctx()
                        //     .debug_painter()
                        //     .debug_rect(ui.max_rect(), Color32::GREEN, "");

                        let size = Vec2::new(ui.available_width(), height);
                        let (response, painter) = ui.allocate_painter(size, Sense::hover());

                        match right_ams {
                            Some((_, unit)) => {
                                // debug!("Left AMS: {:?}", unit);
                                _draw_ams_h2d(
                                    ui,
                                    &response,
                                    &painter,
                                    bambu,
                                    unit,
                                    ams.current_tray,
                                    bambu.device.extruder.as_ref(),
                                );
                            }
                            None => {
                                match external_right {
                                    Some(color) => {
                                        _draw_external_spool_h2d(
                                            ui, &response, &painter, bambu, color,
                                        );
                                    }
                                    None => {
                                        // debug!("No external spool found");
                                    }
                                }
                            }
                        }
                    });
            });
        });

    // ui.ctx().debug_painter().debug_rect(
    //     ui.max_rect(),
    //     Color32::from_rgba_unmultiplied(255, 0, 0, 50),
    //     "",
    // );
}

const MARGIN_H: f32 = 2.;
const SLOT_SIZE: (f32, f32) = (30., 40.);

fn _draw_external_spool_h2d(
    ui: &mut egui::Ui,
    response: &response::Response,
    painter: &egui::Painter,
    bambu: &crate::status::bambu_status::PrinterStateBambu,
    color: &str,
) {
    // ui.label("Ext");

    let rect = response.rect;

    let y = rect.top() + MARGIN_H + (SLOT_SIZE.1 / 2.);

    let rect = Rect::from_center_size(
        Pos2::new(rect.center().x + SLOT_SIZE.0, y),
        Vec2::new(SLOT_SIZE.0, SLOT_SIZE.1),
    );

    let Ok(color) = Color32::from_hex(&format!("#{}", color)) else {
        error!("Invalid color: {}", color);
        return;
    };

    // painter.debug_text(
    //     Pos2::new(rect.left() + 5., rect.top() + 5.),
    //     egui::Align2::LEFT_TOP,
    //     ui.style().visuals.widgets.noninteractive.fg_stroke.color,
    //     "External Spool",
    // );

    painter.rect_filled(rect, 3, color);

    let border_color = ui.style().visuals.widgets.noninteractive.fg_stroke.color;
    painter.rect_stroke(
        rect,
        3,
        Stroke::new(3., border_color),
        egui::StrokeKind::Inside,
    );

    let mut rect = response.rect;
    rect.set_center(Pos2::new(rect.left() + rect.width() / 2. - 30., y));

    let layout = Layout::left_to_right(egui::Align::Center);

    ui.with_layout(layout, |ui| {
        ui.put(
            rect,
            egui::Label::new(RichText::new("External").strong().size(14.)),
        );
    });

    // unimplemented!()
}

fn _draw_ams_h2d(
    ui: &egui::Ui,
    response: &response::Response,
    painter: &egui::Painter,
    bambu: &crate::status::bambu_status::PrinterStateBambu,
    unit: &AmsUnit,
    current_tray: Option<AmsCurrentSlot>,
    extruder: Option<&crate::status::bambu_status::h2d_extruder::H2DExtruder>,
) {
    let rect = response.rect;

    let border_color = ui.style().visuals.widgets.noninteractive.fg_stroke.color;

    for (i, slot) in unit.slots.iter().enumerate() {
        // center = margin + slot width / 2 + slot width * i
        let x = rect.left()
            + MARGIN_H
            + ((SLOT_SIZE.0 + MARGIN_H * 2.) * i as f32)
            + (SLOT_SIZE.0 / 2.);
        let y = rect.top() + MARGIN_H + (SLOT_SIZE.1 / 2.);

        let rect = Rect::from_center_size(Pos2::new(x, y), Vec2::new(SLOT_SIZE.0, SLOT_SIZE.1));

        let is_current = match current_tray {
            Some(AmsCurrentSlot::Tray { ams_id, tray_id }) => {
                unit.id == ams_id as i64 && i == tray_id as usize
            }
            _ => false,
        };

        match slot {
            Some(slot) => {
                painter.rect_filled(rect, 3, slot.color);
                // if is_current {
                //     // let mut hsva: ecolor::HsvaGamma = slot.color.into();
                //     // hsva.v *= 0.7;
                //     // Stroke::new(10., Color32::from(hsva))

                //     let stroke = Stroke::new(
                //         5.,
                //         ui.style().visuals.widgets.noninteractive.fg_stroke.color,
                //     );

                //     let rect = rect.expand(1.);

                //     painter.rect_stroke(rect, 3, stroke, egui::StrokeKind::Inside);
                // } else {
                // };

                if is_current {
                    // let color = Color32::BLACK;
                    let color = make_border_color(slot.color);

                    let stroke = Stroke::new(5., color);
                    painter.rect_stroke(rect, 2, stroke, egui::StrokeKind::Inside);

                    // painter.circle_filled(
                    //     // rect.center(),
                    //     // rect.width() / 2. - 3.,
                    //     // Color32::from_black_alpha(100),
                    // );
                    // painter.rect_stroke(rect, 3, stroke, egui::StrokeKind::Inside);
                } else {
                    let stroke = Stroke::new(3., border_color);
                    painter.rect_stroke(rect, 3, stroke, egui::StrokeKind::Inside);
                }
            }
            None => {
                painter.rect_stroke(
                    rect,
                    3,
                    Stroke::new(3.0, border_color),
                    egui::StrokeKind::Inside,
                );
            }
        }
    }

    // unimplemented!()
}

fn make_border_color(main_color: Color32) -> Color32 {
    use palette::IntoColor;
    let mut hsv: ecolor::Hsva = main_color.into();
    let mut color: palette::Hsl = palette::Hsv::new(hsv.h, hsv.s, hsv.v).into_color();

    // // this doesn't work for some reason
    // let color = palette::Srgb::new(main_color.r(), main_color.g(), main_color.b());
    // let color: palette::Srgb<f32> = color.into();
    // let mut color: palette::Hsl = color.into_color();

    let c0 = color;
    if color.lightness > 0.5 {
        color.lightness *= 0.7
    } else {
        color.lightness *= 1.4
    };
    // debug!("Original color: \n{:?}, new color: \n{:?}", c0, color);

    let color: palette::Hsv = color.into_color();

    let color = Color32::from(ecolor::Hsva::new(
        // (color.hue.into_cartesian() + 1.0) / 2.,
        color.hue.into_positive_degrees() / 360.,
        color.saturation,
        color.value,
        1.0,
    ));

    color
}
