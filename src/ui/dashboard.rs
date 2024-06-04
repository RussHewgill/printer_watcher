use iced::widget::{column, container, row, scrollable, text, Column, Row};

use super::{message::AppMsg, model::AppModel, printer_widget::PRINTER_WIDGET_SIZE};

impl AppModel {
    pub fn show_dashboard(&self) -> iced::Element<AppMsg> {
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
                let pos = super::model::GridLocation::new(x, y);
                let content = if let Some(id) = self.printer_order.get(&pos) {
                    if let Some(w) = self.printer_widgets.get(&id) {
                        w.view()
                    } else {
                        text("Printer not found").into()
                    }
                } else {
                    text("Empty").into()
                };

                row = row.push(
                    container(content)
                        .width(PRINTER_WIDGET_SIZE.0)
                        .height(PRINTER_WIDGET_SIZE.1),
                );
                // row = row.push(content);
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

        container(column![
            iced::widget::vertical_space().height(10.),
            // iced::widget::vertical_space(10),
            content
        ])
        .into()

        // container(content).padding(20).into()
        // content.into()
    }
}
