use crate::backend::application::context::ContextBuilder;
use crate::backend::application::context::prompts::*;
use crate::backend::application::director::Director;
use crate::backend::application::port::chat_port::PromptTurn;
use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::character::Character;
use crate::backend::domain::message::Message;
use crate::backend::persistence::act_persistence::{count_messages_per_act, get_active_act};
use crate::backend::persistence::error::PersistenceError;
use sqlx::SqliteConnection;

// ----------------------------------------------------------------------------
// Chat prompt building
// ----------------------------------------------------------------------------
impl ContextBuilder {
    pub async fn build_autonomous_chat_prompt(
        conn: &mut SqliteConnection,
        adventure: &Adventure,
        director: &Director,
        messages: &Vec<&Message>,
    ) -> Result<Vec<PromptTurn>, PersistenceError> {
        let head = [
            AUTONOMOUS_CHAT_PROMPT.to_string(),
            Self::build_settings_prompt(adventure),
            AUTONOMOUS_CHAT_INSTRUCTIONS.to_string(),
            Self::build_characters_prompt(adventure),
        ];

        Self::assemble(conn, adventure, director, &head, messages).await
    }

    pub async fn build_single_character_chat_prompt(
        conn: &mut SqliteConnection,
        adventure: &Adventure,
        director: &Director,
        character: &Character,
        messages: &Vec<&Message>,
    ) -> Result<Vec<PromptTurn>, PersistenceError> {
        let head = [
            SINGLE_CHARACTER_CHAT_PROMPT.to_string(),
            Self::build_settings_prompt(adventure),
            SINGLE_CHARACTER_CHAT_INSTRUCTIONS.to_string(),
            "## CHARACTER".to_string(),
            character.render(),
        ];

        Self::assemble(conn, adventure, director, &head, messages).await
    }

    pub async fn build_narrator_chat_prompt(
        conn: &mut SqliteConnection,
        adventure: &Adventure,
        director: &Director,
        messages: &Vec<&Message>,
    ) -> Result<Vec<PromptTurn>, PersistenceError> {
        let head = [
            NARRATOR_CHAT_PROMPT.to_string(),
            Self::build_settings_prompt(adventure),
            NARRATOR_CHAT_INSTRUCTIONS.to_string(),
        ];

        Self::assemble(conn, adventure, director, &head, messages).await
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
// Chat-only assembly internals
// ----------------------------------------------------------------------------
impl ContextBuilder {
    async fn assemble(
        conn: &mut SqliteConnection,
        adventure: &Adventure,
        director: &Director,
        head: &[String],
        messages: &Vec<&Message>,
    ) -> Result<Vec<PromptTurn>, PersistenceError> {
        let mut turns = vec![PromptTurn::system(head.join("\n\n"))];

        turns.push(PromptTurn::system(
            Self::build_static_context(conn, adventure).await?,
        ));
        turns.extend(Self::build_previous_messages(adventure, messages));

        let messages_in_act = count_messages_per_act(conn, adventure.id).await?;
        let max_messages = director.get_max_act_messages();

        if director.should_request_pacing_prompt(messages_in_act) {
            let last_closure_score = get_active_act(conn, adventure.id)
                .await?
                .map(|act| act.last_closure_score)
                .unwrap_or(0);
            turns.push(PromptTurn::system(Self::build_pacing_prompt(
                messages_in_act,
                max_messages,
                last_closure_score,
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
        last_closure_score: u16,
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
            .replace("{3}", &last_closure_score.to_string())
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
