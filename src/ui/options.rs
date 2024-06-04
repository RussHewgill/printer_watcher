use iced::widget::{container, scrollable, text, Column, Row};

use super::{message::AppMsg, model::AppModel, printer_widget::PRINTER_WIDGET_SIZE};

impl AppModel {
    pub fn show_options(&self) -> iced::Element<AppMsg> {
        text("Options").into()
    }
}
