use iced::widget::text::Wrapping;
use iced::widget::{Space, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

use crate::backend::domain::memory::state::State as SceneState;
use crate::backend::domain::memory::{CharacterCondition, Fact};
use crate::ui::pages::chat::{Message, Scene};
use crate::ui::{common, theme};

const META_LENGTH: f32 = 300.0;

pub fn meta_rail(chat_state: &Scene) -> Element<'_, Message> {
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

    notes = notes.push(note_card("Conditions", conditions));
    notes = notes.push(note_card("Open threads", threads));

    if !chat_state.facts.is_empty() {
        let mut facts = column![common::subheading("Facts")].spacing(6.0);

        for fact in &chat_state.facts {
            facts = facts.push(fact_row(fact));
        }

        notes = notes.push(facts);
    }

    let mut rail = column![scrollable(notes).height(Length::Fill)]
        .spacing(12.0)
        .width(Length::Fill);

    if let Some(scene_state) = scene {
        rail = rail.push(time_and_place(scene_state));
    }

    common::panel(rail, 14.0)
        .width(Length::Fixed(META_LENGTH))
        .height(Length::Fill)
        .into()
}

fn time_and_place(scene_state: &SceneState) -> Element<Message> {
    let card = column![
        text(scene_state.place.as_str())
            .size(13.2)
            .color(theme::TEXT_LIGHT)
            .wrapping(Wrapping::Word),
        text(scene_state.day.as_str())
            .size(12.1)
            .color(theme::TEXT_MUTED)
            .width(Length::Fill)
            .wrapping(Wrapping::Word),
        Space::new().height(Length::Fixed(2.0)),
        Space::new().height(Length::Fixed(2.0)),
    ]
    .spacing(4.0)
    .width(Length::Fill);

    container(card)
        .padding([8.0, 10.0])
        .width(Length::Fill)
        .style(theme::inset)
        .into()
}

fn note_card(title: &str, items: Vec<String>) -> Element<'_, Message> {
    if items.is_empty() {
        return Space::new().into();
    }

    let mut rows = column![].spacing(3.0);

    for item in items {
        rows = rows.push(
            text(format!("· {item}"))
                .size(12.1)
                .color(theme::TEXT_MUTED)
                .width(Length::Fill)
                .wrapping(Wrapping::Word),
        );
    }

    let card = container(rows)
        .padding([6.0, 8.0])
        .width(Length::Fill)
        .style(theme::inset);

    column![common::subheading(title), card].spacing(6.0).into()
}

fn fact_row(fact: &Fact) -> Element<'_, Message> {
    container(
        column![
            row![
                text(fact.subject.as_str())
                    .size(12.1)
                    .color(theme::BRONZE)
                    .font(common::semibold()),
                text(fact.permanence.as_str())
                    .size(9.8)
                    .color(theme::TEXT_DIM),
            ]
            .spacing(6.0)
            .align_y(Alignment::Center),
            text(fact.fact.as_str())
                .size(12.1)
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
