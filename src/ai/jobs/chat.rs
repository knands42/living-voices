use crate::ai::{
    client::OpenAiCompatibleClient,
    error::AiError,
    types::{ChatMessage, Sampling},
};
use crate::backend::domain::message::Message;

/// Render a stored message as the conversation line handed to a model:
/// `[<speaker> <kind>] to [<target>]: <text>` per part, parts joined with ` + `,
/// e.g. `[Alice whisper] to [Bob]: come here + [Alice do]: raise the drawing`.
impl From<&Message> for ChatMessage {
    fn from(message: &Message) -> Self {
        let rendered = message
            .parts
            .iter()
            .map(|part| {
                let mut segment = format!("[{} {}]", message.speaker_name, part.kind);
                if let Some(target) = &part.target_name {
                    segment.push_str(&format!(" to [{target}]"));
                }
                segment.push_str(&format!(": {}", part.text));
                segment
            })
            .collect::<Vec<_>>()
            .join(" + ");
        ChatMessage::user(rendered)
    }
}

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

    client.send(model, &messages, Sampling::CHAT, None).await
}
