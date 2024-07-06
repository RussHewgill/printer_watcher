use anyhow::{anyhow, bail, ensure, Context, Result};
use egui::{Label, RichText, TextStyle};
use tracing::{debug, error, info, trace, warn};

pub fn run_error_app(error: String) -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            // .with_icon(icon)
            .with_resizable(false)
            .with_max_inner_size([300., 200.])
            .with_inner_size([300.0, 200.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Printer Watcher Error",
        native_options,
        Box::new(move |cc| Ok(Box::new(ErrorApp { error }))),
    )
}

pub struct ErrorApp {
    pub error: String,
}

impl eframe::App for ErrorApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add(Label::new(
                    RichText::new("Error:").text_style(TextStyle::Heading),
                ));
                ui.label(&self.error);
            });
        });
    }
}
