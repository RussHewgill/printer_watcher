use iced::{advanced::Widget, Element, Length, Size, Theme};

use crate::status::GenericPrinterState;

#[derive(Default)]
pub struct PrinterWidgetBambu {
    printer_state: GenericPrinterState,
}

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

impl<'a, Message, Theme, R> From<PrinterWidgetBambu> for Element<'a, Message, Theme, R>
where
    R: iced::advanced::renderer::Renderer,
{
    fn from(w: PrinterWidgetBambu) -> Self {
        Element::new(w)
    }
}
