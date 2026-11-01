use crate::backend::application::context::ContextBuilder;
use crate::backend::application::context::prompts::CHAR_GEN_PROMPT;
use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::character::Character;

// ----------------------------------------------------------------------------
// Image prompt building
// ----------------------------------------------------------------------------
impl ContextBuilder {
    pub fn build_char_gen_prompt(adventure: &Adventure, character: &Character) -> String {
        let mut subject = vec![format!("## {}", character.name)];
        if !character.headline.trim().is_empty() {
            subject.push("### Character Headline".to_string());
            subject.push(character.headline.clone());
        }
        // Prefer the unconfirmed draft so a fresh generation reflects the
        // player's latest text; fall back to the committed appearance.
        let appearance = if character.appearance_draft.trim().is_empty() {
            character.appearance.trim()
        } else {
            character.appearance_draft.trim()
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

        [
            CHAR_GEN_PROMPT.to_string(),
            format!("#SUBJECT: {}", subject.join(". ")),
            format!("#STYLE: {}", style.join(". ")),
        ]
        .join("\n\n")
    }
}
