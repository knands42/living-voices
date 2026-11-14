use iced::widget::text::Wrapping;
use iced::widget::{Space, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

use crate::backend::domain::memory::{CharacterCondition, Fact};
use crate::ui::pages::chat::{Message, State};
use crate::ui::{common, theme};

const META_LENGTH: f32 = 280.0;

pub fn meta_rail(chat_state: &State) -> Element<'_, Message> {
    let scene = chat_state.scene_state.as_ref();

    let conditions = scene
        .map(|scene| {
            scene
                .character_conditions
                .iter()
                .map(CharacterCondition::to_string)
                .collect()
        })
        .unwrap_or_default();

    let threads = scene
        .map(|scene| scene.open_threads.clone())
        .unwrap_or_default();

    let mut notes = column![common::heading("What's going on")]
        .spacing(12.0)
        .width(Length::Fill);

    notes = notes.push(note_list("Conditions", conditions));
    notes = notes.push(note_list("Open threads", threads));

    if !chat_state.facts.is_empty() {
        let mut facts = column![common::subheading("Facts")].spacing(6.0);

        for fact in &chat_state.facts {
            facts = facts.push(fact_row(fact));
        }

        notes = notes.push(facts);
    }

    if let Some(scene) = scene {
        notes = notes.push(
            column![
                common::subheading("Time and place"),
                text(scene.day.as_str()).size(10.5).color(theme::TEXT_MUTED),
                text(scene.place.as_str())
                    .size(10.5)
                    .color(theme::TEXT_MUTED)
                    .width(Length::Fill)
                    .wrapping(Wrapping::Word),
            ]
            .spacing(3.0),
        );
    }

    common::panel(
        column![scrollable(notes).height(Length::Fill)].width(Length::Fill),
        14.0,
    )
    .width(Length::Fixed(META_LENGTH))
    .height(Length::Fill)
    .into()
}

fn note_list<'a>(title: &'a str, items: Vec<String>) -> Element<'a, Message> {
    if items.is_empty() {
        return Space::new().into();
    }

    let mut rows = column![common::subheading(title)].spacing(3.0);

    for item in items {
        rows = rows.push(
            text(format!("· {item}"))
                .size(10.5)
                .color(theme::TEXT_MUTED)
                .width(Length::Fill)
                .wrapping(Wrapping::Word),
        );
    }

    rows.into()
}

fn fact_row(fact: &Fact) -> Element<'_, Message> {
    container(
        column![
            row![
                text(fact.subject.as_str())
                    .size(10.5)
                    .color(theme::BRONZE)
                    .font(common::semibold()),
                text(fact.permanence.as_str())
                    .size(8.5)
                    .color(theme::TEXT_DIM),
            ]
            .spacing(6.0)
            .align_y(Alignment::Center),
            text(fact.fact.as_str())
                .size(10.5)
                .color(theme::TEXT_MUTED)
                .width(Length::Fill)
                .wrapping(Wrapping::Word),
        ]
        .spacing(2.0),
    )
    .padding([6.0, 8.0])
    .width(Length::Fill)
    .style(theme::inset)
    .into()
}
