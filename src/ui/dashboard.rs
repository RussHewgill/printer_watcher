use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui::{Color32, Response, Stroke, Vec2};

use crate::{
    config::{printer_config::PrinterConfig, printer_id::PrinterId},
    status::PrinterState,
};

use super::{app::App, ui_types::GridLocation};

impl App {
    /// MARK: show_dashboard
    pub fn show_dashboard(&mut self, ui: &mut egui::Ui) {
        self.place_printers();

        let (width, height) = crate::ui::PRINTER_WIDGET_SIZE;

        let mut max_rect = ui.max_rect();

        max_rect.set_width(width);
        max_rect.set_height(height);

        // unimplemented!()
        // ui.label("test");

        // for (pos, id) in self.printer_order.iter() {
        //     debug!("showing printer: {:?} at {:?}", id, pos);
        // }

        // for id in self.unplaced_printers.iter() {
        //     debug!("showing unplaced printer: {:?}", id);
        // }

        let edge_padding = 0.;
        let printer_padding = 0.;

        let offset_x = Vec2::new(width + printer_padding, 0.);
        let offset_y = Vec2::new(0., height + printer_padding);

        let fixed_width = width - 16.;
        let fixed_height = height - 16. * 1.5;

        /// drag and drop
        let mut from = None;
        let mut to = None;

        for y in 0..self.options.dashboard_size.1 {
            let mut max_rect_row = max_rect;
            for x in 0..self.options.dashboard_size.0 {
                let pos = GridLocation { col: x, row: y };
                let (id, color) = self.get_printer_id_color(pos);

                ui.allocate_ui_at_rect(max_rect_row, |ui| {
                    /// Set colors
                    let prev_inactive = ui.visuals().widgets.inactive.bg_stroke;
                    let prev_active = ui.visuals().widgets.active.bg_stroke;
                    if let Some(color) = color {
                        ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::new(4., color);
                        ui.visuals_mut().widgets.active.bg_stroke = Stroke::new(4., color);
                    } else {
                        ui.visuals_mut().widgets.inactive.bg_stroke.width = 4.;
                        ui.visuals_mut().widgets.active.bg_stroke.width = 4.;
                    }

                    let frame = egui::Frame::group(ui.style())
                        .inner_margin(4.)
                        .outer_margin(4.)
                        // .stroke(Stroke::new(5., color))
                        // .stroke(Stroke::new(50., Color32::RED))
                        // .fill(color)
                        .rounding(6.);

                    let (_, dropped_payload) = ui.dnd_drop_zone::<GridLocation, ()>(frame, |ui| {
                        // Unset colors, is this necessary?
                        ui.visuals_mut().widgets.inactive.bg_stroke = prev_inactive;
                        ui.visuals_mut().widgets.active.bg_stroke = prev_active;

                        egui::containers::Resize::default()
                            .fixed_size(Vec2::new(fixed_width, fixed_height))
                            .show(ui, |ui| {
                                let Some(id) = id else {
                                    ui.label("Empty");
                                    // ui.allocate_space(ui.available_size());
                                    return;
                                };

                                let Some(printer) = self.config.get_printer(&id) else {
                                    warn!("Printer not found: {:?}", id);
                                    return;
                                };

                                if self.printer_states.contains_key(&id) {
                                    // let resp = self.printer_widget(ui, pos, &printer);
                                    self.printer_widget(ui, pos, &printer);
                                } else {
                                    ui.label("Printer not found");
                                    // ui.allocate_space(Vec2::new(w, h));
                                    // ui.allocate_space(ui.available_size());
                                    return;
                                }
                            });

                        // ui.label(format!("({}, {})", x, y));
                    });

                    // ui.allocate_space(ui.available_size());
                    if let Some(dragged_payload) = dropped_payload {
                        from = Some(dragged_payload);
                        to = Some(GridLocation { col: x, row: y });
                    }
                });

                max_rect_row = max_rect_row.translate(offset_x);
            }
            max_rect = max_rect.translate(offset_y);
        }

        if let (Some(from), Some(to)) = (from, to) {
            self.move_printer(&from, &to);
        }

        //
    }

    fn printer_widget(&mut self, ui: &mut egui::Ui, pos: GridLocation, printer: &PrinterConfig) {
        match printer {
            PrinterConfig::Bambu(id, printer) => {
                let Ok(printer) = printer.try_read() else {
                    warn!("printer locked");
                    return;
                };

                self.show_printer_bambu(ui, pos, &printer);
            }
            PrinterConfig::Klipper(id, printer) => {
                let Ok(printer) = printer.try_read() else {
                    warn!("printer locked");
                    return;
                };

                self.show_printer_klipper(ui, pos, &printer);
            }
            PrinterConfig::Prusa(id, printer) => {
                let Ok(printer) = printer.try_read() else {
                    warn!("printer locked");
                    return;
                };

                self.show_printer_prusa(ui, pos, &printer);
            } // PrinterConfig::Octoprint(id, print) => {
              //     todo!();
              // }
        }
    }

    fn place_printers(&mut self) {
        loop {
            let Some(pos) = self.next_empty_slot() else {
                break;
            };

            let Some(id) = self.unplaced_printers.pop() else {
                break;
            };

            self.printer_order.insert(pos, id);
        }
    }

    fn next_empty_slot(&self) -> Option<GridLocation> {
        for x in 0..self.options.dashboard_size.0 {
            for y in 0..self.options.dashboard_size.1 {
                let loc = GridLocation::new(x, y);
                if !self.printer_order.contains_key(&loc) {
                    return Some(loc);
                }
            }
        }
        None
    }

    fn get_printer_id_color(&mut self, pos: GridLocation) -> (Option<PrinterId>, Option<Color32>) {
        // warn!("TODO: get_printer_id_color");
        let id = if let Some(id) = self.printer_order.get(&pos) {
            id.clone()
        } else {
            return (None, None);
        };

        let Some(printer) = self.config.get_printer(&id) else {
            // warn!("Printer not found: {:?}", id);
            return (None, None);
        };

        let color = if let Some(status) = self.printer_states.get(&id) {
            match &status.state {
                PrinterState::Paused => Color32::from_rgb(173, 125, 90),
                PrinterState::Printing => Color32::from_rgb(121, 173, 116),
                PrinterState::Error(_) => Color32::from_rgb(173, 125, 90),
                PrinterState::Idle | PrinterState::Finished => Color32::from_rgb(158, 44, 150),
                PrinterState::Busy => Color32::from_rgb(73, 84, 218),
                // PrinterState::Disconnected => Color32::from_rgb(191, 0, 5),
                PrinterState::Disconnected => Color32::from_rgb(0, 0, 0),
                // _ => Color32::from_gray(127),
                // _ => Color32::GREEN,
                PrinterState::Unknown(_) => Color32::YELLOW,
            }
        } else {
            // debug!("no state");
            // Color32::from_gray(127)
            Color32::RED
        };

        // (Some(id), Some(color))

        // unimplemented!()
        return (Some(id), Some(color));
    }
}
