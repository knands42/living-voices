use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::ai::{error::AiError, types::ChatMessage};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JsonSchema {
    pub name: String,
    pub schema: serde_json::Value,
}

/// A client for any OpenAI-compatible `/chat/completions` endpoint.
#[derive(Clone, Debug)]
pub struct OpenAiCompatibleClient {
    http: reqwest::Client,
    base_url: String,
}

impl OpenAiCompatibleClient {
    /// Build a client with the default request timeout.
    pub fn new(base_url: impl Into<String>) -> Result<Self, AiError> {
        Self::with_timeout(base_url, REQUEST_TIMEOUT)
    }

    /// Build a client with an explicit request timeout.
    pub fn with_timeout(base_url: impl Into<String>, timeout: Duration) -> Result<Self, AiError> {
        let base_url = base_url.into();
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|source| AiError::Transport {
                url: base_url.clone(),
                source,
            })?;

        Ok(Self { http, base_url })
    }

    pub async fn send(
        &self,
        model: &str,
        messages: &[ChatMessage],
        temperature: f32,
        response_format: Option<ResponseFormat<'_>>,
        keep_alive: Option<&str>,
    ) -> Result<String, AiError> {
        let url = self.chat_completions_url();
        let body = ChatRequest {
            model,
            messages,
            temperature,
            stream: false,
            response_format,
            keep_alive,
        };

        let response = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|source| AiError::Transport {
                url: url.clone(),
                source,
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AiError::Status {
                url,
                status: status.as_u16(),
                body,
            });
        }

        let reply: ChatResponse = response
            .json()
            .await
            .map_err(|source| AiError::Decode { url, source })?;

        reply.into_content(model)
    }

    fn chat_completions_url(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    temperature: f32,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    keep_alive: Option<&'a str>,
}

/// Serialises to the OpenAI `response_format` wire shape.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseFormat<'a> {
    JsonSchema { json_schema: &'a JsonSchema },
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

impl ChatResponse {
    fn into_content(self, model: &str) -> Result<String, AiError> {
        let content = self
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .filter(|content| !content.is_empty())
            .ok_or_else(|| AiError::EmptyCompletion {
                model: model.to_string(),
            })?;

        Ok(content)
    }
}
