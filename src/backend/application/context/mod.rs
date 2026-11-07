mod chat_context;
mod image_context;
mod prompts;

use crate::backend::application::port::chat_port::PromptTurn;
use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::memory::{Act, ActStatus, Fact, State};
use crate::backend::domain::message::Message;
use crate::backend::persistence::act_persistence::{count_messages_per_act, get_acts};
use crate::backend::persistence::error::PersistenceError;
use crate::backend::persistence::fact_persistence::get_facts;
use crate::backend::persistence::state_persistence::get_state;
use sqlx::SqliteConnection;
use thiserror::Error;
use uuid::Uuid;

pub struct ContextBuilder;

/// How many closed acts keep their full digest in the prompt. Older ones collapse to
/// their index line, so the block stops growing with the length of the adventure.
const DIGESTED_PAST_ACTS: usize = 2;

/// How a past cast turn is written into the history
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CastTurn {
    Named,
    Replied,
    FollowedUp,
}

#[derive(Error, Debug)]
pub enum ContextBuilderError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] PersistenceError),
    #[error("Missing appearance description")]
    MissingAppearanceDescription,
}

/// Everything a prompt needs from the store, fetched once by the caller so the
/// builders stay pure (no `conn`, no queries, no snapshot skew).
pub struct ContextData {
    pub acts: Vec<Act>,
    pub facts: Vec<Fact>,
    pub state: Option<State>,
    pub messages_in_act: u16,
}

impl ContextData {
    pub async fn load(
        conn: &mut SqliteConnection,
        adventure_id: Uuid,
    ) -> Result<Self, PersistenceError> {
        Ok(Self {
            acts: get_acts(conn, adventure_id).await?,
            facts: get_facts(conn, adventure_id).await?,
            state: get_state(conn, adventure_id).await?,
            messages_in_act: count_messages_per_act(conn, adventure_id).await?,
        })
    }

    fn active_act(&self) -> Option<&Act> {
        self.acts.iter().find(|act| act.status == ActStatus::Active)
    }
}

// ----------------------------------------------------------------------------
// Shared assembly internals, used by both chat_context and image_context.
// ----------------------------------------------------------------------------
impl ContextBuilder {
    fn build_settings_prompt(adventure: &Adventure) -> String {
        let mut lines = vec!["## SETTINGS".to_string()];

        lines.push(format!("Title: {}", adventure.title));
        if !adventure.headline.trim().is_empty() {
            lines.push(format!("Premise: {}", adventure.headline));
        }
        if !adventure.tone.trim().is_empty() {
            lines.push(format!("Tone: {}", adventure.tone));
        }
        if !adventure.lore.text.trim().is_empty() {
            lines.push(format!("Lore: {}", adventure.lore.text));
        }
        lines.push(format!("Narrator: {}", adventure.narrator.name));

        lines.join("\n")
    }

    fn build_static_context(data: &ContextData) -> String {
        let final_prompt = [
            "## CONTEXT".to_string(),
            Self::build_acts_prompt(data),
            Self::build_current_state(data),
        ];

        final_prompt.join("\n\n")
    }

    fn build_acts_prompt(data: &ContextData) -> String {
        let current = data.active_act();
        let past: Vec<&Act> = data
            .acts
            .iter()
            .filter(|act| {
                act.status != ActStatus::Active
                    && act.status != ActStatus::Unstarted
                    && (act.digest.is_some() || act.index_line.is_some())
            })
            .collect();

        let mut sections = Vec::new();

        if !past.is_empty() {
            let digested = past.len().saturating_sub(DIGESTED_PAST_ACTS);

            let mut block = String::from("### PAST ACTS");
            for act in &past[..digested] {
                block.push_str(&format!("\n{}", act.render_index_line()));
            }
            for act in &past[digested..] {
                block.push_str(&format!("\n{}", act.render_digest_entry()));
            }
            sections.push(block);
        }

        if let Some(act) = current {
            sections.push(format!("### CURRENT ACT\n{}", act.render_index()));

            let running = act.render_running_digest();
            if !running.trim().is_empty() {
                sections.push(format!("### THIS ACT SO FAR\n{running}"));
            }
        }

        if let Some(facts) = Fact::render(&data.facts) {
            sections.push(format!("### FACTS\n{facts}"));
        }

        sections.join("\n\n")
    }

    fn build_current_state(data: &ContextData) -> String {
        if let Some(state) = &data.state
            && let Some(rendered) = state.render()
        {
            return format!("### CURRENT STATE\n{rendered}");
        }

        String::new()
    }

    /// The dynamic messaging setup to differentiate the user from the cast.
    fn build_previous_messages(
        adventure: &Adventure,
        messages: &[Message],
        cast_turn: CastTurn,
    ) -> Vec<PromptTurn> {
        messages
            .iter()
            .map(|message| Self::message_to_turn(adventure, message, cast_turn))
            .collect()
    }

    /// Classify a stored message. Does it come from the player or NPC?
    fn message_to_turn(
        adventure: &Adventure,
        message: &Message,
        cast_turn: CastTurn,
    ) -> PromptTurn {
        if Self::is_player_message(adventure, message) {
            PromptTurn::player(message.text.clone())
        } else {
            PromptTurn::cast(Self::render_cast_turn(message, cast_turn))
        }
    }

    fn render_cast_turn(message: &Message, cast_turn: CastTurn) -> String {
        match cast_turn {
            CastTurn::Named => format!("{}: {}", message.speaker_name, message.text),
            CastTurn::Replied => serde_json::json!({
                "replier": message.speaker_name,
                "reply": message.text,
                "act_closure": null,
            })
            .to_string(),
            CastTurn::FollowedUp => serde_json::json!({
                "replier": message.speaker_name,
                "reply": message.text,
            })
            .to_string(),
        }
    }

    fn is_player_message(adventure: &Adventure, message: &Message) -> bool {
        if message.narrator_id.is_some() {
            return false;
        }

        match message.character_id {
            Some(author_id) => adventure
                .characters
                .iter()
                .any(|c| c.id == author_id && c.is_player),
            None => false,
        }
    }
}
