use iced::{
    widget::{
        button, column, container, horizontal_space, row, scrollable, text, vertical_space, Column,
    },
    Alignment, Application, Color, Command, Element, Length, Theme,
};

pub mod message;
pub mod model;
mod printer_widget;

use message::AppMsg;
use model::{AppFlags, AppModel};

impl Application for AppModel {
    type Executor = iced::executor::Default;
    type Flags = AppFlags;
    type Message = AppMsg;
    type Theme = Theme;

    fn new(flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        let mut out = Self {
            current_tab: Default::default(),
            config: flags.config,
            cmd_tx: flags.cmd_tx,
            msg_rx: flags.msg_rx,
            printer_states: flags.printer_states,
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
        Command::none()
    }

    #[cfg(feature = "nope")]
    fn view(&self) -> iced::Element<Self::Message> {
        let scroll_to_end_button = || {
            button("Scroll to end").padding(10)
            // .on_press(Message::ScrollToEnd)
        };

        let scroll_to_beginning_button = || {
            button("Scroll to beginning").padding(10)
            // .on_press(Message::ScrollToBeginning)
        };

        let content = iced::widget::scrollable(
            //horizontal content
            row![
                column![
                    text("Let's do some scrolling!"),
                    vertical_space().height(2400)
                ],
                scroll_to_end_button(),
                text("Horizontal - Beginning!"),
                horizontal_space().width(1200),
                //vertical content
                column![
                    text("Horizontal - Middle!"),
                    scroll_to_end_button(),
                    text("Vertical - Beginning!"),
                    vertical_space().height(1200),
                    text("Vertical - Middle!"),
                    vertical_space().height(1200),
                    text("Vertical - End!"),
                    scroll_to_beginning_button(),
                    vertical_space().height(40),
                ]
                .spacing(40),
                horizontal_space().width(1200),
                text("Horizontal - End!"),
                scroll_to_beginning_button(),
            ]
            .align_items(Alignment::Center)
            .padding([0, 40, 0, 40])
            .spacing(40),
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
        .width(Length::Fill)
        .height(Length::Fill)
        // .id(SCROLLABLE_ID.clone())
        // .on_scroll(Message::Scrolled)
        ;

        container(content).padding(20).into()
    }

    fn view(&self) -> iced::Element<Self::Message> {
        // let mut col = Column::new()
        //     .spacing(20)
        //     .align_items(iced::Alignment::Center)
        //     .width(Length::Fill);

        // for y in 0..self.app_options.dashboard_size.1 {
        //     col = col.push(
        //         // container(text(format!("Row {}", y)))
        //         // container(row![text("1"), text("2"),])
        //         //     .width(280.)
        //         //     .height(150.0),
        //         row![text("1"), text("2"),],
        //     );
        // }

        let printer_box = || {
            // canvas()
            container(text("X"))
                .width(Length::Fixed(200.))
                .height(Length::Fixed(300.))
                // .style(iced::theme::Container::)
                .style(
                    iced::widget::container::Appearance::default()
                        .with_background(iced::Background::Color(Color::from_rgb8(255, 0, 0)))
                        .with_border(Color::from_rgb8(0, 255, 0), 3),
                )
        };

        #[rustfmt::skip]
        let content = iced::widget::scrollable(
            // col
            // text("1"),
            column![
                printer_box(),
                printer_box(),
            ],
            #[cfg(feature = "nope")]
            column![
                row![
                    text("1"),
                    horizontal_space().width(1200),
                    text("2"),
                ],
                vertical_space().height(600),
                row![
                    text("3"),
                    text("4"),
                ],
                vertical_space().height(600),
                row![
                    text("5"),
                    text("6"),
                ],
                // vertical_space().height(2400)
            ],
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

    #[cfg(feature = "nope")]
    fn view(&self) -> iced::Element<Self::Message> {
        // printer_widget::PrinterWidgetBambu::default().into()

        let mut column = Column::new()
            .spacing(20)
            .align_items(iced::Alignment::Center)
            .width(Length::Fill);

        for y in 0..self.app_options.dashboard_size.1 {
            // column = column.push(text(format!("Row {}", y)));
            column = column.push(container(text("X")).width(280.).height(150.0));
        }

        let properties = scrollable::Properties::new()
            .width(10)
            .margin(0)
            .scroller_width(10)
            .alignment(scrollable::Alignment::Start);

        scrollable(container(column))
            .direction(scrollable::Direction::Both {
                vertical: properties,
                horizontal: properties,
            })
            // .direction(scrollable::Direction::Vertical(
            //     scrollable::Properties::new(),
            // ))
            .height(250.0)
            .width(300.0)
            .into()
        // container(column)
        //     // .width(Length::Fill)
        //     // .height(Length::Fill)
        //     // .center_x()
        //     // .center_y()
        //     .into()
    }
}
