use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui::{Color32, Response, Stroke, UiBuilder, Vec2};

use crate::ui::app::App;

impl App {
    pub fn show_quick_view(&mut self, ui: &mut egui::Ui) {
        ui.label("Quick View - TODO");
    }
}
