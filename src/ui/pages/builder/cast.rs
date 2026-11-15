use iced::widget::text::Wrapping;
use iced::widget::{column, row, scrollable, text};
use iced::{Alignment, Element, Length};

use crate::backend::domain::character::{Character, CharacterStatus};
use crate::ui::pages::builder::widgets::{card, empty_state, field, pill, section_header, tag};
use crate::ui::pages::builder::{Draft, Message};
use crate::ui::{common, theme};

const CARD_WIDTH: f32 = 260.0;

pub fn cast(draft: &Draft) -> Element<'_, Message> {
    let head = row![
        section_header("The Cast", "Characters, their traits and relationships."),
        pill("Save cast", 120.0, draft.cast_can_save(), Message::SaveCast),
        pill("+ Add character", 150.0, true, Message::AddCharacter),
    ]
    .spacing(16.0)
    .align_y(Alignment::Center);

    let body: Element<'_, Message> = if draft.adventure.characters.is_empty() {
        empty_state("No cast yet — add your first character.")
    } else {
        let cards: Vec<Element<'_, Message>> =
            draft.adventure.characters.iter().map(cast_card).collect();

        scrollable(row(cards).spacing(16.0).wrap())
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    };

    column![head, body]
        .spacing(20.0)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// A cast card: the name this page edits, then what the rest of the app reads.
fn cast_card(character: &Character) -> Element<'_, Message> {
    let hue = common::name_hue(character.name.as_str());
    let avatar = common::initials(
        &common::initials_of(character.name.as_str()),
        hue,
        52.0,
        0.58,
    );

    let name = field(
        character.name.as_str(),
        "Name this character…",
        move |name| Message::CharacterNameChanged(character.id, name),
    );

    let mut badges = row![].spacing(4.0).align_y(Alignment::Center);
    if character.is_player {
        badges = badges.push(tag("player", theme::GOLD));
    }
    if !character.genre.trim().is_empty() {
        badges = badges.push(tag(character.genre.as_str(), theme::TEXT_MUTED));
    }
    badges = badges.push(tag(
        character.status.as_str(),
        status_tint(character.status),
    ));

    let headline = if character.headline.trim().is_empty() {
        "No headline yet"
    } else {
        character.headline.as_str()
    };

    let mut detail = column![
        name,
        badges,
        text(headline)
            .size(12.0)
            .color(theme::TEXT_MUTED)
            .wrapping(Wrapping::Word),
    ]
    .spacing(8.0)
    .width(Length::Fill);

    if !character.traits.is_empty() {
        let traits: Vec<Element<'_, Message>> = character
            .traits
            .iter()
            .take(4)
            .map(|trait_| tag(trait_.as_str(), theme::TEXT_MUTED))
            .collect();
        detail = detail.push(row(traits).spacing(4.0).wrap());
    }

    card(
        column![avatar, detail].spacing(12.0).width(Length::Fill),
        Length::Fixed(CARD_WIDTH),
        theme::BRONZE_DIM,
    )
}

fn status_tint(status: CharacterStatus) -> iced::Color {
    match status {
        CharacterStatus::Present => theme::SEA,
        CharacterStatus::Away => theme::AMBER,
        CharacterStatus::Unavailable => theme::ROSE,
    }
}
