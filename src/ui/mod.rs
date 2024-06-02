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

pub mod message;
pub mod model;
mod printer_widget;

use message::AppMsg;
use model::{AppFlags, AppModel};
use printer_widget::{PrinterWidget, PrinterWidgetBambu};

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
                PrinterConfig::Bambu(id, _) => {
                    let s = GenericPrinterState::default();
                    printer_widgets.insert(
                        id.clone(),
                        printer_widget::PrinterWidget::Bambu(PrinterWidgetBambu::new(s)),
                    );
                }
                PrinterConfig::Klipper(_, _) => todo!(),
                PrinterConfig::Prusa(_, _) => todo!(),
            }
        }

        let mut out = Self {
            current_tab: Default::default(),
            config: flags.config,
            cmd_tx: flags.cmd_tx,
            // msg_rx: flags.msg_rx,
            msg_rx: Arc::new(std::sync::Mutex::new(Some(flags.msg_rx))),
            // printer_states: flags.printer_states,
            printer_order,
            printer_widgets,
            unplaced_printers: vec![],
            app_options: Default::default(),
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
                debug!("got msg: {:?}", msg);
                self.handle_printer_msg(msg);
            }
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

        if let Some(rx) = self.msg_rx.lock().unwrap().take() {
            debug!("spawning subscription");
            message::subscribe(rx)
        } else {
            debug!("no subscription");
            iced::Subscription::none()
        }

        // message::subscribe(rx)

        // iced::Subscription::from_recipe(message::AppMsgRecipe { rx: rx })
        // iced::subscription::channel("AppMsgSubscription", 25, |mut output| async move {
        //     loop {
        //         //
        //     }
        // })
    }

    fn view(&self) -> iced::Element<Self::Message> {
        let mut cols = Column::new()
            .spacing(4)
            .align_items(iced::Alignment::Start)
            // .width(Length::Fill);
            ;

        for y in 0..self.app_options.dashboard_size.1 {
            let mut row = Row::new()
                .spacing(4)
                // .width(Length::Fill);
                .align_items(iced::Alignment::Start);

            for x in 0..self.app_options.dashboard_size.0 {
                let pos = model::GridLocation::new(x, y);
                let content = if let Some(id) = self.printer_order.get(&pos) {
                    if let Some(w) = self.printer_widgets.get(&id) {
                        w.view()
                    } else {
                        text("Printer not found").into()
                    }
                } else {
                    text("Empty").into()
                };

                row = row.push(container(content).width(280.).height(150.0));
            }
            cols = cols.push(row);
        }

        let content = iced::widget::scrollable(
            cols
        )
            .direction({
                let properties = scrollable::Properties::new()
                    .width(10)
                    .margin(0)
                    .scroller_width(10)
                    .alignment(scrollable::Alignment::Start);

                scrollable::Direction::Both {
                    horizontal: properties,
                    vertical: properties,
                }
            })
            // .width(Length::Fill)
            // .height(Length::Fill)
            ;

        // container(content).padding(20).into()
        content.into()
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
