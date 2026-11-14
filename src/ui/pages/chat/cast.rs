use iced::widget::text::Wrapping;
use iced::widget::{column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

use crate::backend::domain::character::Character;
use crate::ui::pages::chat::look::{AVATAR, initials_of, name_hue};
use crate::ui::pages::chat::{Message, State};
use crate::ui::{common, theme};

pub const CAST_LENGTH: f32 = 250.0;

pub fn cast_rail(chat_state: &State) -> Element<'_, Message> {
    let mut rows = column![].spacing(4.0).width(Length::Fill);

    if let Some(adventure) = &chat_state.adventure {
        for character in &adventure.characters {
            rows = rows.push(cast_row(character));
        }
    }

    common::panel(
        column![
            common::heading("Cast"),
            scrollable(rows).height(Length::Fill),
        ]
        .spacing(10.0)
        .height(Length::Fill),
        14.0,
    )
    .width(Length::Fixed(CAST_LENGTH))
    .height(Length::Fill)
    .into()
}

fn cast_row(character: &Character) -> Element<'_, Message> {
    let hue = name_hue(character.name.as_str());
    let initials = initials_of(character.name.as_str());
    let avatar = common::initials(&initials, hue, AVATAR, 0.55);

    let name = row![
        text(character.name.as_str())
            .size(12.0)
            .color(theme::hsv(hue, 0.45, 0.95))
            .font(common::semibold()),
        text(character.status.as_str())
            .size(9.0)
            .color(theme::TEXT_DIM),
    ]
    .spacing(6.0)
    .align_y(Alignment::Center);

    let identity = column![
        name,
        text(character.headline.as_str())
            .size(10.0)
            .color(theme::TEXT_MUTED)
            .width(Length::Fill)
            .wrapping(Wrapping::Word),
    ]
    .spacing(2.0)
    .width(Length::Fill);

    let body = row![avatar, identity]
        .spacing(9.0)
        .align_y(Alignment::Start);

    if character.is_player {
        container(body)
            .padding(9.0)
            .style(theme::player_card)
            .into()
    } else {
        container(body).padding(9.0).into()
    }
}
