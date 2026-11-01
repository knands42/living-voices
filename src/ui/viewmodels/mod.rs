use slint::{ModelRc, SharedString, VecModel};

use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::character::Character;
use crate::backend::domain::memory::{Act, Fact, State};
use crate::backend::domain::message::Message;
use crate::ui::{ActView, AdventureView, CharacterView, FactView, MessageView, StateView};
use uuid::Uuid;

pub fn adventure_view(adventure: Option<&Adventure>) -> AdventureView {
    match adventure {
        Some(adventure) => {
            let characters: Vec<CharacterView> = adventure
                .characters
                .iter()
                .filter_map(|char| (!char.is_player).then(|| character_view(char)))
                .collect();
            let player = adventure
                .characters
                .iter()
                .find(|c| c.is_player)
                .map(character_view)
                .unwrap_or_default();

            AdventureView {
                id: SharedString::from(adventure.id.to_string()),
                title: SharedString::from(adventure.title.clone()),
                headline: SharedString::from(adventure.headline.clone()),
                tone: SharedString::from(adventure.tone.clone()),
                characters: ModelRc::new(VecModel::from(characters)),
                player,
            }
        }
        None => AdventureView::default(),
    }
}

fn character_view(character: &Character) -> CharacterView {
    CharacterView {
        id: SharedString::from(character.id.to_string()),
        name: SharedString::from(character.name.clone()),
        headline: SharedString::from(character.headline.clone()),
        is_player: character.is_player,
    }
}

pub fn act_view(act: Option<&Act>) -> ActView {
    match act {
        Some(act) => ActView {
            goal: SharedString::from(act.goal.clone()),
            status: SharedString::from(act.status.to_string()),
            ordinal: act.ordinal as i32,
        },
        None => ActView::default(),
    }
}

pub fn state_view(state: Option<&State>) -> StateView {
    match state {
        Some(state) => StateView {
            day: SharedString::from(state.day.clone()),
            place: SharedString::from(state.place.clone()),
            characters_present: string_model(&state.characters_present),
            open_threads: string_model(&state.open_threads),
            conditions: string_model(&state.conditions),
        },
        None => StateView::default(),
    }
}

pub fn facts_model(facts: &[Fact]) -> ModelRc<FactView> {
    let items: Vec<FactView> = facts.iter().map(fact_view).collect();
    ModelRc::new(VecModel::from(items))
}

pub fn messages_model(messages: &[Message], player_id: Option<Uuid>) -> ModelRc<MessageView> {
    let items: Vec<MessageView> = messages
        .iter()
        .map(|message| message_view(message, player_id))
        .collect();
    ModelRc::new(VecModel::from(items))
}

pub fn message_view(message: &Message, player_id: Option<Uuid>) -> MessageView {
    MessageView {
        speaker: SharedString::from(message.speaker_name.clone()),
        text: SharedString::from(message.text.clone()),
        is_player: player_id.is_some() && message.character_id == player_id,
        is_narrator: message.narrator_id.is_some(),
    }
}

fn fact_view(fact: &Fact) -> FactView {
    FactView {
        subject: SharedString::from(fact.subject.clone()),
        fact: SharedString::from(fact.fact.clone()),
        permanence: SharedString::from(fact.permanence.to_string()),
    }
}

fn string_model(values: &[String]) -> ModelRc<SharedString> {
    let items: Vec<SharedString> = values.iter().map(SharedString::from).collect();
    ModelRc::new(VecModel::from(items))
}
