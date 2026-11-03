mod chat_context;
mod image_context;
mod prompts;

use crate::backend::application::port::chat_port::PromptTurn;
use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::memory::{Act, ActStatus, Fact};
use crate::backend::domain::message::Message;
use crate::backend::persistence::act_persistence::get_acts;
use crate::backend::persistence::error::PersistenceError;
use crate::backend::persistence::fact_persistence::get_facts;
use crate::backend::persistence::state_persistence::get_state;
use sqlx::SqliteConnection;
use thiserror::Error;
use uuid::Uuid;

pub struct ContextBuilder;

#[derive(Error, Debug)]
pub enum ContextBuilderError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] PersistenceError),
    #[error("Missing appearance description")]
    MissingAppearanceDescription,
}

// TODO: Drop conn from parameters... Provide everything that is needed via parameter
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

    async fn build_static_context(
        conn: &mut SqliteConnection,
        adventure: &Adventure,
    ) -> Result<String, PersistenceError> {
        let final_prompt = vec![
            "## CONTEXT".to_string(),
            Self::build_acts_prompt(conn, adventure).await?,
            Self::build_current_state(conn, adventure).await?,
        ];

        Ok(final_prompt.join("\n\n"))
    }

    async fn build_acts_prompt(
        conn: &mut SqliteConnection,
        adventure: &Adventure,
    ) -> Result<String, PersistenceError> {
        let acts = get_acts(conn, adventure.id).await?;

        let current = acts.iter().find(|act| act.status == ActStatus::Active);
        let past: Vec<&Act> = acts
            .iter()
            .filter(|act| {
                act.status != ActStatus::Active
                    && act.status != ActStatus::Unstarted
                    && act.digest.is_some()
            })
            .collect();

        let mut sections = Vec::new();

        if !past.is_empty() {
            let mut block = String::from("### PAST ACTS");
            for act in &past {
                block.push_str(&format!("\n{}", act.render_digest_entry()));
                if let Some(facts) = Self::render_act_facts(conn, adventure, act.id).await? {
                    block.push_str(&format!("\n{facts}"));
                }
            }
            sections.push(block);
        }

        if let Some(act) = current {
            let mut block = format!("### CURRENT ACT\n{}", act.render_index());
            if let Some(facts) = Self::render_act_facts(conn, adventure, act.id).await? {
                block.push_str(&format!("\n{facts}"));
            }
            sections.push(block);

            let running = act.render_running_digest();
            if !running.trim().is_empty() {
                sections.push(format!("### THIS ACT SO FAR\n{running}"));
            }
        }

        Ok(sections.join("\n\n"))
    }

    async fn render_act_facts(
        conn: &mut SqliteConnection,
        adventure: &Adventure,
        act_id: Uuid,
    ) -> Result<Option<String>, PersistenceError> {
        let facts = get_facts(conn, adventure.id, act_id).await?;
        Ok(Fact::render(&facts))
    }

    async fn build_current_state(
        conn: &mut SqliteConnection,
        adventure: &Adventure,
    ) -> Result<String, PersistenceError> {
        if let Some(state) = get_state(conn, adventure.id).await? {
            if let Some(rendered) = state.render() {
                return Ok(format!("### CURRENT STATE\n{rendered}"));
            }
        }

        Ok(String::new())
    }

    /// The dynamic messaging setup to differentiate the user from the cast.
    fn build_previous_messages(adventure: &Adventure, messages: &[Message]) -> Vec<PromptTurn> {
        messages
            .iter()
            .map(|message| Self::message_to_turn(adventure, message))
            .collect()
    }

    /// Classify a stored message. Does it come from the player or NPC?
    fn message_to_turn(adventure: &Adventure, message: &Message) -> PromptTurn {
        if Self::is_player_message(adventure, message) {
            PromptTurn::player(message.text.clone())
        } else {
            PromptTurn::cast(format!("{}: {}", message.speaker_name, message.text))
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
