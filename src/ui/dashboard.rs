use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

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

        for (pos, id) in self.printer_order.iter() {
            debug!("showing printer: {:?} at {:?}", id, pos);
        }

        for id in self.unplaced_printers.iter() {
            debug!("showing unplaced printer: {:?}", id);
        }

        // unimplemented!()

        let (pos, id) = self.printer_order.iter().next().unwrap();

        let printer = self.config.get_printer(id).unwrap();

        match printer {
            crate::config::printer_config::PrinterConfig::Bambu(_, printer) => {
                let printer = printer.blocking_read();
                self.show_printer_bambu(ui, *pos, &printer);
            }
            crate::config::printer_config::PrinterConfig::Klipper(_, _) => todo!(),
            crate::config::printer_config::PrinterConfig::Prusa(_, _) => todo!(),
        }

        //
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
}
