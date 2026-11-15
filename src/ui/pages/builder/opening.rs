use iced::widget::{Space, button, column, row, text};
use iced::{Alignment, Element, Length};

use crate::ui::pages::builder::widgets::{
    caption, card, editor, field, labelled, pill, section_header, surface,
};
use crate::ui::pages::builder::{Beat, Draft, FactDraft, Message, Opening, Speaker};
use crate::ui::{common, theme};

pub fn opening(draft: &Draft) -> Element<'_, Message> {
    let state = if draft.opening.is_ready() {
        "Opening is set up."
    } else {
        "Not set up yet."
    };

    let head = row![
        section_header(
            "The Opening",
            "Where the story begins, the facts around it, and the first lines."
        ),
        text(state).size(12.5).color(theme::TEXT_MUTED),
        pill(
            "Save opening",
            140.0,
            draft.opening.can_save(),
            Message::SaveOpening
        ),
    ]
    .spacing(16.0)
    .align_y(Alignment::Center);

    let body = column![scene_fields(draft), facts_fields(draft), beat_fields(draft),]
        .spacing(26.0)
        .width(Length::Fill);

    column![head, surface(body)]
        .spacing(20.0)
        .width(Length::Fill)
        .into()
}

fn scene_fields(draft: &Draft) -> Element<'_, Message> {
    column![
        caption("WHERE IT BEGINS"),
        row![
            labelled(
                "PLACE",
                field(
                    draft.opening.place.as_str(),
                    "The drowned harbour…",
                    Message::OpeningPlaceChanged
                )
            ),
            labelled(
                "DATE",
                field(
                    draft.opening.day.as_str(),
                    "The first night of the tide…",
                    Message::OpeningDayChanged
                )
            ),
        ]
        .spacing(24.0),
    ]
    .spacing(10.0)
    .into()
}

fn facts_fields(draft: &Draft) -> Element<'_, Message> {
    let mut facts = column![].spacing(8.0);

    if draft.opening.facts.is_empty() {
        facts = facts.push(
            text("No facts yet. Add the things that are already true.")
                .size(11.5)
                .color(theme::TEXT_DIM),
        );
    }

    for (index, fact) in draft.opening.facts.iter().enumerate() {
        facts = facts.push(fact_row(index, fact));
    }

    column![
        row![
            caption("WORLD FACTS"),
            Space::new().width(Length::Fill),
            pill("+ Add fact", 110.0, true, Message::AddFact),
        ]
        .align_y(Alignment::Center),
        facts,
    ]
    .spacing(10.0)
    .into()
}

fn fact_row(index: usize, fact: &FactDraft) -> Element<'_, Message> {
    row![
        field(fact.subject.as_str(), "who or what", move |subject| {
            Message::FactSubjectChanged(index, subject)
        }),
        field(fact.fact.as_str(), "what is true about it", move |text| {
            Message::FactTextChanged(index, text)
        }),
    ]
    .spacing(10.0)
    .into()
}

fn beat_fields(draft: &Draft) -> Element<'_, Message> {
    let mut beats = column![].spacing(12.0);

    for (index, beat) in draft.opening.beats.iter().enumerate() {
        beats = beats.push(beat_card(draft, index, beat));
    }

    column![
        row![
            caption("THE FIRST LINES"),
            Space::new().width(Length::Fill),
            pill(
                "+ Add line",
                120.0,
                draft.opening.beats.len() < Opening::MAX_BEATS,
                Message::AddBeat
            ),
        ]
        .align_y(Alignment::Center),
        beats,
    ]
    .spacing(10.0)
    .into()
}

fn beat_card<'a>(draft: &'a Draft, index: usize, beat: &'a Beat) -> Element<'a, Message> {
    let speaker: Element<'_, Message> = if index == 0 {
        text("NARRATOR")
            .size(10.5)
            .color(theme::GOLD)
            .font(common::semibold())
            .into()
    } else {
        speaker_picker(draft, index, beat.speaker)
    };

    let mut header = row![speaker].spacing(8.0).align_y(Alignment::Center);

    if index > 0 {
        header = header.push(Space::new().width(Length::Fill)).push(
            button(text("Remove").size(11.0))
                .padding([4.0, 10.0])
                .style(theme::back_button)
                .on_press(Message::RemoveBeat(index)),
        );
    }

    card(
        column![
            header.width(Length::Fill),
            editor(&beat.content, 92.0, move |action| Message::BeatEdited(
                index, action
            )),
        ]
        .spacing(8.0)
        .width(Length::Fill),
        Length::Fill,
        theme::BRONZE_DIM,
    )
}

fn speaker_picker(draft: &Draft, index: usize, chosen: Speaker) -> Element<'_, Message> {
    let mut chips: Vec<Element<'_, Message>> =
        vec![speaker_chip(Speaker::Narrator, "Narrator", chosen, index)];

    for character in draft
        .adventure
        .characters
        .iter()
        .filter(|character| !character.is_player)
    {
        chips.push(speaker_chip(
            Speaker::Character(character.id),
            character.name.as_str(),
            chosen,
            index,
        ));
    }

    row(chips).spacing(6.0).wrap().into()
}

fn speaker_chip<'a>(
    speaker: Speaker,
    label: &'a str,
    chosen: Speaker,
    index: usize,
) -> Element<'a, Message> {
    button(text(label).size(11.0))
        .padding([5.0, 10.0])
        .style(theme::chip(chosen == speaker))
        .on_press(Message::BeatSpeakerPicked(index, speaker))
        .into()
}
