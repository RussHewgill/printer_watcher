use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::config::printer_id::PrinterId;

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

// #[derive(Default, Clone, Deserialize, Serialize)]
#[derive(Default, Clone)]
pub struct ThumbnailMap {
    in_progress: HashSet<PrinterId>,
    thumbnails: HashMap<PrinterId, (String, Vec<u8>)>,
}

impl ThumbnailMap {
    pub fn get(&self, printer_id: &PrinterId) -> Option<&(String, Vec<u8>)> {
        self.thumbnails.get(printer_id)
    }

    pub fn insert(&mut self, printer_id: PrinterId, thumbnail: (String, Vec<u8>)) {
        self.thumbnails.insert(printer_id, thumbnail);
    }

    pub fn is_in_progress(&self, printer_id: &PrinterId) -> bool {
        self.in_progress.contains(printer_id)
    }

    pub fn set_in_progress(&mut self, printer_id: PrinterId, in_progress: bool) {
        if in_progress {
            self.in_progress.insert(printer_id);
        } else {
            self.in_progress.remove(&printer_id);
        }
    }
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
