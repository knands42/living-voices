use crate::backend::application::context::prompts::{
    CHAR_FULL_BODY_FRAMING, CHAR_IDENTITY_PROMPT, CHAR_OUTFIT_PROMPT, CHAR_PORTRAIT_FRAMING,
    CHAR_POSE_BODY_PROMPT,
};
use crate::backend::application::context::{ContextBuilder, ContextBuilderError};
use crate::backend::application::port::chat_port::PromptTurn;
use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::character::{Character, ImageFraming};
use crate::backend::domain::memory::CharacterCondition;
use crate::backend::domain::message::Message;

// ----------------------------------------------------------------------------
// Image prompt building
//
// A character image prompt is three layers that age at different speeds:
// identity (never), outfit (per scene) and pose (per beat). Each has its own
// model call and its own slot on the character; only `compose_character_image_prompt`
// knows how they fit together, so asking for a different shot never re-runs a model.
// ----------------------------------------------------------------------------
impl ContextBuilder {
    pub fn build_identity_char_prompt(
        adventure: &Adventure,
        character: &Character,
        character_conditions: Vec<CharacterCondition>,
    ) -> Result<Vec<PromptTurn>, ContextBuilderError> {
        if let Some(appearance) = &character.appearance {
            return Self::build_character_prompt(
                CHAR_IDENTITY_PROMPT,
                adventure,
                character.name.as_str(),
                character_conditions,
                appearance,
                &[],
            );
        }

        Err(ContextBuilderError::MissingAppearanceDescription)
    }

    pub fn build_outfit_char_prompt(
        adventure: &Adventure,
        character: &Character,
        character_conditions: Vec<CharacterCondition>,
        uncovered_messages: &[Message],
    ) -> Result<Vec<PromptTurn>, ContextBuilderError> {
        if let Some(appearance) = &character.appearance {
            return Self::build_character_prompt(
                CHAR_OUTFIT_PROMPT,
                adventure,
                character.name.as_str(),
                character_conditions,
                appearance,
                uncovered_messages,
            );
        }

        Err(ContextBuilderError::MissingAppearanceDescription)
    }

    pub fn build_pose_body_prompt(
        adventure: &Adventure,
        character: &Character,
        character_conditions: Vec<CharacterCondition>,
        uncovered_messages: &[Message],
    ) -> Result<Vec<PromptTurn>, ContextBuilderError> {
        if let Some(appearance) = &character.appearance {
            return Self::build_character_prompt(
                CHAR_POSE_BODY_PROMPT,
                adventure,
                character.name.as_str(),
                character_conditions,
                appearance,
                uncovered_messages,
            );
        }

        Err(ContextBuilderError::MissingAppearanceDescription)
    }

    /// The finished prompt for an image model: the framing the caller asked for,
    /// then every layer the character has. A portrait drops the pose, since the
    /// pose layer describes a whole-body action the crop would cut away.
    pub fn compose_character_image_prompt(
        character: &Character,
        framing: ImageFraming,
    ) -> Option<String> {
        let identity = character.image_identity()?;

        let framing_clause = match framing {
            ImageFraming::Portrait => CHAR_PORTRAIT_FRAMING,
            ImageFraming::FullBody => CHAR_FULL_BODY_FRAMING,
        };

        let mut layers = vec![framing_clause, identity];
        if let Some(outfit) = character.appearance_current_outfit.as_deref() {
            layers.push(outfit);
        }
        if framing == ImageFraming::FullBody
            && let Some(pose) = character.appearance_current_pose.as_deref()
        {
            layers.push(pose);
        }

        let prompt = layers
            .into_iter()
            .map(str::trim)
            .filter(|layer| !layer.is_empty())
            .collect::<Vec<_>>()
            .join(", ");

        (!prompt.is_empty()).then_some(prompt)
    }

    fn build_character_prompt(
        instruction: &str,
        adventure: &Adventure,
        character_name: &str,
        character_conditions: Vec<CharacterCondition>,
        appearance: &str,
        uncovered_messages: &[Message],
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

        if !messages_content.is_empty() {
            final_prompt.extend(messages_content);
        }

        Ok(final_prompt)
    }
}
