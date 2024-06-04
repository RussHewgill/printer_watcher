use std::sync::Arc;

use iced::{
    advanced::Widget,
    widget::{column, container, row, text},
    Color, Element, Length, Size, Theme,
};
use tokio::sync::RwLock;

use crate::{
    config::printer_config::PrinterConfigBambu,
    status::{GenericPrinterState, GenericPrinterStateUpdate},
};

use super::message::AppMsg;

pub const PRINTER_WIDGET_SIZE: (f32, f32) = (250., 350.);

pub enum PrinterWidget {
    Bambu(PrinterWidgetBambu),
}

impl PrinterWidget {
    pub fn view(&self) -> Element<'_, AppMsg> {
        match self {
            PrinterWidget::Bambu(w) => w.view(),
        }
    }

    pub fn update(&mut self, update: GenericPrinterStateUpdate) {
        match self {
            PrinterWidget::Bambu(w) => {
                w.printer_state.update(update);
            }
        }
    }
}

// #[derive(Default)]
pub struct PrinterWidgetBambu {
    printer_config: Arc<RwLock<PrinterConfigBambu>>,
    printer_state: GenericPrinterState,
}

impl PrinterWidgetBambu {
    pub fn new(
        printer_config: Arc<RwLock<PrinterConfigBambu>>,
        printer_state: GenericPrinterState,
    ) -> Self {
        Self {
            printer_config,
            printer_state,
        }
    }

    pub fn view(&self) -> Element<'_, AppMsg> {
        let status_icon = iced::widget::Image::new(iced::widget::image::Handle::from_path(
            "assets/icons/play-circle_poly.svg",
        ))
        .width(30.)
        .height(30.);

        let name = self.printer_config.blocking_read().name.clone();

        let header = row![
            status_icon,
            // status_icon,
            text("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        ];

        // let content = column![
        //     row![iced::widget::Image::new(
        //         iced::widget::image::Handle::from_path("assets/icons/play-circle_poly.svg")
        //     ),],
        //     // text("test"),
        // ];

        // let content = content.height(Length::Fill).width(Length::Fill);
        // let content = content
        //     .height(Length::Fixed(PRINTER_WIDGET_SIZE.0))
        //     .width(Length::Fixed(PRINTER_WIDGET_SIZE.1));

        let content = column![
            header,
            // header,
            iced::widget::Image::new(iced::widget::image::Handle::from_path("test.jpg")),
            text("wat")
        ];

        container(content)
            .width(Length::Fixed(PRINTER_WIDGET_SIZE.0))
            .height(Length::Fixed(PRINTER_WIDGET_SIZE.1))
            .padding(10)
            .style(
                iced::widget::container::Appearance::default()
                    // .with_background(iced::Background::Color(Color::from_rgb8(255, 0, 0)))
                    .with_border(Color::from_rgb8(121, 173, 116), 3),
            )
            .into()
    }

    #[cfg(feature = "nope")]
    pub fn view(&self) -> Element<'_, AppMsg> {
        let content = column![
            text(&format!("Bambu Printer")),
            text(&format!("State: {:?}", self.printer_state.state)),
            text(&format!(
                "Nozzle Temp: {:.1}°C / {:.0}",
                self.printer_state.nozzle_temp, self.printer_state.nozzle_temp_target
            )),
            text(&format!(
                "Bed Temp: {:.1}°C / {:.0}",
                self.printer_state.bed_temp, self.printer_state.bed_temp_target
            )),
            text(&format!("Progress: {:.0}%", self.printer_state.progress)),
        ];

        container(content)
            .width(Length::Fixed(200.))
            .height(Length::Fixed(300.))
            .padding(10)
            .style(
                iced::widget::container::Appearance::default()
                    .with_background(iced::Background::Color(Color::from_rgb8(255, 0, 0)))
                    .with_border(Color::from_rgb8(0, 255, 0), 3),
            )
            .into()
    }
}
