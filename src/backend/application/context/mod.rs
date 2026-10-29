mod prompts;

use crate::backend::application::context::prompts::*;
use crate::backend::application::director::Director;
use crate::backend::application::port::chat_port::PromptTurn;
use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::character::Character;
use crate::backend::domain::memory::{Act, ActStatus};
use crate::backend::domain::message::Message;
use crate::backend::persistence::act_persistence::{count_messages_per_act, get_acts};
use crate::backend::persistence::error::PersistenceError;
use crate::backend::persistence::message_persistence::fetch_last_n;
use crate::backend::persistence::state_persistence::get_state;
use sqlx::SqliteConnection;

pub struct ContextBuilder;

// ----------------------------------------------------------------------------
// Public API
// ----------------------------------------------------------------------------
impl ContextBuilder {
    pub async fn build_autonomous_chat_prompt(
        conn: &mut SqliteConnection,
        adventure: &Adventure,
        director: &Director,
    ) -> Result<Vec<PromptTurn>, PersistenceError> {
        let head = [
            AUTONOMOUS_CHAT_PROMPT.to_string(),
            Self::build_settings_prompt(adventure),
            Self::build_characters_prompt(adventure),
            AUTONOMOUS_CHAT_REPLY_CONTRACT.to_string(),
            AUTONOMOUS_CHAT_BODY_INSTRUCTION.to_string(),
            AUTONOMOUS_CHAT_RULES_INSTRUCTION.to_string(),
            AUTONOMOUS_CHAT_EXAMPLES_INSTRUCTION.to_string(),
        ];

        Self::assemble(conn, adventure, director, &head).await
    }

    pub async fn build_single_character_chat_prompt(
        conn: &mut SqliteConnection,
        adventure: &Adventure,
        director: &Director,
        character: &Character,
    ) -> Result<Vec<PromptTurn>, PersistenceError> {
        let head = [
            SINGLE_CHARACTER_CHAT_PROMPT.to_string(),
            Self::build_settings_prompt(adventure),
            SINGLE_CHARACTER_CHAT_BODY_INSTRUCTION.to_string(),
            SINGLE_CHARACTER_CHAT_RULES_INSTRUCTION.to_string(),
            SINGLE_CHARACTER_CHAT_EXAMPLES_INSTRUCTION.to_string(),
            character.render(),
        ];

        Self::assemble(conn, adventure, director, &head).await
    }

    pub async fn build_narrator_chat_prompt(
        conn: &mut SqliteConnection,
        adventure: &Adventure,
        director: &Director,
    ) -> Result<Vec<PromptTurn>, PersistenceError> {
        let head = [
            NARRATOR_CHAT_PROMPT.to_string(),
            Self::build_settings_prompt(adventure),
            NARRATOR_CHAT_BODY_INSTRUCTION.to_string(),
            NARRATOR_CHAT_RULES_INSTRUCTION.to_string(),
            NARRATOR_CHAT_EXAMPLES_INSTRUCTION.to_string(),
        ];

        Self::assemble(conn, adventure, director, &head).await
    }

    pub async fn build_distill_prompt(
        conn: &mut SqliteConnection,
        adventure: &Adventure,
        messages: &[Message],
    ) -> Result<Vec<PromptTurn>, PersistenceError> {
        Self::build_memory_prompt(conn, adventure, messages, DISTILL_PROMPT).await
    }

    pub async fn build_act_close_prompt(
        conn: &mut SqliteConnection,
        adventure: &Adventure,
        messages: &[Message],
    ) -> Result<Vec<PromptTurn>, PersistenceError> {
        Self::build_memory_prompt(conn, adventure, messages, ACT_CLOSE_PROMPT).await
    }

    pub async fn build_act_transition_prompt(
        conn: &mut SqliteConnection,
        adventure: &Adventure,
        messages: &[Message],
    ) -> Result<Vec<PromptTurn>, PersistenceError> {
        Self::build_memory_prompt(conn, adventure, messages, ACT_TRANSITION_PROMPT).await
    }
}

// ----------------------------------------------------------------------------
// Private assembly internals
// ----------------------------------------------------------------------------
impl ContextBuilder {
    // TODO: provide messages here already, instead of fetching it
    async fn assemble(
        conn: &mut SqliteConnection,
        adventure: &Adventure,
        director: &Director,
        head: &[String],
    ) -> Result<Vec<PromptTurn>, PersistenceError> {
        let mut turns = vec![PromptTurn::system(head.join("\n\n"))];

        turns.push(PromptTurn::system(
            Self::build_static_context(conn, adventure).await?,
        ));
        turns.extend(Self::build_previous_messages(conn, adventure).await?);

        let messages_in_act = count_messages_per_act(conn, adventure.id).await?;
        let max_messages = director.get_max_act_messages();

        if director.should_request_pacing_prompt(messages_in_act) {
            turns.push(PromptTurn::system(Self::build_pacing_prompt(
                messages_in_act,
                max_messages,
            )));
        }

        if director.should_request_closure_prompt(messages_in_act) {
            turns.push(PromptTurn::system(Self::build_closure_prompt(
                messages_in_act,
                max_messages,
            )));
        }

        Ok(turns)
    }

    fn build_settings_prompt(adventure: &Adventure) -> String {
        let mut lines = vec!["## SETTINGS".to_string()];

        lines.push(format!("Title: {}", adventure.title));
        if !adventure.description.trim().is_empty() {
            lines.push(format!("Premise: {}", adventure.description));
        }
        if let Some(genre) = adventure.genre.as_deref().filter(|g| !g.trim().is_empty()) {
            lines.push(format!("Genre: {genre}"));
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

    fn build_closure_prompt(
        current_number_of_messages: u16,
        current_act_messages_max_threshold: u16, // TODO: later it will be controlled on the adventure settings, for now defaults to 200
    ) -> String {
        CLOSURE_PROMPT
            .replace("{0}", &current_number_of_messages.to_string())
            .replace("{1}", &current_act_messages_max_threshold.to_string())
    }

    fn build_pacing_prompt(
        current_number_of_messages: u16,
        current_act_messages_max_threshold: u16,
    ) -> String {
        let percent = if current_act_messages_max_threshold == 0 {
            0
        } else {
            (current_number_of_messages as u32 * 100 / current_act_messages_max_threshold as u32)
                as u16
        };

        PACING_PROMPT
            .replace("{0}", &current_number_of_messages.to_string())
            .replace("{1}", &current_act_messages_max_threshold.to_string())
            .replace("{2}", &percent.to_string())
    }

    async fn build_memory_prompt(
        conn: &mut SqliteConnection,
        adventure: &Adventure,
        messages: &[Message],
        lead_prompt: &str,
    ) -> Result<Vec<PromptTurn>, PersistenceError> {
        let mut turns = Vec::with_capacity(messages.len() + 2);
        turns.push(PromptTurn::system(lead_prompt.to_string()));
        turns.push(PromptTurn::system(
            Self::build_static_context(conn, adventure).await?,
        ));

        turns.extend(messages.iter().map(|message| {
            PromptTurn::player(format!("{}: {}", message.speaker_name, message.text))
        }));

        Ok(turns)
    }

    async fn build_static_context(
        conn: &mut SqliteConnection,
        adventure: &Adventure,
    ) -> Result<String, PersistenceError> {
        let final_prompt = vec![
            "## CONTEXT".to_string(),
            Self::build_acts_prompt(conn, adventure).await?,
            Self::build_current_state(conn, adventure).await?,
            Self::build_relevant_memories(conn, adventure).await?,
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
            sections.push(format!("### PAST ACTS\n{}", Act::render_digest(&past)));
        }

        if let Some(act) = current {
            sections.push(format!("### CURRENT ACT\n{}", act.render_index()));

            let running = act.render_running_digest();
            if !running.trim().is_empty() {
                sections.push(format!("### THIS ACT SO FAR\n{running}"));
            }
        }

        Ok(sections.join("\n\n"))
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
    async fn build_previous_messages(
        conn: &mut SqliteConnection,
        adventure: &Adventure,
    ) -> Result<Vec<PromptTurn>, PersistenceError> {
        let messages = fetch_last_n(conn, adventure.id, AUTONOMOUS_HISTORY_WINDOW).await?;

        Ok(messages
            .into_iter()
            .map(|message| Self::message_to_turn(adventure, &message))
            .collect())
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

    async fn build_relevant_memories(
        _conn: &mut SqliteConnection,
        _adventure: &Adventure,
    ) -> Result<String, PersistenceError> {
        // TODO: retrieve the live facts for the acts
        Ok(String::new())
    }

    /// The full cast, each rendered in the adventure's tone.
    fn build_characters_prompt(adventure: &Adventure) -> String {
        if adventure.characters.is_empty() {
            return String::new();
        }

        let mut block = String::from("## CHARACTERS CAST");
        for character in &adventure.characters {
            block.push('\n');
            block.push_str(&character.render());
        }

        block
    }
}
