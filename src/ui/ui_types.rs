use serde::{Deserialize, Serialize};

#[derive(PartialEq, Deserialize, Serialize)]
pub enum Tab {
    Dashboard,
    Graphs,
    Printers,
    Projects,
    Options,
    // Debugging,
}

impl Default for Tab {
    fn default() -> Self {
        Self::Dashboard
    }
}

#[derive(Clone)]
pub enum Thumbnail {
    None,
    // Image(),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct GridLocation {
    pub col: usize,
    pub row: usize,
}

impl GridLocation {
    pub fn new(col: usize, row: usize) -> Self {
        Self { col, row }
    }
}

#[derive(Deserialize, Serialize)]
pub struct AppOptions {
    // pub dark_mode: bool,
    pub dashboard_size: (usize, usize),
    // pub selected_printer: Option<PrinterId>,
    // pub selected_printer_cfg: Option<NewPrinterEntry>,
}

impl Default for AppOptions {
    fn default() -> Self {
        Self {
            // dark_mode: false,
            dashboard_size: (4, 2),
            // selected_printer: None,
            // selected_printer_cfg: None,
        }
    }
}
