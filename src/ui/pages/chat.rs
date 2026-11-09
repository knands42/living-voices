use iced::widget::{button, column, container, text};
use iced::{Element, Length};

use crate::ui::main::{Message, Page};

pub fn view() -> Element<'static, Message> {
    container(
        column![
            text("Chat").size(24),
            text("hello world"),
            button("Back to adventures").on_press(Message::PageRequested(Page::MainHub)),
        ]
        .spacing(16),
    )
    .style(container::rounded_box)
    .padding(24)
    .width(Length::Fill)
    .into()
}
