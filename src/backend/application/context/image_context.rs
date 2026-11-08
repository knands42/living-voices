use crate::backend::application::context::prompts::{
    CHAR_CURRENT_BODY_PROMPT, CHAR_FULL_BODY_FRAMING, CHAR_OUTFIT_PROMPT, CHAR_PORTRAIT_FRAMING,
    CHAR_POSE_BODY_PROMPT,
};
use crate::backend::application::context::{CastTurn, ContextBuilder, ContextBuilderError};
use crate::backend::application::port::chat_port::PromptTurn;
use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::character::{Character, ImageFraming};
use crate::backend::domain::memory::CharacterCondition;
use crate::backend::domain::message::Message;

// ----------------------------------------------------------------------------
// Image prompt building
//
// A character image prompt is three layers that age at different speeds: the body
// (per act, and never when nothing changed), outfit (per scene) and pose (per beat).
// Each has its own model call and its own slot on the character; only
// `compose_character_image_prompt` knows how they fit together, so asking for a
// different shot never re-runs a model.
// ----------------------------------------------------------------------------
impl ContextBuilder {
    pub fn build_current_body_prompt(
        adventure: &Adventure,
        character: &Character,
        character_conditions: Vec<CharacterCondition>,
        uncovered_messages: &[Message],
    ) -> Result<Vec<PromptTurn>, ContextBuilderError> {
        if let Some(identity) = &character.appearance {
            return Self::build_character_prompt(
                CHAR_CURRENT_BODY_PROMPT,
                adventure,
                character.name.as_str(),
                character_conditions,
                identity,
                uncovered_messages,
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
        if !character.appearance_current_outfit.is_empty() {
            return Self::build_character_prompt(
                CHAR_OUTFIT_PROMPT,
                adventure,
                character.name.as_str(),
                character_conditions,
                &character.appearance_current_outfit,
                uncovered_messages,
            );
        }

        Err(ContextBuilderError::MissingOutfit)
    }

    pub fn build_pose_body_prompt(
        adventure: &Adventure,
        character: &Character,
        character_conditions: Vec<CharacterCondition>,
        uncovered_messages: &[Message],
    ) -> Result<Vec<PromptTurn>, ContextBuilderError> {
        let Some(look) = character.current_body_or_default() else {
            return Err(ContextBuilderError::MissingAppearanceDescription);
        };
        let current_look = format!("{look}, {}", character.appearance_current_outfit);

        Self::build_character_prompt(
            CHAR_POSE_BODY_PROMPT,
            adventure,
            character.name.as_str(),
            character_conditions,
            &current_look,
            uncovered_messages,
        )
    }

    /// The finished prompt for an image model + the framing the caller asked for. The
    /// original shot is drawn from the identity; every derived shot is drawn from the
    /// body the story left, so a wound earned in act two survives into the pose.
    pub fn compose_character_image_prompt(
        character: &Character,
        framing: ImageFraming,
    ) -> Option<String> {
        let look = match framing {
            ImageFraming::FullBody => character.identity(),
            ImageFraming::Portrait | ImageFraming::FullBodyPose => {
                character.current_body_or_default()
            }
        }?;

        let mut layers = Vec::with_capacity(4);
        match framing {
            ImageFraming::Portrait => layers.push(CHAR_PORTRAIT_FRAMING),
            ImageFraming::FullBody => layers.push(CHAR_FULL_BODY_FRAMING),
            ImageFraming::FullBodyPose => {}
        }
        layers.push(look);

        let outfit = character.appearance_current_outfit.trim();
        if !outfit.is_empty() {
            layers.push(outfit);
        }
        if framing == ImageFraming::FullBodyPose
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
        look: &str,
        uncovered_messages: &[Message],
    ) -> Result<Vec<PromptTurn>, ContextBuilderError> {
        let mut subject = vec![format!("## {}", character_name)];
        let mut messages_content: Vec<PromptTurn> = Vec::with_capacity(uncovered_messages.len());

        if !look.is_empty() {
            subject.push("### Character Look".to_string());
            subject.push(look.to_string());
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
            messages_content.extend(Self::build_previous_messages(
                adventure,
                uncovered_messages,
                CastTurn::Narrator,
            ));
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
