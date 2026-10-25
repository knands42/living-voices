use living_voices::ai::integrations::narrator::Narrator;
use living_voices::backend::application::port::chat_port::{NarratorHandler, PromptTurn};
use living_voices::config::TextModel;

const OLLAMA_URL: &str = "http://localhost:11434/v1";
const CHAT_MODEL: &str = "llama3.1:8b";

fn narrator() -> Narrator {
    Narrator::new(TextModel {
        base_url: OLLAMA_URL.to_string(),
        model: CHAT_MODEL.to_string(),
        temperature: 0.5,
        request_timeout_secs: 120,
    })
}

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn narrator_respond_returns_a_non_empty_narration() {
    let context = [
        PromptTurn::system(
            "You are the narrator of an interactive roleplay. Narrate the world in one \
             or two short sentences; no character speaks.",
        ),
        PromptTurn::player("I step into the flooded hall."),
    ];

    let reply = narrator()
        .narrator_respond(&context)
        .await
        .expect("narrator_respond");

    // The narrator's wording is its own business; we only assert the pipe works
    // and a player turn actually draws narration back (an empty reply is the bug
    // class seen when the player turn is misrouted as the assistant's).
    assert!(
        !reply.trim().is_empty(),
        "narrator_respond returned an empty narration"
    );
}

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn narrator_respond_answers_the_latest_player_turn() {
    // A short exchange that ends on the player, so the narrator has a concrete
    // beat to answer rather than a dangling assistant turn.
    let context = [
        PromptTurn::system(
            "You are the narrator of an interactive roleplay. Narrate only; keep it to one \
             or two short sentences.",
        ),
        PromptTurn::player("Is anyone there?"),
        PromptTurn::cast("NARRATOR: Silence, then a drip echoing somewhere ahead."),
        PromptTurn::player("I follow the sound."),
    ];

    let reply = narrator()
        .narrator_respond(&context)
        .await
        .expect("narrator_respond");

    assert!(
        !reply.trim().is_empty(),
        "narrator_respond returned an empty narration for a player-ending prompt"
    );
}
