use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Background, Border, Element, Length};
use uuid::Uuid;

use crate::backend::domain::character::{Character, CharacterStatus};
use crate::ui::pages::builder::Effect;
use crate::ui::pages::builder::widgets::{
    caption, empty_state, field, labelled, pill, section_header, surface, tag,
};
use crate::ui::{common, theme};

const CARD_WIDTH: f32 = 260.0;

#[derive(Debug, Clone)]
pub enum Message {
    AddRequested,
    Added(Result<Character, String>),
    Picked(usize),
    Changed(Character),
    EditClosed,
    RemoveRequested,
    Removed(Uuid, Result<(), String>),
    Saved(Result<Character, String>),
}

#[derive(Default)]
pub struct State {
    index: Option<usize>,
}

impl State {
    pub fn update(&mut self, message: Message, characters: &mut Vec<Character>) -> Option<Effect> {
        match message {
            Message::AddRequested => return Some(Effect::AddCharacter),
            Message::Added(Ok(character)) => {
                characters.push(character);
                self.index = Some(characters.len() - 1);
            }
            Message::Added(Err(err)) => log::error!("could not add a character: {err}"),
            Message::Picked(index) => self.index = Some(index),
            Message::Changed(character) => {
                if let Some(existing) = characters
                    .iter_mut()
                    .find(|existing| existing.id == character.id)
                {
                    *existing = character;
                }
            }
            Message::EditClosed => {
                let character_id = self.edited(characters).map(|character| character.id);
                self.index = None;
                return character_id.map(Effect::SaveCharacter);
            }
            Message::RemoveRequested => {
                let character = self.index.and_then(|index| characters.get(index))?;
                return Some(Effect::RemoveCharacter(character.id));
            }
            Message::Removed(character_id, Ok(())) => {
                characters.retain(|character| character.id != character_id);
                self.index = None;
            }
            Message::Removed(_, Err(err)) => {
                log::error!("could not remove the character: {err}")
            }
            Message::Saved(Ok(updated)) => {
                if let Some(existing) = characters
                    .iter_mut()
                    .find(|existing| existing.id == updated.id)
                {
                    existing.appearance_current_body = updated.appearance_current_body;
                    existing.appearance_current_outfit = updated.appearance_current_outfit;
                }
            }
            Message::Saved(Err(err)) => log::error!("could not save the character: {err}"),
        }

        None
    }

    pub fn view<'a>(&'a self, characters: &'a [Character]) -> Element<'a, Message> {
        let action = if self.edited(characters).is_some() {
            pill("Save", 150.0, true, Message::EditClosed)
        } else {
            pill("+ Add character", 150.0, true, Message::AddRequested)
        };

        let head = row![
            section_header("The Cast", "Characters, their traits and relationships."),
            action,
        ]
        .spacing(16.0)
        .align_y(Alignment::Center);

        let body: Element<'a, Message> = match self.edited(characters) {
            Some(character) => character_editor(character),
            None => grid(characters),
        };

        column![head, body]
            .spacing(20.0)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    pub fn is_ready(&self, characters: &[Character]) -> bool {
        characters
            .iter()
            .any(|character| !character.is_player && !character.is_placeholder())
    }

    pub fn can_save(&self, characters: &[Character]) -> bool {
        !characters.is_empty()
    }

    fn edited<'a>(&self, characters: &'a [Character]) -> Option<&'a Character> {
        self.index.and_then(|index| characters.get(index))
    }
}

fn grid(characters: &[Character]) -> Element<'_, Message> {
    if characters.is_empty() {
        return empty_state("No cast yet — add your first character.");
    }

    let cards: Vec<Element<'_, Message>> = characters
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
        .on_press(Message::Picked(index))
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
                .on_press(Message::RemoveRequested),
        );
    }

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
                .on_press(Message::Changed(Character {
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
        Message::Changed(character)
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
