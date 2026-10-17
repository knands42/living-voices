use crate::ai::{
    client::OpenAiCompatibleClient,
    error::AiError,
    types::{ChatMessage, Sampling},
};

/// Generate the assistant's next line.
///
/// `system` is the character persona (identity, voice, what they will reveal);
/// `history` is the prepared conversation so far, most recent last; and
/// `user_message` is the new line to answer.
pub async fn respond(
    client: &OpenAiCompatibleClient,
    model: &str,
    system: &str,
    history: &[ChatMessage],
    user_message: &str,
) -> Result<String, AiError> {
    let mut messages = Vec::with_capacity(history.len() + 2);
    messages.push(ChatMessage::system(system));
    messages.extend_from_slice(history);
    messages.push(ChatMessage::user(user_message));

    client.request(model, &messages, Sampling::CHAT).await
}
