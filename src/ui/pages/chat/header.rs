use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length};

use crate::ui::layout::{self, Placement};
use crate::ui::pages::chat::cast::CAST_LENGTH;
use crate::ui::pages::chat::{Message, OngoingAdventure};
use crate::ui::{common, theme};

pub fn header(
    chat_state: &OngoingAdventure,
    fit: layout::ChatLayout,
    revealed: Option<layout::Rail>,
) -> Element<'_, Message> {
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
            .size(14.0)
            .color(theme::TEXT_MUTED)
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .wrapping(Wrapping::Word),
    ]
    .spacing(2.0)
    .align_x(Alignment::Center);

    let mut bar = row![].spacing(14.0).align_y(Alignment::Center);

    if fit.cast == Placement::Docked {
        bar = bar.push(
            container(back)
                .width(Length::Fixed(CAST_LENGTH))
                .align_x(Alignment::Start),
        );
    } else {
        bar = bar.push(
            row![back, rail_chip(layout::Rail::Cast, revealed)]
                .spacing(8.0)
                .align_y(Alignment::Center),
        );
    }

    bar = bar.push(
        container(identity)
            .width(Length::Fill)
            .align_x(Alignment::Center),
    );

    let mut actions = row![].spacing(8.0).align_y(Alignment::Center);

    if fit.facts == Placement::Drawer {
        actions = actions.push(rail_chip(layout::Rail::Facts, revealed));
    }

    if fit.cast == Placement::Docked {
        actions = actions.push(Space::new().width(Length::Fixed(CAST_LENGTH)));
    }

    bar.push(actions).into()
}

fn rail_chip<'a>(rail: layout::Rail, revealed: Option<layout::Rail>) -> Element<'a, Message> {
    button(text(rail.label()).size(11.0))
        .padding([5.0, 10.0])
        .style(theme::chip(revealed == Some(rail)))
        .on_press(Message::RevealToggled(rail))
        .into()
}

fn title(chat_state: &OngoingAdventure) -> &str {
    chat_state.adventure.title.as_str()
}

fn headline(chat_state: &OngoingAdventure) -> &str {
    chat_state.adventure.headline.as_str()
}
