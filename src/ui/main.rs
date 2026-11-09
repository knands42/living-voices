use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length, Size, Task, Theme};

use crate::ui::pages::{chat, main_hub};

pub fn run() -> iced::Result {
    iced::application(State::boot, State::update, State::view)
        .title("Living Voices")
        .theme(|_: &State| Theme::CatppuccinMacchiato)
        .window_size(Size::new(1360.0, 900.0))
        .antialiasing(true)
        .run()
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    MainHub,
    Chat,
}

#[derive(Debug, Clone)]
pub enum Message {
    PageRequested(Page),
}

#[derive(Debug, Default)]
pub struct State {
    page: Page,
}

impl State {
    /// The one place the window starts from, so `main` never builds UI state.
    fn boot() -> Self {
        Self::default()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PageRequested(page) => {
                self.page = page;
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let content = match self.page {
            Page::MainHub => main_hub::view(),
            Page::Chat => chat::view(),
        };

        container(
            column![
                row![
                    text("Living Voices").size(28),
                    Space::new().width(Length::Fill),
                    self.tabs(),
                ]
                .align_y(Alignment::Center)
                .spacing(16),
                content,
            ]
            .spacing(24),
        )
        .padding(24)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    /// The switch every page shares: a new page needs a `Page` variant and a row
    /// here, nothing else.
    fn tabs(&self) -> Element<'_, Message> {
        let pages = [("Adventures", Page::MainHub), ("Chat", Page::Chat)];

        pages
            .into_iter()
            .fold(row![].spacing(8), |row, (label, page)| {
                let tab = button(text(label)).on_press(Message::PageRequested(page));
                row.push(if self.page == page {
                    tab.style(button::primary)
                } else {
                    tab.style(button::secondary)
                })
            })
            .into()
    }
}
