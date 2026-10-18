use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::ai::{
    client::{JsonSchema, OpenAiCompatibleClient, ResponseFormat},
    error::AiError,
    types::ChatMessage,
};
use crate::backend::application::port::chat_port::{
    Candidate, ChatHandler, ChatReply, ChatResponseError,
};
use crate::config::TextModel;

const NARRATOR_LABEL: &str = "narrator";

const NARRATOR_SYSTEM_PROMPT: &str = "\
You are the narrator of a roleplay. Describe whatever happens next in the world (the scene, atmosphere, an event, etc).
Do not speak as the player's character. The next narration (or event) needs to be coherent with the past events, but it can also be surprising.
Reply with the narration as plain text — no JSON, no quotation marks around the whole
reply.";

const NARRATOR_TONE_PROMPT: &str = "Match this tone: ";

const CHARACTER_SYSTEM_PROMPT: &str = "\
You are a single character in a roleplay. Stay fully in character — speak only as
them, in their voice, revealing only what they would know. Reply with their next
line of dialogue or action as plain text — no narration of other characters, no
JSON, no quotation marks around the whole reply.";

const AUTONOMOUS_SYSTEM_PROMPT: &str = "\
You direct one turn of a roleplay. From the cast listed below, choose who speaks
next — a character, or the narrator — then write their line in their own voice
and tone. You may let the narrator introduce a surprising but coherent event.
Reply with JSON only, exactly these two fields:
- \"replier\": the exact name of the speaker you chose, or \"narrator\".
- \"reply\": that speaker's line, as plain text.";

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Chat {
    #[serde(skip)]
    config: TextModel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ChatCompletion {
    replier: String,
    reply: String,
}

impl ChatCompletion {
    fn from_reply(raw: &str) -> Result<Self, AiError> {
        serde_json::from_str(raw).map_err(|_| AiError::MalformedMemory {
            reply: raw.to_string(),
        })
    }
}

impl Chat {
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

    /// Shared path for the two KNOWN-speaker calls
    async fn speak(
        &self,
        system_prompt: &str,
        context: &str,
        memory: &str,
        user_message: &str,
    ) -> Result<String, ChatResponseError> {
        let client = self.client()?;

        let mut messages = Vec::with_capacity(3);
        messages.push(ChatMessage::system(system_prompt));
        messages.push(ChatMessage::system(context));
        messages.push(ChatMessage::system(memory));
        messages.push(ChatMessage::user(user_message));

        client
            .send(
                self.config.model.as_str(),
                &messages,
                self.config.temperature,
                None,
            )
            .await
            .map_err(|e| ChatResponseError::Provider(e.to_string()))
    }
}

impl ChatHandler for Chat {
    async fn character_respond(
        &self,
        candidate: &Candidate,
        memory: &str,
        user_message: &str,
    ) -> Result<ChatReply, ChatResponseError> {
        let reply = self
            .speak(
                CHARACTER_SYSTEM_PROMPT,
                candidate.context.as_str(),
                memory,
                user_message,
            )
            .await?;
        Ok(ChatReply {
            replier: candidate.name.clone(),
            reply,
        })
    }

    async fn narrator_respond(
        &self,
        narrator_tone: &str,
        memory: &str,
        user_message: &str,
    ) -> Result<ChatReply, ChatResponseError> {
        let reply = self
            .speak(
                NARRATOR_SYSTEM_PROMPT,
                format!("{NARRATOR_TONE_PROMPT}{narrator_tone}").as_str(),
                memory,
                user_message,
            )
            .await?;
        Ok(ChatReply {
            replier: NARRATOR_LABEL.to_string(),
            reply,
        })
    }

    async fn autonomous_respond(
        &self,
        narrator_tone: &str,
        candidates: &[Candidate],
        memory: &str,
        user_message: &str,
    ) -> Result<ChatReply, ChatResponseError> {
        let client = self.client()?;

        let mut messages = Vec::with_capacity(3);
        messages.push(ChatMessage::system(AUTONOMOUS_SYSTEM_PROMPT));
        messages.push(ChatMessage::system(render_cast(narrator_tone, candidates)));
        messages.push(ChatMessage::system(memory));
        messages.push(ChatMessage::user(user_message));

        let format = ResponseFormat::JsonSchema {
            json_schema: &schema(),
        };

        let raw = client
            .send(
                self.config.model.as_str(),
                &messages,
                self.config.temperature,
                Some(format),
            )
            .await
            .map_err(|e| ChatResponseError::Provider(e.to_string()))?;

        let completion = ChatCompletion::from_reply(&raw)
            .map_err(|e| ChatResponseError::Provider(e.to_string()))?;

        Ok(ChatReply {
            replier: completion.replier,
            reply: completion.reply,
        })
    }
}

fn render_cast(narrator_tone: &str, cast: &[Candidate]) -> String {
    let mut roster = String::from("Cast and their tones:\n");
    roster.push_str(&format!("- \"{NARRATOR_LABEL}\": {narrator_tone}\n"));
    for c in cast {
        roster.push_str(&format!("- \"{}\": {}\n", c.name, c.context));
    }
    roster
}

/// JSON schema forcing `{ replier, reply }` for the autonomous call.
fn schema() -> JsonSchema {
    JsonSchema {
        name: "chat_reply".to_string(),
        schema: json!({
            "type": "object",
            "properties": {
                "replier": { "type": "string" },
                "reply": { "type": "string" }
            },
            "required": ["replier", "reply"],
            "additionalProperties": false
        }),
    }
}
