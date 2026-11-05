use std::hash::{Hash, Hasher};

use slint::{ModelRc, SharedString, VecModel};

use crate::backend::domain::adventure::{Adventure, AdventureSummary};
use crate::backend::domain::character::Character;
use crate::backend::domain::memory::{Act, Fact, State};
use crate::backend::domain::message::Message;
use crate::ui::{
    ActAuthorView, ActView, AdventureSummaryView, AdventureView, BuilderView, CharacterChatView,
    CharacterView, FactView, MessageView, StateView,
};
use uuid::Uuid;

pub fn adventure_view(adventure: Option<&Adventure>) -> AdventureView {
    match adventure {
        Some(adventure) => {
            let characters: Vec<CharacterChatView> = adventure
                .characters
                .iter()
                .filter(|&char| (!char.is_player))
                .map(|char| character_chat_view(char))
                .collect();
            let player = adventure
                .characters
                .iter()
                .find(|c| c.is_player)
                .map(character_chat_view)
                .unwrap_or_default();

            AdventureView {
                id: SharedString::from(adventure.id.to_string()),
                title: SharedString::from(adventure.title.clone()),
                headline: SharedString::from(adventure.headline.clone()),
                tone: SharedString::from(adventure.tone.clone()),
                characters: ModelRc::new(VecModel::from(characters)),
                lore: SharedString::from(adventure.lore.text.clone()),
                status: SharedString::from(adventure.status.as_str()),
                player,
            }
        }
        None => AdventureView::default(),
    }
}

pub fn adventure_summaries_model(
    adventure_summaries: &[AdventureSummary],
) -> ModelRc<AdventureSummaryView> {
    let items: Vec<AdventureSummaryView> = adventure_summaries
        .iter()
        .map(|a| AdventureSummaryView {
            id: SharedString::from(a.id.to_string()),
            title: SharedString::from(a.title.clone()),
            description: SharedString::from(a.headline.clone()),
            status: SharedString::from(a.status.as_str()),
        })
        .collect();
    ModelRc::new(VecModel::from(items))
}

pub fn builder_view(adventure: Option<&Adventure>, acts: &[Act], has_opening: bool) -> BuilderView {
    const PLACEHOLDER_TONE: &str = "neutral";

    match adventure {
        Some(adventure) => {
            let cast: Vec<CharacterChatView> = adventure
                .characters
                .iter()
                .filter(|c| !c.is_player)
                .map(character_chat_view)
                .collect();

            let act_views: Vec<ActAuthorView> = acts.iter().map(act_author_view).collect();

            let story_done =
                !adventure.headline.trim().is_empty() && adventure.tone.trim() != PLACEHOLDER_TONE;
            let cast_done = !cast.is_empty();
            let acts_done = acts.iter().any(|act| act.is_valid_act());
            let opening_done = has_opening;
            let ready_count = [story_done, cast_done, acts_done, opening_done]
                .iter()
                .filter(|done| **done)
                .count() as i32;

            BuilderView {
                id: SharedString::from(adventure.id.to_string()),
                title: SharedString::from(adventure.title.clone()),
                headline: SharedString::from(adventure.headline.clone()),
                status: SharedString::from(adventure.status.to_string()),
                narrator_name: SharedString::from(adventure.narrator.name.clone()),
                tone: SharedString::from(adventure.tone.clone()),
                lore: SharedString::from(adventure.lore.text.clone()),
                cast: ModelRc::new(VecModel::from(cast)),
                acts: ModelRc::new(VecModel::from(act_views)),
                story_done,
                cast_done,
                acts_done,
                opening_done,
                ready_count,
            }
        }
        None => BuilderView::default(),
    }
}

fn act_author_view(act: &Act) -> ActAuthorView {
    ActAuthorView {
        ordinal: act.ordinal as i32,
        goal: SharedString::from(act.goal.clone()),
        trigger: SharedString::from(act.trigger.clone().unwrap_or_default()),
        valid: act.is_valid_act(),
    }
}

pub fn character_view(character: &Character) -> CharacterView {
    CharacterView {
        id: SharedString::from(character.id.to_string()),
        adventure_id: SharedString::from(character.adventure_id.to_string()),
        name: SharedString::from(character.name.clone()),
        initials: SharedString::from(initials(&character.name)),
        hue: name_hue(&character.name),
        headline: SharedString::from(character.headline.clone()),
        status: SharedString::from(character.status.as_str()),
        backstory: SharedString::from(character.backstory.clone()),
        appearance: SharedString::from(character.appearance.clone().unwrap_or_default()),
        appearance_identity_prompt: optional_text(&character.appearance_identity_prompt),
        appearance_current_outfit: optional_text(&character.appearance_current_outfit),
        appearance_current_pose: optional_text(&character.appearance_current_pose),
        portrait_path: optional_text(&character.portrait_path),
        is_player: character.is_player,
        traits: string_model(&character.traits),
        tags: string_model(&character.tags),
        relations: string_model(&character.relations),
    }
}

fn optional_text(value: &Option<String>) -> SharedString {
    SharedString::from(value.clone().unwrap_or_default())
}

fn initials(name: &str) -> String {
    name.chars().take(2).collect::<String>().to_uppercase()
}

fn character_chat_view(character: &Character) -> CharacterChatView {
    CharacterChatView {
        id: SharedString::from(character.id.to_string()),
        name: SharedString::from(character.name.clone()),
        initials: SharedString::from(initials(&character.name)),
        headline: SharedString::from(character.headline.clone()),
        is_player: character.is_player,
        hue: name_hue(&character.name),
    }
}

fn name_hue(name: &str) -> i32 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    name.hash(&mut hasher);
    (hasher.finish() % 360) as i32
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
            conditions: string_model(
                &state
                    .character_conditions
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>(),
            ),
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
        hue: name_hue(&message.speaker_name),
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
