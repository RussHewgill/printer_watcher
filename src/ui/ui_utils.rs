use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui::{Color32, CornerRadius, Rect, Response, UiBuilder, Vec2};

pub fn put_ui(
    ui: &mut egui::Ui,
    // ui: &mut egui::Ui,
    size: Vec2,
    layout: Option<egui::Layout>,
    add_contents: impl FnOnce(&mut egui::Ui) -> Response,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());

    let layout = if let Some(layout) = layout {
        layout
    } else {
        egui::Layout::left_to_right(egui::Align::Center)
    };

    let builder = UiBuilder {
        max_rect: Some(rect),
        layout: Some(layout),
        ..Default::default()
    };
    // let mut ui = ui.child_ui(rect, layout, None);
    let mut ui = ui.new_child(builder);

    // add_contents(&mut ui)

    ui.set_max_size(size);

    // ui.visuals_mut().widgets.active.bg_fill = Color32::RED;
    // ui.visuals_mut().widgets.inactive.bg_fill = Color32::GREEN;
    egui::Frame::new()
        // .fill(Color32::BLUE)
        .inner_margin(0.)
        .outer_margin(0.)
        .show(&mut ui, |ui| {
            ui.set_max_size(size);
            let resp = add_contents(ui);
            ui.allocate_space(ui.available_size());
            resp
        })
        .response
}

fn put_ui_prev(
    ui: &mut egui::Ui,
    // ui: &mut egui::Ui,
    size: Vec2,
    add_contents: impl FnOnce(&mut egui::Ui) -> Response,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());

    let layout = egui::Layout::left_to_right(egui::Align::Center);

    let builder = UiBuilder {
        max_rect: Some(rect),
        layout: Some(layout),
        ..Default::default()
    };

    let mut ui = ui.new_child(builder);
    // let mut ui = ui.child_ui(rect, layout, None);

    ui.set_max_size(size);

    // ui.visuals_mut().widgets.active.bg_fill = Color32::RED;
    // ui.visuals_mut().widgets.inactive.bg_fill = Color32::GREEN;
    egui::Frame::new()
        // .fill(Color32::BLUE)
        .inner_margin(0.)
        .outer_margin(0.)
        .show(&mut ui, |ui| {
            ui.set_max_size(size);
            let resp = add_contents(ui);
            ui.allocate_space(ui.available_size());
            resp
        })
        .response

    // response
    // ui.allocate_ui_at_rect(rect, |ui| {
    //     //
    // })
    // .response
}

pub fn draw_pause_overlay(ui: &mut egui::Ui, resp: &Response) {
    let painter = ui.painter_at(resp.rect); // Get painter clipped to the image rect
    let rect = resp.rect;
    let center = rect.center();

    // Draw Pause Icon Overlay
    let painter = ui.painter_at(resp.rect); // Get painter clipped to the image rect
    let rect = resp.rect;
    let center = rect.center();

    // Make icon size relative to image size
    let icon_height = rect.height() * 0.4;
    let bar_width = icon_height * 0.35;
    let gap = bar_width * 0.5;
    let total_width = bar_width * 2.0 + gap;

    // Calculate positions for the two bars
    let bar1_top_left = center - Vec2::new(total_width / 2.0, icon_height / 2.0);
    let bar1_bottom_right = bar1_top_left + Vec2::new(bar_width, icon_height);

    let bar2_top_left = center + Vec2::new(gap / 2.0, -icon_height / 2.0);
    let bar2_bottom_right = bar2_top_left + Vec2::new(bar_width, icon_height);

    // Optional: Dim the background image slightly
    painter.rect_filled(
        rect,
        CornerRadius::same(4), // Match image corner radius
        Color32::from_rgba_unmultiplied(0, 0, 0, 100), // Semi-transparent black
    );

    // Draw the pause bars
    let icon_color = Color32::from_rgba_unmultiplied(255, 255, 255, 200); // Semi-transparent white
    let icon_rounding = (bar_width * 0.2) as u8;

    painter.rect_filled(
        egui::Rect::from_min_max(bar1_top_left, bar1_bottom_right),
        CornerRadius::same(icon_rounding),
        icon_color,
    );
    painter.rect_filled(
        egui::Rect::from_min_max(bar2_top_left, bar2_bottom_right),
        CornerRadius::same(icon_rounding),
        icon_color,
    );
}

pub fn draw_fan_speed(ui: &mut egui::Ui, resp: &Response, speed: f32) {
    // let painter = ui.painter_at(resp.rect); // Get painter clipped to the image rect
    // let rect = resp.rect;
    // let center = rect.center();

    unimplemented!()
}
