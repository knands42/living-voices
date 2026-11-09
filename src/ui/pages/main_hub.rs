use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length};

use crate::ui::main::{Message, Page};

/// Placeholder rows: this is where `AdventureManager::get_adventures_summaries`
/// will land, rendered as the same list.
const ADVENTURES: [(&str, &str, &str); 3] = [
    (
        "The Gilded Cage",
        "A hostage-wrapped court intrigue in a city that sells its own heirs.",
        "In progress",
    ),
    (
        "Salt and Ash",
        "A fishing town stopped answering the mainland three winters ago.",
        "Draft",
    ),
    (
        "Where the Lanterns Go",
        "Every road out of the valley leads back to the same inn.",
        "Draft",
    ),
];

pub fn view() -> Element<'static, Message> {
    let entries = ADVENTURES.into_iter().fold(
        column![].spacing(12),
        |list, (title, description, status)| list.push(entry(title, description, status)),
    );

    column![
        row![
            text("Choose your adventure").size(24),
            Space::new().width(Length::Fill),
            button("New adventure"),
            button("Open chat").on_press(Message::PageRequested(Page::Chat)),
        ]
        .align_y(Alignment::Center)
        .spacing(12),
        entries,
    ]
    .spacing(20)
    .into()
}

fn entry(
    title: &'static str,
    description: &'static str,
    status: &'static str,
) -> Element<'static, Message> {
    container(
        row![
            column![text(title).size(20), text(description).size(14)].spacing(4),
            Space::new().width(Length::Fill),
            text(status).size(14),
        ]
        .align_y(Alignment::Center)
        .spacing(16),
    )
    .style(container::rounded_box)
    .padding(16)
    .width(Length::Fill)
    .into()
}
