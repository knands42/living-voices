use iced::widget::{Space, button, column, row, text, text_editor};
use iced::{Alignment, Element, Length};
use uuid::Uuid;

use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::character::Character;
use crate::backend::domain::memory::{Fact, State as SceneState};
use crate::backend::domain::message::Message as MessageLine;
use crate::ui::pages::builder::Effect;
use crate::ui::pages::builder::widgets::{
    caption, card, editor, field, labelled, pill, section_header, surface,
};
use crate::ui::{common, theme};

/// Who says one line of the opening.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Speaker {
    Narrator,
    Character(Uuid),
}

#[derive(Debug, Clone)]
pub enum Message {
    PlaceChanged(String),
    DayChanged(String),
    FactSubjectChanged(usize, String),
    FactTextChanged(usize, String),
    AddFact,
    RemoveFact(usize),
    BeatEdited(usize, text_editor::Action),
    BeatSpeakerPicked(usize, Speaker),
    AddBeat,
    RemoveBeat(usize),
    SaveRequested,
    Saved(Result<(), String>),
}

#[derive(Default)]
struct FactDraft {
    subject: String,
    fact: String,
}

struct Beat {
    speaker: Speaker,
    content: text_editor::Content,
}

/// The scene the adventure starts in: where and when, the facts around it, and
/// the first lines, the first of which the narrator speaks.
pub struct State {
    place: String,
    day: String,
    facts: Vec<FactDraft>,
    beats: Vec<Beat>,
}

impl State {
    pub const MAX_BEATS: usize = 3;

    pub fn load(scene: &SceneState, facts: Vec<Fact>, messages: Vec<MessageLine>) -> Self {
        let beats = if messages.is_empty() {
            vec![Beat::narrator()]
        } else {
            messages
                .iter()
                .take(Self::MAX_BEATS)
                .map(Beat::from_message)
                .collect()
        };

        let facts = facts
            .into_iter()
            .map(|fact| FactDraft {
                subject: fact.subject,
                fact: fact.fact,
            })
            .collect();

        Self {
            place: from_scene(&scene.place),
            day: from_scene(&scene.day),
            facts,
            beats,
        }
    }

    pub fn update(&mut self, message: Message) -> Option<Effect> {
        match message {
            Message::PlaceChanged(place) => self.place = place,
            Message::DayChanged(day) => self.day = day,
            Message::FactSubjectChanged(index, subject) => {
                self.edit_fact(index, |fact| fact.subject = subject)
            }
            Message::FactTextChanged(index, text) => self.edit_fact(index, |fact| fact.fact = text),
            Message::AddFact => self.facts.push(FactDraft::default()),
            Message::RemoveFact(index) => self.remove_fact(index),
            Message::BeatEdited(index, action) => {
                if let Some(beat) = self.beats.get_mut(index) {
                    beat.content.perform(action);
                }
            }
            Message::BeatSpeakerPicked(index, speaker) => self.pick_speaker(index, speaker),
            Message::AddBeat => self.add_beat(),
            Message::RemoveBeat(index) => self.remove_beat(index),
            Message::SaveRequested => return Some(Effect::SaveOpening),
            Message::Saved(Err(err)) => log::error!("could not save the opening: {err}"),
            Message::Saved(Ok(())) => {}
        }

        None
    }

    pub fn view<'a>(&'a self, characters: &'a [Character]) -> Element<'a, Message> {
        let state = if self.is_ready() {
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
                self.can_save(),
                Message::SaveRequested
            ),
        ]
        .spacing(16.0)
        .align_y(Alignment::Center);

        let body = column![
            scene_fields(self),
            facts_fields(self),
            beat_fields(self, characters)
        ]
        .spacing(26.0)
        .width(Length::Fill);

        column![head, surface(body)]
            .spacing(20.0)
            .width(Length::Fill)
            .into()
    }

    pub fn stored_place(&self) -> String {
        to_scene(&self.place)
    }

    pub fn stored_day(&self) -> String {
        to_scene(&self.day)
    }

    pub fn facts_snapshot(&self) -> Vec<Fact> {
        self.facts
            .iter()
            .filter(|draft| !draft.fact.trim().is_empty())
            .map(|draft| Fact::new(draft.subject.trim(), draft.fact.trim()))
            .collect()
    }

    pub fn outgoing_messages(&self, adventure: &Adventure) -> Vec<MessageLine> {
        self.beats
            .iter()
            .filter(|beat| !beat.text().trim().is_empty())
            .map(|beat| beat.message(adventure))
            .collect()
    }

    pub fn is_ready(&self) -> bool {
        !self.place.trim().is_empty()
            && !self.day.trim().is_empty()
            && !self.first_line().trim().is_empty()
    }

    fn can_save(&self) -> bool {
        !self.first_line().trim().is_empty()
    }

    fn first_line(&self) -> String {
        self.beats.first().map(Beat::text).unwrap_or_default()
    }

    fn edit_fact(&mut self, index: usize, change: impl FnOnce(&mut FactDraft)) {
        if let Some(fact) = self.facts.get_mut(index) {
            change(fact);
        }
    }

    fn remove_fact(&mut self, index: usize) {
        if index < self.facts.len() {
            self.facts.remove(index);
        }
    }

    fn pick_speaker(&mut self, index: usize, speaker: Speaker) {
        if index == 0 {
            return;
        }

        if let Some(beat) = self.beats.get_mut(index) {
            beat.speaker = speaker;
        }
    }

    fn add_beat(&mut self) {
        if self.beats.len() < Self::MAX_BEATS {
            self.beats.push(Beat::narrator());
        }
    }

    fn remove_beat(&mut self, index: usize) {
        if index > 0 && index < self.beats.len() {
            self.beats.remove(index);
        }
    }
}

impl Beat {
    fn narrator() -> Self {
        Self {
            speaker: Speaker::Narrator,
            content: text_editor::Content::new(),
        }
    }

    fn from_message(message: &MessageLine) -> Self {
        let speaker = match message.character_id {
            Some(character_id) => Speaker::Character(character_id),
            None => Speaker::Narrator,
        };

        Self {
            speaker,
            content: text_editor::Content::with_text(&message.text),
        }
    }

    fn text(&self) -> String {
        self.content.text()
    }

    fn message(&self, adventure: &Adventure) -> MessageLine {
        let text = self.text();

        match self.speaker {
            Speaker::Narrator => {
                MessageLine::new(adventure.id, None, Some(adventure.narrator.id), text, false)
            }
            Speaker::Character(character_id) => {
                MessageLine::new(adventure.id, Some(character_id), None, text, false)
            }
        }
    }
}

fn from_scene(value: &str) -> String {
    if value == SceneState::UNKNOWN {
        String::new()
    } else {
        value.to_string()
    }
}

fn to_scene(value: &str) -> String {
    if value.trim().is_empty() {
        SceneState::UNKNOWN.to_string()
    } else {
        value.trim().to_string()
    }
}

fn scene_fields(state: &State) -> Element<'_, Message> {
    column![
        caption("WHERE IT BEGINS"),
        row![
            labelled(
                "PLACE",
                field(
                    state.place.as_str(),
                    "The drowned harbour…",
                    Message::PlaceChanged
                )
            ),
            labelled(
                "DATE",
                field(
                    state.day.as_str(),
                    "The first night of the tide…",
                    Message::DayChanged
                )
            ),
        ]
        .spacing(24.0),
    ]
    .spacing(10.0)
    .into()
}

fn facts_fields(state: &State) -> Element<'_, Message> {
    let mut facts = column![].spacing(8.0);

    if state.facts.is_empty() {
        facts = facts.push(
            text("No facts yet. Add the things that are already true.")
                .size(11.5)
                .color(theme::TEXT_DIM),
        );
    }

    for (index, fact) in state.facts.iter().enumerate() {
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
        button(text("×").size(14.0))
            .padding([5.0, 9.0])
            .style(theme::back_button)
            .on_press(Message::RemoveFact(index)),
    ]
    .spacing(10.0)
    .align_y(Alignment::Center)
    .into()
}

fn beat_fields<'a>(state: &'a State, characters: &'a [Character]) -> Element<'a, Message> {
    let mut beats = column![].spacing(12.0);

    for (index, beat) in state.beats.iter().enumerate() {
        beats = beats.push(beat_card(index, beat, characters));
    }

    column![
        row![
            caption("THE FIRST LINES"),
            Space::new().width(Length::Fill),
            pill(
                "+ Add line",
                120.0,
                state.beats.len() < State::MAX_BEATS,
                Message::AddBeat
            ),
        ]
        .align_y(Alignment::Center),
        beats,
    ]
    .spacing(10.0)
    .into()
}

fn beat_card<'a>(
    index: usize,
    beat: &'a Beat,
    characters: &'a [Character],
) -> Element<'a, Message> {
    let speaker: Element<'a, Message> = if index == 0 {
        text("NARRATOR")
            .size(10.5)
            .color(theme::GOLD)
            .font(common::semibold())
            .into()
    } else {
        speaker_picker(characters, index, beat.speaker)
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

fn speaker_picker<'a>(
    characters: &'a [Character],
    index: usize,
    chosen: Speaker,
) -> Element<'a, Message> {
    let mut chips: Vec<Element<'a, Message>> =
        vec![speaker_chip(Speaker::Narrator, "Narrator", chosen, index)];

    for character in characters.iter().filter(|character| !character.is_player) {
        chips.push(speaker_chip(
            Speaker::Character(character.id),
            character.name.as_str(),
            chosen,
            index,
        ));
    }

    row(chips).spacing(6.0).wrap().into()
}

fn speaker_chip(
    speaker: Speaker,
    label: &str,
    chosen: Speaker,
    index: usize,
) -> Element<'_, Message> {
    button(text(label).size(11.0))
        .padding([5.0, 10.0])
        .style(theme::chip(chosen == speaker))
        .on_press(Message::BeatSpeakerPicked(index, speaker))
        .into()
}
