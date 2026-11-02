use crate::backend::application::context::prompts::{CHAR_FULL_BODY_PROMPT, CHAR_PORTRAIT_PROMPT};
use crate::backend::application::context::{ContextBuilder, ContextBuilderError};
use crate::backend::application::port::chat_port::PromptTurn;
use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::character::Character;
use crate::backend::domain::memory::CharacterCondition;
use crate::backend::domain::message::Message;

// ----------------------------------------------------------------------------
// Image prompt building
// ----------------------------------------------------------------------------
impl ContextBuilder {
    pub fn build_portrait_char_prompt(
        adventure: &Adventure,
        character: &Character,
        character_conditions: Vec<CharacterCondition>,
    ) -> Result<Vec<PromptTurn>, ContextBuilderError> {
        if let Some(appearance) = &character.appearance {
            return Self::build_character_prompt(
                CHAR_PORTRAIT_PROMPT,
                adventure,
                character.name.as_str(),
                character_conditions,
                appearance,
                &vec![],
            );
        }

        Err(ContextBuilderError::MissingAppearanceDescription)
    }

    pub fn build_full_body_char_prompt(
        adventure: &Adventure,
        character: &Character,
        character_conditions: Vec<CharacterCondition>,
    ) -> Result<Vec<PromptTurn>, ContextBuilderError> {
        if let Some(appearance) = &character.appearance {
            return Self::build_character_prompt(
                CHAR_FULL_BODY_PROMPT,
                adventure,
                character.name.as_str(),
                character_conditions,
                appearance,
                &vec![],
            );
        }

        Err(ContextBuilderError::MissingAppearanceDescription)
    }

    pub fn build_pose_body_prompt(
        adventure: &Adventure,
        character: &Character,
        character_conditions: Vec<CharacterCondition>,
        uncovered_messages: &Vec<&Message>,
    ) -> Result<Vec<PromptTurn>, ContextBuilderError> {
        if let Some(appearance) = &character.appearance {
            return Self::build_character_prompt(
                CHAR_FULL_BODY_PROMPT,
                adventure,
                character.name.as_str(),
                character_conditions,
                appearance,
                uncovered_messages,
            );
        }

        Err(ContextBuilderError::MissingAppearanceDescription)
    }

    fn build_character_prompt(
        instruction: &str,
        adventure: &Adventure,
        character_name: &str,
        character_conditions: Vec<CharacterCondition>,
        appearance: &str,
        uncovered_messages: &Vec<&Message>,
    ) -> Result<Vec<PromptTurn>, ContextBuilderError> {
        let mut subject = vec![format!("## {}", character_name)];
        let mut messages_content: Vec<PromptTurn> = Vec::with_capacity(uncovered_messages.len());

        // Appearance info
        if !appearance.is_empty() {
            subject.push("### Character Appearance".to_string());
            subject.push(appearance.to_string());
        }
        if !character_conditions.is_empty() {
            subject.push("### Character Conditions".to_string());
            for condition in character_conditions {
                subject.push(condition.note);
            }
        }

        // Previous messages
        if !uncovered_messages.is_empty() {
            messages_content.push(PromptTurn::system("### Character Messages".to_string()));
            messages_content.extend(Self::build_previous_messages(adventure, uncovered_messages));
        }

        let mut final_prompt = Vec::with_capacity(3);
        final_prompt.push(PromptTurn::system(instruction));
        final_prompt.push(PromptTurn::system(format!(
            "#SUBJECT: {}",
            subject.join(". ")
        )));

        if (!messages_content.is_empty()) {
            final_prompt.extend(messages_content);
        }

        Ok(final_prompt)
    }
}
