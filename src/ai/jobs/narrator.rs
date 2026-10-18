use serde::Serialize;

use crate::ai::{client::OpenAiCompatibleClient, types::ChatMessage};
use crate::backend::application::port::chat_port::{ChatReply, ChatResponseError, NarratorHandler};
use crate::config::TextModel;

const NARRATOR_LABEL: &str = "narrator";

const NARRATOR_SYSTEM_PROMPT: &str = "\
You are the narrator of a roleplay. Describe whatever happens next in the world (the scene, atmosphere, an event, etc).
Do not speak as the player's character. The next narration (or event) needs to be coherent with the past events, but it can also be surprising.
Reply with the narration as plain text — no JSON, no quotation marks around the whole
reply.";

const NARRATOR_TONE_PROMPT: &str = "Match this tone: ";

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Narrator {
    #[serde(skip)]
    config: TextModel,
}

impl Narrator {
    pub fn new(config: TextModel) -> Self {
        Self { config }
    }

    fn client(&self) -> Result<OpenAiCompatibleClient, ChatResponseError> {
        OpenAiCompatibleClient::with_timeout(
            &self.config.base_url,
            std::time::Duration::from_secs(self.config.request_timeout_secs),
        )
        .map_err(|e| ChatResponseError::Provider(e.to_string()))
    }
}

impl NarratorHandler for Narrator {
    async fn narrator_respond(
        &self,
        narrator_tone: &str,
        memory: &str,
        user_message: &str,
    ) -> Result<ChatReply, ChatResponseError> {
        let client = self.client()?;

        let messages = [
            ChatMessage::system(NARRATOR_SYSTEM_PROMPT),
            ChatMessage::system(format!("{NARRATOR_TONE_PROMPT}{narrator_tone}")),
            ChatMessage::system(memory),
            ChatMessage::user(user_message),
        ];

        let reply = client
            .send(
                self.config.model.as_str(),
                &messages,
                self.config.temperature,
                None,
            )
            .await
            .map_err(|e| ChatResponseError::Provider(e.to_string()))?;

        Ok(ChatReply {
            replier: NARRATOR_LABEL.to_string(),
            reply,
        })
    }
}
