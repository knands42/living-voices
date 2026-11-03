use crate::backend::application::context::prompts::*;
use crate::backend::application::context::{ContextBuilder, ContextData};
use crate::backend::application::director::Director;
use crate::backend::application::port::chat_port::PromptTurn;
use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::character::Character;
use crate::backend::domain::message::Message;

// ----------------------------------------------------------------------------
// Chat prompt building
// ----------------------------------------------------------------------------
impl ContextBuilder {
    pub fn build_autonomous_chat_prompt(
        adventure: &Adventure,
        director: &Director,
        data: &ContextData,
        messages: &[Message],
    ) -> Vec<PromptTurn> {
        let head = [
            AUTONOMOUS_CHAT_PROMPT.to_string(),
            Self::build_settings_prompt(adventure),
            AUTONOMOUS_CHAT_INSTRUCTIONS.to_string(),
            Self::build_characters_prompt(adventure),
        ];

        Self::assemble(adventure, director, data, &head, messages)
    }

    pub fn build_single_character_chat_prompt(
        adventure: &Adventure,
        director: &Director,
        data: &ContextData,
        character: &Character,
        messages: &[Message],
    ) -> Vec<PromptTurn> {
        let head = [
            SINGLE_CHARACTER_CHAT_PROMPT.to_string(),
            Self::build_settings_prompt(adventure),
            SINGLE_CHARACTER_CHAT_INSTRUCTIONS.to_string(),
            "## CHARACTER".to_string(),
            character.render(),
        ];

        Self::assemble(adventure, director, data, &head, messages)
    }

    pub fn build_narrator_chat_prompt(
        adventure: &Adventure,
        director: &Director,
        data: &ContextData,
        messages: &[Message],
    ) -> Vec<PromptTurn> {
        let head = [
            NARRATOR_CHAT_PROMPT.to_string(),
            Self::build_settings_prompt(adventure),
            NARRATOR_CHAT_INSTRUCTIONS.to_string(),
        ];

        Self::assemble(adventure, director, data, &head, messages)
    }

    pub fn build_distill_prompt(data: &ContextData, messages: &[Message]) -> Vec<PromptTurn> {
        Self::build_memory_prompt(data, messages, DISTILL_PROMPT)
    }

    pub fn build_act_close_prompt(data: &ContextData, messages: &[Message]) -> Vec<PromptTurn> {
        Self::build_memory_prompt(data, messages, ACT_CLOSE_PROMPT)
    }

    pub fn build_act_transition_prompt(
        data: &ContextData,
        messages: &[Message],
    ) -> Vec<PromptTurn> {
        Self::build_memory_prompt(data, messages, ACT_TRANSITION_PROMPT)
    }
}

// ----------------------------------------------------------------------------
// Chat-only assembly internals
// ----------------------------------------------------------------------------
impl ContextBuilder {
    fn assemble(
        adventure: &Adventure,
        director: &Director,
        data: &ContextData,
        head: &[String],
        messages: &[Message],
    ) -> Vec<PromptTurn> {
        let mut turns = vec![PromptTurn::system(head.join("\n\n"))];

        turns.push(PromptTurn::system(Self::build_static_context(data)));
        turns.extend(Self::build_previous_messages(adventure, messages));

        let messages_in_act = data.messages_in_act;
        let max_messages = director.get_max_act_messages();

        if director.should_request_pacing_prompt(messages_in_act) {
            let last_closure_score = data
                .active_act()
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

        turns
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

    fn build_memory_prompt(
        data: &ContextData,
        messages: &[Message],
        lead_prompt: &str,
    ) -> Vec<PromptTurn> {
        let mut turns = Vec::with_capacity(messages.len() + 2);
        turns.push(PromptTurn::system(lead_prompt.to_string()));
        turns.push(PromptTurn::system(Self::build_static_context(data)));

        turns.extend(messages.iter().map(|message| {
            PromptTurn::player(format!("{}: {}", message.speaker_name, message.text))
        }));

        turns
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
