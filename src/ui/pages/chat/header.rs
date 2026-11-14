use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length};

use crate::ui::pages::chat::cast::CAST_LENGTH;
use crate::ui::pages::chat::{Message, Scene};
use crate::ui::{common, theme};

pub fn header(chat_state: &Scene) -> Element<'_, Message> {
    let back = button(text("← Adventures Hub").size(11.5))
        .padding([7.0, 13.0])
        .style(theme::back_button)
        .on_press(Message::BackRequested);

    let identity = column![
        text(title(chat_state))
            .size(21.0)
            .color(theme::GOLD)
            .font(common::brand()),
        text(headline(chat_state))
            .size(11.0)
            .color(theme::TEXT_MUTED)
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .wrapping(Wrapping::Word),
    ]
    .spacing(2.0)
    .align_x(Alignment::Center);

    row![
        container(back)
            .width(Length::Fixed(CAST_LENGTH))
            .align_x(Alignment::Start),
        container(identity)
            .width(Length::Fill)
            .align_x(Alignment::Center),
        Space::new().width(Length::Fixed(CAST_LENGTH)),
    ]
    .spacing(14.0)
    .align_y(Alignment::Center)
    .into()
}

fn title(chat_state: &Scene) -> &str {
    chat_state
        .adventure
        .as_ref()
        .map_or("No adventure open", |adventure| adventure.title.as_str())
}

fn headline(chat_state: &Scene) -> &str {
    chat_state
        .adventure
        .as_ref()
        .map_or("Pick an adventure from the shelf", |adventure| {
            adventure.headline.as_str()
        })
}
