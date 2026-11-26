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
    show_adventure_headline: bool,
) -> Element<'_, Message> {
    let button_back = button(text("← Adventures Hub").size(12.5))
        .padding([7.0, 13.0])
        .style(theme::back_button)
        .on_press(Message::BackRequested);

    let adventure_title_column_header = {
        let mut adventure_column = column![];
        adventure_column = adventure_column.push(
            text(title(chat_state))
                .size(21.0)
                .color(theme::GOLD)
                .font(common::brand()),
        );

        if show_adventure_headline {
            adventure_column = adventure_column.push(
                text(headline(chat_state))
                    .size(14.0)
                    .color(theme::TEXT_MUTED)
                    .width(Length::Fill)
                    .align_x(Alignment::Center)
                    .wrapping(Wrapping::Word),
            )
        }

        adventure_column = adventure_column.spacing(2.0).align_x(Alignment::Center);
        adventure_column
    };

    let adventure_row_header = {
        let mut bar = row![].spacing(14.0).align_y(Alignment::Center);

        if fit.cast == Placement::Docked {
            bar = bar.push(
                container(button_back)
                    .width(Length::Fixed(CAST_LENGTH))
                    .align_x(Alignment::Start),
            );
        } else {
            bar = bar.push(
                row![button_back, rail_toggle(layout::Rail::Cast, revealed)]
                    .spacing(8.0)
                    .align_y(Alignment::Center),
            );
        }

        bar = bar.push(
            container(adventure_title_column_header)
                .width(Length::Fill)
                .align_x(Alignment::Center),
        );

        bar
    };

    let mut actions = row![].spacing(8.0).align_y(Alignment::Center);

    if fit.cast == Placement::Docked {
        actions = actions.push(Space::new().width(Length::Fixed(CAST_LENGTH)));
    }

    if fit.facts == Placement::Drawer {
        actions = actions.push(rail_toggle(layout::Rail::Facts, revealed));
    }

    adventure_row_header.push(actions).into()
}

fn rail_toggle<'a>(rail: layout::Rail, revealed: Option<layout::Rail>) -> Element<'a, Message> {
    button(text(rail.label()).size(12.5).font(common::semibold()))
        .padding([7.0, 14.0])
        .style(theme::rail_toggle(revealed == Some(rail)))
        .on_press(Message::RevealToggled(rail))
        .into()
}

fn title(chat_state: &OngoingAdventure) -> &str {
    chat_state.adventure.title.as_str()
}

fn headline(chat_state: &OngoingAdventure) -> &str {
    chat_state.adventure.headline.as_str()
}
