use serde::Serialize;

use crate::ai::{
    client::OpenAiCompatibleClient,
    types::{ChatMessage, Sampling},
};
use crate::backend::application::port::chat_port::{ChatHandler, ChatResponseError};

/// Spoken by a named character. The model IS that character
const CHARACTER_SYSTEM_PROMPT: &str = "\
You are a single character in a roleplay. Stay fully in character — speak only as
them, in their voice, revealing only what they would know. Reply with their next
line of dialogue or action as plain text — no narration of other characters, no
JSON, no quotation marks around the whole reply.";

/// Spoken by the narrator — scene, atmosphere, and the world's reactions.
const NARRATOR_SYSTEM_PROMPT: &str = "\
You are the narrator of a roleplay. Describe whatever happens next in the world (the scene, atmosphere, an event, etc).
Do not speak as the player's character. The next narration (or event) needs to be coherent with the past events, but it can also be surprising.
Reply with the narration as plain text — no JSON, no quotation marks around the whole
reply.";

const TONE_PROMPT: &str = "Match this tone: ";

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Chat {
    #[serde(skip)]
    base_url: String,
    #[serde(skip)]
    model: String,
}

impl Chat {
    pub fn new(base_url: String, model: String) -> Self {
        Self { base_url, model }
    }

    /// Shared call path: both voices differ only in the system prompt + tone.
    async fn speak(
        &self,
        system_prompt: &str,
        tone: &str,
        history: &[ChatMessage],
        user_message: &str,
    ) -> Result<String, ChatResponseError> {
        let client = OpenAiCompatibleClient::new(&self.base_url)
            .map_err(|e| ChatResponseError::Provider(e.to_string()))?;

        let mut messages = Vec::with_capacity(history.len() + 3);
        messages.push(ChatMessage::system(system_prompt));
        messages.push(ChatMessage::system(format!("{TONE_PROMPT}{tone}")));
        messages.extend_from_slice(history);
        messages.push(ChatMessage::user(user_message));

        client
            .send(self.model.as_str(), &messages, Sampling::CHAT, None)
            .await
            .map_err(|e| ChatResponseError::Provider(e.to_string()))
    }
}

impl ChatHandler for Chat {
    async fn character_respond(
        &self,
        character_tone: &str,
        history: &[ChatMessage],
        user_message: &str,
    ) -> Result<String, ChatResponseError> {
        self.speak(
            CHARACTER_SYSTEM_PROMPT,
            character_tone,
            history,
            user_message,
        )
        .await
    }

    async fn narrator_respond(
        &self,
        narrator_tone: &str,
        history: &[ChatMessage],
        user_message: &str,
    ) -> Result<String, ChatResponseError> {
        self.speak(NARRATOR_SYSTEM_PROMPT, narrator_tone, history, user_message)
            .await
    }
}
