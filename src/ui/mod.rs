use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use std::{collections::HashMap, sync::Arc};

use iced::{
    widget::{
        button, column, container, horizontal_space, row, scrollable, text, vertical_space, Column,
        Row,
    },
    Alignment, Application, Color, Command, Element, Length, Theme,
};

pub mod dashboard;
pub mod icons;
pub mod message;
pub mod model;
pub mod options;
mod printer_widget;

use message::AppMsg;
use model::{AppFlags, AppModel, AppOptions, Tab};
use printer_widget::{PrinterWidget, PrinterWidgetBambu, PRINTER_WIDGET_SIZE};

use crate::{
    config::printer_config::PrinterConfig,
    conn_manager::{worker_message::WorkerMsg, PrinterConnMsg},
    status::GenericPrinterState,
};

impl Application for AppModel {
    type Executor = iced::executor::Default;
    type Flags = AppFlags;
    type Message = AppMsg;
    type Theme = Theme;

    fn new(flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        let printer_order = flags
            .state
            .printer_order
            .into_iter()
            .map(|(k, v)| (k, v.into()))
            .collect();

        let mut printer_widgets = HashMap::new();

        for printer in flags.config.printers() {
            match printer {
                PrinterConfig::Bambu(id, config) => {
                    let s = GenericPrinterState::default();
                    printer_widgets.insert(
                        id.clone(),
                        printer_widget::PrinterWidget::Bambu(PrinterWidgetBambu::new(config, s)),
                    );
                }
                PrinterConfig::Klipper(_, _) => todo!(),
                PrinterConfig::Prusa(_, _) => todo!(),
            }
        }

        let mut out = Self {
            current_tab: Tab::default(),
            config: flags.config,
            cmd_tx: flags.cmd_tx,
            // msg_rx: flags.msg_rx,
            msg_rx: Arc::new(tokio::sync::Mutex::new(flags.msg_rx)),
            // printer_states: flags.printer_states,
            printer_order,
            printer_widgets,
            dragged_printer: None,
            unplaced_printers: vec![],
            app_options: AppOptions::default(),
        };

        (out, iced::Command::none())
    }

    fn title(&self) -> String {
        "Printer Watcher".to_string()
    }

    fn theme(&self) -> Self::Theme {
        if self.app_options.dark_mode {
            iced::Theme::Dark
        } else {
            iced::Theme::Light
        }
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        match message {
            AppMsg::PrinterConnMsg(msg) => {
                // debug!("got msg: {:?}", msg);
                self.handle_printer_msg(msg);
            }
            AppMsg::SwitchTab(t) => self.current_tab = t,
        }
        Command::none()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        // Subscription::batch(self.downloads.iter().map(Download::subscription))
        // unimplemented!()
        // iced::subscription::run(|_| {
        // })
        // iced::Subscription::none()

        // let rx = self.msg_rx.lock().unwrap().take().unwrap();

        // if let Some(rx) = self.msg_rx.lock().unwrap().take() {
        //     debug!("spawning subscription");
        //     message::subscribe(rx)
        // } else {
        //     debug!("no subscription");
        //     iced::Subscription::none()
        // }

        // message::subscribe(&self.msg_rx)
        // unimplemented!()

        struct Listener;

        let rx = self.msg_rx.clone();

        // iced::Subscription::from_recipe(message::AppMsgRecipe { rx: rx })
        iced::subscription::channel(
            std::any::TypeId::of::<Listener>(),
            25,
            |mut output| async move {
                loop {
                    let mut rx = rx.lock().await;
                    match rx.recv().await {
                        Some(msg) => {
                            if let Err(e) =
                                futures::SinkExt::send(&mut output, AppMsg::PrinterConnMsg(msg))
                                    .await
                            {
                                error!("error sending message: {:?}", e);
                            }
                        }
                        None => {
                            error!("no message");
                        }
                    }
                    //
                }
            },
        )
    }

    fn view(&self) -> iced::Element<Self::Message> {
        iced_aw::Tabs::new(AppMsg::SwitchTab)
            .push(
                Tab::Dashboard,
                iced_aw::TabLabel::Text("Dashboard".to_string()),
                self.show_dashboard(),
            )
            .push(
                Tab::Options,
                iced_aw::TabLabel::Text("Options".to_string()),
                self.show_options(),
            )
            .set_active_tab(&self.current_tab)
            .into()

        // text("wat").into()

        // match self.current_tab {
        // }
        // unimplemented!()
    }
}

impl AppModel {
    pub fn handle_printer_msg(&mut self, msg: PrinterConnMsg) {
        match msg {
            PrinterConnMsg::WorkerMsg(id, msg) => match msg {
                WorkerMsg::StatusUpdate(update) => {
                    let Some(entry) = self.printer_widgets.get_mut(&id) else {
                        warn!("printer not found: {:?}", id);
                        return;
                    };

                    entry.update(update);
                }
                _ => unimplemented!(),
            },
        }
    }
}
