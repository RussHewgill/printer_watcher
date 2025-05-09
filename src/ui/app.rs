use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::{
    config::{printer_id::PrinterId, AppConfig},
    conn_manager::{PrinterConnCmd, PrinterConnMsg},
    status::GenericPrinterState,
    streaming::StreamCmd,
};

use super::ui_types::{AppOptions, GridLocation, PreviewType, Tab, ThumbnailMap, WebcamTexture};

#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
pub struct App {
    pub current_tab: Tab,

    #[serde(skip)]
    pub config: AppConfig,

    #[serde(skip)]
    pub cmd_tx: Option<tokio::sync::mpsc::UnboundedSender<PrinterConnCmd>>,

    #[serde(skip)]
    pub stream_cmd_tx: Option<tokio::sync::mpsc::UnboundedSender<StreamCmd>>,

    #[serde(skip)]
    pub msg_rx: Option<tokio::sync::mpsc::UnboundedReceiver<PrinterConnMsg>>,

    #[serde(skip)]
    pub printer_states: Arc<DashMap<PrinterId, GenericPrinterState>>,

    pub printer_order: HashMap<GridLocation, PrinterId>,
    #[serde(skip)]
    pub unplaced_printers: Vec<PrinterId>,

    // #[serde(skip)]
    pub thumbnails: ThumbnailMap,
    pub preview_setting: HashMap<PrinterId, PreviewType>,

    #[serde(skip)]
    pub webcam_textures: Arc<DashMap<PrinterId, WebcamTexture>>,

    #[serde(skip)]
    pub selected_stream: Option<PrinterId>,
    // #[serde(skip)]
    // pub printer_config_page: PrinterConfigPage,
    pub options: AppOptions,
    // #[serde(skip)]
    // pub login_window: Option<AppLogin>,

    // /// selected printer, show right panel when Some
    // pub selected_printer_controls: Option<PrinterId>,

    // #[serde(skip)]
    // pub printer_textures: Arc<DashMap<PrinterId, WebcamTexture>>,
}

/// new
impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        config: AppConfig,
        printer_states: Arc<DashMap<PrinterId, GenericPrinterState>>,
        cmd_tx: tokio::sync::mpsc::UnboundedSender<PrinterConnCmd>,
        msg_rx: tokio::sync::mpsc::UnboundedReceiver<PrinterConnMsg>,
        stream_cmd_tx: tokio::sync::mpsc::UnboundedSender<StreamCmd>,
    ) -> Self {
        let mut out = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
            // warn!("using default app state");
            // Self::default()
        } else {
            Self::default()
        };

        /// fake printers for testing
        {
            // crate::fake_printer::fake_printer(&config, printer_states.clone());
        }

        out.config = config;
        out.printer_states = printer_states;

        out.cmd_tx = Some(cmd_tx);
        out.msg_rx = Some(msg_rx);
        out.stream_cmd_tx = Some(stream_cmd_tx);

        out.unplaced_printers = out.config.printer_ids();

        debug!("printer_order: {:?}", out.printer_order);
        debug!("unplaced_printers: {:?}", out.unplaced_printers);

        /// for each printer that isn't in printer_order, queue to add
        for (_, id) in out.printer_order.iter() {
            out.unplaced_printers.retain(|p| p != id);
        }

        // out.printer_textures = printer_textures;

        #[cfg(feature = "nope")]
        /// remove printers that were previously placed but are no longer in the config
        {
            let current_printers = out
                .config
                .printer_ids()
                .into_iter()
                // .map(|c| c.serial.clone())
                .collect::<HashSet<_>>();
            out.unplaced_printers
                .retain(|p| current_printers.contains(p));
            out.printer_order
                .retain(|_, v| current_printers.contains(v));
        }

        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        cc.egui_ctx.set_fonts(fonts);

        cc.egui_ctx
            .send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::Vec2::new(
                1250., 950.,
            )));

        out
    }
}

/// sync
impl App {
    fn read_channels(&mut self) {
        let rx = self.msg_rx.as_mut().unwrap();

        let msg = match rx.try_recv() {
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => return,
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                error!("Disconnected from printer connection manager");
                return;
            }
            Ok(msg) => msg,
        };

        match msg {
            // PrinterConnMsg::WorkerMsg(_) => {}
            // PrinterConnMsg::LoggedIn => {}
            // PrinterConnMsg::SyncedProjects(projects) => {
            //     self.projects = projects;
            // }
            PrinterConnMsg::NewThumbnail(id, file, img) => {
                self.thumbnails.insert(id, (file, img));
            }
            _ => {
                warn!("unhandled message: {:?}", msg);
            }
        }
    }

    pub fn send_cmd(&self, cmd: PrinterConnCmd) -> Result<()> {
        let tx = self.cmd_tx.as_ref().unwrap();
        tx.send(cmd)?;
        Ok(())
    }

    pub fn send_stream_cmd(&self, cmd: StreamCmd) -> Result<()> {
        let tx = self.stream_cmd_tx.as_ref().unwrap();
        tx.send(cmd)?;
        Ok(())
    }
}

/// MARK: App
impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.read_channels();

        if cfg!(debug_assertions) && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.current_tab, Tab::Dashboard, "Dashboard");
                // ui.selectable_value(&mut self.current_tab, Tab::Graphs, "Graphs");
                // ui.selectable_value(&mut self.current_tab, Tab::Printers, "Printers");
                // ui.selectable_value(&mut self.current_tab, Tab::Projects, "Projects");
                ui.selectable_value(&mut self.current_tab, Tab::Options, "Options");
            });
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // let now = chrono::Local::now();

                // /// time from now until 10pm
                // let dt0 = now.date_naive().and_hms(22, 0, 0);
                // let dt0 = dt0 - now;

                let now = chrono::Local::now();
                let target = now
                    .date_naive()
                    .and_hms_opt(22, 0, 0)
                    .unwrap()
                    .and_local_timezone(now.timezone())
                    .unwrap();

                // If it's already past 10 PM, calculate for next day
                let dt0 = if now.time() >= chrono::NaiveTime::from_hms_opt(22, 0, 0).unwrap() {
                    target + chrono::Duration::days(1) - now
                } else {
                    target - now
                };

                ui.label(format!(
                    "Time to 10PM: {:02}h{:02}min",
                    dt0.num_hours(),
                    dt0.num_minutes() % 60
                ));

                let target = (now + chrono::Duration::days(1))
                    .date_naive()
                    .and_hms_opt(8, 0, 0)
                    .unwrap()
                    .and_local_timezone(now.timezone())
                    .unwrap();

                let dt0 = target - now;

                ui.separator();

                ui.label(format!(
                    "Time to 8AM tomorrow: {:02}h{:02}min",
                    dt0.num_hours(),
                    dt0.num_minutes() % 60
                ));
            });
        });

        match self.current_tab {
            Tab::Dashboard => {
                if let Some(id) = self.selected_stream.as_ref() {
                    // self.show_stream(ctx, id.clone());
                    let id = id.clone();
                    egui::CentralPanel::default().show(ctx, |ui| {
                        self.show_fullscreen_printer(ui, id);
                    });
                } else {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        egui::containers::ScrollArea::both()
                            .auto_shrink(false)
                            .show(ui, |ui| {
                                self.show_dashboard(ui);
                            });
                    });
                }
            }
            Tab::Graphs => {
                // egui::CentralPanel::default().show(ctx, |ui| {
                //     self.show_graphs(ui);
                // });
                unimplemented!()
            }
            Tab::Projects => {
                // self.show_project_view(ctx);
                unimplemented!()
            }
            Tab::Printers => {
                // self.show_printers_config(ctx);
                unimplemented!()
            }
            Tab::Options => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    self.show_options(ui);
                });

                // egui::CentralPanel::default().show(ctx, |ui| {
                //     ctx.texture_ui(ui);
                // });
            }
        }
    }
}

impl App {
    pub fn show_fullscreen_printer(&mut self, ui: &mut egui::Ui, id: PrinterId) {
        let Some(entry) = self.webcam_textures.get(&id) else {
            self.selected_stream = None;
            return;
        };
        if !entry.enabled.load(std::sync::atomic::Ordering::SeqCst) {
            self.selected_stream = None;
        }
        let entry = entry.texture.clone();

        let size = ui.available_size();

        // let size = Vec2::new(thumbnail_width, thumbnail_height);
        let img = egui::Image::from_texture((entry.id(), entry.size_vec2()))
            // .fit_to_exact_size(size)
            .max_size(size)
            .maintain_aspect_ratio(true)
            .corner_radius(egui::CornerRadius::same(4))
            .sense(egui::Sense::click());

        let resp = ui.add(img);

        if resp.clicked() {
            self.selected_stream = None;
        } else if resp.hovered() {
            ui.ctx().request_repaint();
        }

        //
    }
}
