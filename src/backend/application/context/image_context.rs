use crate::backend::application::context::prompts::CHAR_GEN_PROMPT;
use crate::backend::application::context::{ContextBuilder, ContextBuilderError};
use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::character::Character;

// ----------------------------------------------------------------------------
// Image prompt building
// ----------------------------------------------------------------------------
impl ContextBuilder {
    pub fn build_char_gen_prompt(
        adventure: &Adventure,
        character: &Character,
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

        let mut style = Vec::new();
        if !adventure.tone.trim().is_empty() {
            style.push("## World where the character lives".to_string());
            style.push(format!("### tone: {}", adventure.tone));
        }
        if !adventure.lore.text.trim().is_empty() {
            style.push(format!("### setting: {}", adventure.lore.text));
        }

        Ok([
            CHAR_GEN_PROMPT.to_string(),
            format!("#SUBJECT: {}", subject.join(". ")),
            format!("#STYLE: {}", style.join(". ")),
        ]
        .join("\n\n"))
    }
}
