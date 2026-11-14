use iced::widget::{button, column, container, text};
use iced::{Element, Length, Task};

use crate::ui::pages::{Event, Page};

#[derive(Debug, Clone)]
pub enum Message {
    BackRequested,
}

pub struct State;

impl State {
    pub fn boot() -> (Self, Task<Message>) {
        (Self, Task::none())
    }

    pub fn update(&mut self, message: Message) -> (Task<Message>, Event) {
        match message {
            Message::BackRequested => (Task::none(), Event::Show(Page::MainHub)),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        container(
            column![
                text("Chat").size(24),
                text("hello world"),
                button("Back to adventures").on_press(Message::BackRequested),
            ]
            .spacing(16),
        )
        .style(container::rounded_box)
        .padding(24)
        .width(Length::Fill)
        .into()
    }
}
