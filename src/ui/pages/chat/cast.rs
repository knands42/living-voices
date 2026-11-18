use iced::widget::text::Wrapping;
use iced::widget::{column, container, mouse_area, row, scrollable, text};
use iced::{Alignment, Element, Length};
use uuid::Uuid;

use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::character::Character;
use crate::ui::pages::chat::look::AVATAR;
use crate::ui::{common, theme};

pub const CAST_LENGTH: f32 = 300.0;

#[derive(Debug, Clone)]
pub enum Message {
    Picked(Uuid),
}

pub enum Effect {
    LoadGallery(Uuid),
}

#[derive(Default)]
pub struct State {
    selected: Option<Uuid>,
}

impl State {
    pub fn update(&mut self, message: Message) -> Option<Effect> {
        match message {
            Message::Picked(character_id) => {
                if self.selected == Some(character_id) {
                    self.selected = None;
                    return None;
                }

                self.selected = Some(character_id);
                Some(Effect::LoadGallery(character_id))
            }
        }
    }

    pub fn view<'a>(&self, adventure: &'a Adventure) -> Element<'a, Message> {
        let mut rows = column![].spacing(4.0).width(Length::Fill);

        for character in &adventure.characters {
            let selected = self.selected == Some(character.id);
            rows = rows.push(cast_row(character, selected));
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
}

fn cast_row(character: &Character, selected: bool) -> Element<'_, Message> {
    let hue = common::name_hue(character.name.as_str());
    let initials = common::initials_of(character.name.as_str());
    let avatar = common::initials(&initials, hue, AVATAR, 0.55);

    let name = row![
        text(character.name.as_str())
            .size(14)
            .color(theme::hsv(hue, 0.45, 0.95))
            .font(common::semibold()),
        text(character.status.as_str())
            .size(10.7)
            .color(theme::TEXT_DIM),
    ]
    .spacing(6.0)
    .align_y(Alignment::Center);

    let identity = column![
        name,
        text(character.headline.as_str())
            .size(12)
            .color(theme::TEXT_MUTED)
            .width(Length::Fill)
            .wrapping(Wrapping::Word),
    ]
    .spacing(2.0)
    .width(Length::Fill);

    let body = row![avatar, identity]
        .spacing(9.0)
        .align_y(Alignment::Start);

    let is_player = character.is_player;
    let card = container(body)
        .padding(9.0)
        .width(Length::Fill)
        .style(move |theme| {
            if is_player && !selected {
                crate::ui::theme::player_card(theme)
            } else {
                crate::ui::theme::cast_row(0.0, selected, hue)
            }
        });

    mouse_area(card)
        .on_press(Message::Picked(character.id))
        .into()
}
