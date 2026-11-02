use crate::backend::application::context::prompts::{CHAR_FULL_BODY_PROMPT, CHAR_PORTRAIT_PROMPT};
use crate::backend::application::context::{ContextBuilder, ContextBuilderError};
use crate::backend::domain::character::Character;
use crate::backend::domain::memory::CharacterCondition;

// ----------------------------------------------------------------------------
// Image prompt building
// ----------------------------------------------------------------------------
impl ContextBuilder {
    pub fn build_portrait_char_prompt(
        character: &Character,
        character_conditions: Vec<CharacterCondition>,
    ) -> Result<String, ContextBuilderError> {
        Self::build_character_prompt(CHAR_PORTRAIT_PROMPT, character, character_conditions)
    }

    pub fn build_full_body_char_prompt(
        character: &Character,
        character_conditions: Vec<CharacterCondition>,
    ) -> Result<String, ContextBuilderError> {
        Self::build_character_prompt(CHAR_FULL_BODY_PROMPT, character, character_conditions)
    }

    fn build_character_prompt(
        instruction: &str,
        character: &Character,
        character_conditions: Vec<CharacterCondition>,
    ) -> Result<String, ContextBuilderError> {
        let mut subject = vec![format!("## {}", character.name)];
        if !character.headline.trim().is_empty() {
            subject.push("### Character Headline".to_string());
            subject.push(character.headline.clone());
        }
        let appearance = if character.appearance_prompt.trim().is_empty() {
            return Err(ContextBuilderError::MissingAppearanceDescription);
        } else {
            character.appearance_prompt.trim()
        };
        if !appearance.is_empty() {
            subject.push("### Character Appearance".to_string());
            subject.push(appearance.to_string());
        }
        if !character.traits.is_empty() {
            subject.push("### Character Traits".to_string());
            subject.push(character.traits.join(", "));
        }
        if !character_conditions.is_empty() {
            subject.push("### Character Conditions".to_string());
            for condition in character_conditions {
                subject.push(condition.note);
            }
        }

        Ok([
            instruction.to_string(),
            format!("#SUBJECT: {}", subject.join(". ")),
        ]
        .join("\n\n"))
    }
}
