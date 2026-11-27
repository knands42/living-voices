use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length};

use crate::backend::domain::adventure::Adventure;
use crate::ui::layout::{self, ChatLayout, Docked, Placement};
use crate::ui::pages::chat::Message;
use crate::ui::pages::chat::cast::CAST_LENGTH;
use crate::ui::{common, theme};

const RAIL_TOGGLE_ALLOWANCE: f32 = 80.0;

pub fn header(
    adventure: &Adventure,
    fit: layout::ChatLayout,
    revealed: Option<layout::Rail>,
) -> Element<'_, Message> {
    row![
        leading(fit, revealed),
        title_block(adventure, fit.adventure_headline),
        trailing(fit, revealed),
    ]
    .spacing(14.0)
    .align_y(Alignment::Center)
    .into()
}

fn leading<'a>(chat_layout: ChatLayout, revealed: Option<layout::Rail>) -> Element<'a, Message> {
    let back = back_button(chat_layout.adventure_headline);

    if chat_layout.placement(layout::Rail::Cast) == Placement::Docked {
        return container(back)
            .width(Length::Fixed(CAST_LENGTH))
            .align_x(Alignment::Start)
            .into();
    }

    row![back, rail_toggle(layout::Rail::Cast, revealed)]
        .spacing(8.0)
        .align_y(Alignment::Center)
        .into()
}

fn title_block<'a>(adventure: &'a Adventure, show_headline: bool) -> Element<'a, Message> {
    let mut block = column![
        text(adventure.title.as_str())
            .size(21.0)
            .color(theme::GOLD)
            .font(common::brand())
    ];

    if show_headline {
        block = block.push(
            text(adventure.headline.as_str())
                .size(14.0)
                .color(theme::TEXT_MUTED)
                .width(Length::Fill)
                .align_x(Alignment::Center)
                .wrapping(Wrapping::Word),
        );
    }

    block
        .spacing(2.0)
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .into()
}

fn trailing<'a>(chat_layout: ChatLayout, revealed: Option<layout::Rail>) -> Element<'a, Message> {
    let mirror = if chat_layout.placement(layout::Rail::Facts) == Placement::Drawer {
        CAST_LENGTH - RAIL_TOGGLE_ALLOWANCE
    } else {
        CAST_LENGTH
    };
    let mirror = Space::new().width(Length::Fixed(mirror));
    let facts_toggle = rail_toggle(layout::Rail::Facts, revealed);

    let actions = match chat_layout.docked {
        Docked::Both => row![mirror],
        Docked::CastOnly => row![mirror, facts_toggle],
        Docked::Neither => row![facts_toggle],
    };

    actions.spacing(8.0).align_y(Alignment::Center).into()
}

fn back_button<'a>(show_headline: bool) -> Element<'a, Message> {
    if !show_headline {
        return button(text("←").size(12.5))
            .padding([7.0, 13.0])
            .style(theme::back_button)
            .on_press(Message::BackRequested)
            .into();
    }

    button(text("← Adventures Hub").size(12.5))
        .padding([7.0, 13.0])
        .style(theme::back_button)
        .on_press(Message::BackRequested)
        .into()
}

fn rail_toggle<'a>(rail: layout::Rail, revealed: Option<layout::Rail>) -> Element<'a, Message> {
    button(text(rail.label()).size(12.5).font(common::semibold()))
        .padding([7.0, 14.0])
        .style(theme::rail_toggle(revealed == Some(rail)))
        .on_press(Message::RevealToggled(rail))
        .into()
}
