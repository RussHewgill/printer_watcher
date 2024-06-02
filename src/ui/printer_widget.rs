use iced::{
    advanced::Widget,
    widget::{column, container, text},
    Color, Element, Length, Size, Theme,
};

use crate::status::{GenericPrinterState, GenericPrinterStateUpdate};

use super::message::AppMsg;

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

#[derive(Default)]
pub struct PrinterWidgetBambu {
    printer_state: GenericPrinterState,
}

impl PrinterWidgetBambu {
    pub fn new(printer_state: GenericPrinterState) -> Self {
        Self { printer_state }
    }

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

#[cfg(feature = "nope")]
impl<Message, Theme, R> Widget<Message, Theme, R> for PrinterWidgetBambu
where
    R: iced::advanced::renderer::Renderer,
{
    fn size(&self) -> iced::Size<iced::Length> {
        Size {
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }

    fn layout(
        &self,
        tree: &mut iced::advanced::widget::Tree,
        renderer: &R,
        limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        iced::advanced::layout::Node::new(Size::new(250., 340.))
    }

    fn draw(
        &self,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut R,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
    ) {
        unimplemented!()
    }
}

#[cfg(feature = "nope")]
impl<'a, Message, Theme, R> From<PrinterWidgetBambu> for Element<'a, Message, Theme, R>
where
    R: iced::advanced::renderer::Renderer,
{
    fn from(w: PrinterWidgetBambu) -> Self {
        Element::new(w)
    }
}
