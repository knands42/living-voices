use living_voices::ai::integrations::chat::Chat;
use living_voices::backend::application::port::chat_port::{ChatHandler, PromptTurn};
use living_voices::config::TextModel;

const OLLAMA_URL: &str = "http://localhost:11434/v1";
const CHAT_MODEL: &str = "llama3.1:8b";

fn chat() -> Chat {
    Chat::new(TextModel {
        base_url: OLLAMA_URL.to_string(),
        model: CHAT_MODEL.to_string(),
        temperature: 0.5,
        request_timeout_secs: 120,
        keep_alive: "5m".to_string(),
    })
}

/// A minimal roleplay prompt that ends on the player's turn, so the model has
/// something to answer.
fn prompt() -> Vec<PromptTurn> {
    vec![
        PromptTurn::system(
            "You are Mira, a diver exploring a drowned city. Answer in one short line, \
             in character.",
        ),
        PromptTurn::player("Mira, is the way ahead clear?"),
    ]
}

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn character_respond_returns_a_non_empty_reply() {
    let reply = chat()
        .character_respond(&prompt())
        .await
        .expect("character_respond");

    assert!(
        !reply.trim().is_empty(),
        "character_respond returned an empty reply"
    );
}

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn autonomous_respond_returns_a_replier_and_a_reply() {
    // The autonomous call is schema-constrained to `{ replier, reply }`; the
    // handler parses that JSON into a ChatReply, so both fields must be present.
    let reply = chat()
        .autonomous_respond(&prompt())
        .await
        .expect("autonomous_respond");

    assert!(
        !reply.replier.trim().is_empty(),
        "autonomous reply carried no replier"
    );
    assert!(
        !reply.reply.trim().is_empty(),
        "autonomous reply carried no body"
    );
}
