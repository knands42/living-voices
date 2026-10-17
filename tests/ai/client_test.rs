use living_voices::ai::jobs::memory;
use living_voices::ai::{ChatMessage, OpenAiCompatibleClient, Sampling};

const OLLAMA_URL: &str = "http://localhost:11434/v1";
const CHAT_MODEL: &str = "llama3.1:8b";
const MEMORY_MODEL: &str = "hf.co/sennaLLMLearner/qwen2.5-7b-memory-distiller:Q4_K_M";

#[tokio::test]
#[ignore = "requires Ollama serving llama3.1:8b"]
async fn chat_model_answers_a_prompt() {
    let client = OpenAiCompatibleClient::new(OLLAMA_URL).expect("build client");
    let messages = [ChatMessage::user(
        "In one short sentence, greet a traveller.",
    )];

    let reply = client
        .request(CHAT_MODEL, &messages, Sampling::CHAT)
        .await
        .expect("complete");

    // The model's wording is its own business; we only assert the pipe works.
    assert!(!reply.trim().is_empty(), "model returned an empty reply");
}

#[tokio::test]
#[ignore = "requires Ollama serving the memory distiller"]
async fn memory_model_returns_a_memory_record() {
    let client = OpenAiCompatibleClient::new(OLLAMA_URL).expect("build client");
    let history = [
        ChatMessage::user("I am Mara, a tide-singer from the drowned quarter."),
        ChatMessage::assistant("Pleased to meet you, Mara."),
        ChatMessage::user("I hate the sea."),
    ];

    let update = memory::distill(&client, MEMORY_MODEL, &history)
        .await
        .expect("distill");

    // The distiller's wording is its own business; the contract is a record
    // carrying at least one fact and a summary.
    assert!(!update.facts.is_empty(), "expected at least one fact");
    assert!(!update.summary.trim().is_empty(), "expected a summary");
}
