use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Background, Border, Element, Length};

use crate::backend::domain::character::{Character, CharacterStatus};
use crate::ui::pages::builder::widgets::{
    caption, empty_state, field, labelled, pill, section_header, surface, tag,
};
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

    let body: Element<'_, Message> = match draft.edited_character() {
        Some(character) => character_editor(character),
        None => grid(draft),
    };

    column![head, body]
        .spacing(20.0)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn grid(draft: &Draft) -> Element<'_, Message> {
    if draft.adventure.characters.is_empty() {
        return empty_state("No cast yet — add your first character.");
    }

    let cards: Vec<Element<'_, Message>> = draft
        .adventure
        .characters
        .iter()
        .enumerate()
        .map(|(index, character)| cast_card(index, character))
        .collect();

    scrollable(row(cards).spacing(16.0).wrap())
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// A cast card. Picking it swaps the grid for that character's editor.
fn cast_card(index: usize, character: &Character) -> Element<'_, Message> {
    let hue = common::name_hue(character.name.as_str());
    let avatar = common::initials(
        &common::initials_of(character.name.as_str()),
        hue,
        52.0,
        0.58,
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
        text(character.name.as_str())
            .size(15.0)
            .color(theme::TEXT_LIGHT)
            .font(common::brand())
            .width(Length::Fill),
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

    button(container(column![avatar, detail].spacing(12.0).width(Length::Fill)).padding(14.0))
        .width(Length::Fixed(CARD_WIDTH))
        .padding(0.0)
        .style(card_style)
        .on_press(Message::CharacterPicked(index))
        .into()
}

fn card_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);

    button::Style {
        background: Some(Background::Color(theme::BG_PANEL)),
        text_color: theme::TEXT_LIGHT,
        border: Border::default()
            .rounded(14.0)
            .width(1.0)
            .color(if hovered {
                theme::GOLD
            } else {
                theme::BRONZE_DIM
            }),
        ..button::Style::default()
    }
}

fn character_editor(character: &Character) -> Element<'_, Message> {
    let mut heading = row![
        text(character.name.as_str())
            .size(18.0)
            .color(theme::GOLD)
            .font(common::brand()),
        Space::new().width(Length::Fill),
    ]
    .align_y(Alignment::Center)
    .width(Length::Fill);

    if !character.is_player {
        heading = heading.push(
            button(text("Remove").size(12.0))
                .padding([6.0, 14.0])
                .style(theme::back_button)
                .on_press(Message::RemoveCharacter),
        );
    }

    heading = heading.push(
        button(text("Done").size(12.0))
            .padding([6.0, 14.0])
            .style(theme::back_button)
            .on_press(Message::CharacterEditClosed),
    );

    let form = column![
        heading,
        common::divider(),
        row![
            labelled(
                "NAME",
                edit_field(
                    character.name.as_str(),
                    "Name this character…",
                    character,
                    |character, value| character.name = value
                )
            ),
            labelled(
                "GENRE",
                edit_field(
                    character.genre.as_str(),
                    "male, female…",
                    character,
                    |character, value| character.genre = value
                )
            ),
        ]
        .spacing(24.0),
        labelled(
            "HEADLINE",
            edit_field(
                character.headline.as_str(),
                "A tide-singer of the drowned harbour…",
                character,
                |character, value| character.headline = value
            )
        ),
        status_field(character),
        labelled(
            "BACKSTORY",
            edit_field(
                character.backstory.as_str(),
                "What happened before the story…",
                character,
                |character, value| character.backstory = value
            )
        ),
        look_fields(character),
    ]
    .spacing(18.0)
    .width(Length::Fill);

    surface(form)
}

fn look_fields(character: &Character) -> Element<'_, Message> {
    let look = labelled(
        "LOOK",
        edit_field(
            character.appearance.as_deref().unwrap_or_default(),
            "Face, hair, build and marks…",
            character,
            set_appearance,
        ),
    );

    if !character.is_player {
        return look;
    }

    row![
        look,
        labelled(
            "OUTFIT",
            edit_field(
                character.appearance_current_outfit.as_str(),
                "Cloak, boots, the token at your throat…",
                character,
                |character, value| character.appearance_current_outfit = value
            )
        ),
    ]
    .spacing(24.0)
    .into()
}

fn status_field(character: &Character) -> Element<'_, Message> {
    let mut chips: Vec<Element<'_, Message>> = Vec::with_capacity(CharacterStatus::ALL.len());

    for status in CharacterStatus::ALL {
        chips.push(
            button(text(status.as_str()).size(11.0))
                .padding([5.0, 10.0])
                .style(theme::chip(character.status == status))
                .on_press(Message::CharacterChanged(Character {
                    status,
                    ..character.clone()
                }))
                .into(),
        );
    }

    column![caption("STATUS"), row(chips).spacing(6.0)]
        .spacing(8.0)
        .into()
}

/// Every field sends the whole character back, so the page has one edit path.
fn edit_field<'a>(
    value: &'a str,
    placeholder: &'a str,
    character: &Character,
    change: fn(&mut Character, String),
) -> Element<'a, Message> {
    let base = character.clone();

    field(value, placeholder, move |edited| {
        let mut character = base.clone();
        change(&mut character, edited);
        Message::CharacterChanged(character)
    })
}

fn set_appearance(character: &mut Character, value: String) {
    character.appearance = (!value.trim().is_empty()).then_some(value);
}

fn status_tint(status: CharacterStatus) -> iced::Color {
    match status {
        CharacterStatus::Present => theme::SEA,
        CharacterStatus::Away => theme::AMBER,
        CharacterStatus::Unavailable => theme::ROSE,
    }
}
