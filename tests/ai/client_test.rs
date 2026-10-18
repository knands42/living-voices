use living_voices::ai::jobs::memory;
use living_voices::ai::{
    ChatMessage, JsonSchema, OpenAiCompatibleClient, ResponseFormat, Sampling,
};
use serde_json::json;

const OLLAMA_URL: &str = "http://localhost:11434/v1";
const CHAT_MODEL: &str = "llama3.1:8b";
const MEMORY_MODEL: &str = "hf.co/sennaLLMLearner/qwen2.5-7b-memory-distiller:Q4_K_M";

fn client() -> OpenAiCompatibleClient {
    OpenAiCompatibleClient::new(OLLAMA_URL).expect("build client")
}

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn chat_model_answers_a_prompt() {
    let messages = [ChatMessage::user(
        "In one short sentence, greet a traveller.",
    )];

    let reply = client()
        .send(CHAT_MODEL, &messages, Sampling::CHAT, None)
        .await
        .expect("request");

    // The model's wording is its own business; we only assert the pipe works.
    assert!(!reply.trim().is_empty(), "model returned an empty reply");
}

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn request_json_returns_a_single_schema_shaped_object() {
    let messages = [ChatMessage::user("Describe a quiet harbour at dusk.")];
    let schema = JsonSchema {
        name: "harbour".to_string(),
        schema: json!({
            "type": "object",
            "properties": {
                "mood": { "type": "string" },
                "sounds": { "type": "array", "items": { "type": "string" } }
            },
            "required": ["mood", "sounds"],
            "additionalProperties": false
        }),
    };

    let format = ResponseFormat::JsonSchema {
        json_schema: &schema,
    };
    let reply = client()
        .send(CHAT_MODEL, &messages, Sampling::CHAT, Some(format))
        .await
        .expect("request_json");

    // A schema-constrained reply is a single JSON object, so a plain parse
    // succeeds where the unconstrained distiller needed salvaging.
    let value: serde_json::Value =
        serde_json::from_str(&reply).expect("schema-constrained reply should be one JSON object");
    assert!(value.get("mood").is_some(), "no `mood` in reply: {reply}");
    assert!(
        value.get("sounds").is_some(),
        "no `sounds` in reply: {reply}"
    );
}

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn memory_model_returns_a_memory_record() {
    let history = [
        ChatMessage::user("I am Mara, a tide-singer from the drowned quarter."),
        ChatMessage::assistant("Pleased to meet you, Mara."),
        ChatMessage::user("I hate the sea."),
    ];

    let update = memory::MemoryUpdate::distill(&client(), MEMORY_MODEL, &history)
        .await
        .expect("distill");

    // The distiller's wording is its own business; the contract is a record
    // carrying at least one fact and a summary.
    assert!(!update.facts.is_empty(), "expected at least one fact");
    assert!(!update.summary.trim().is_empty(), "expected a summary");
}
