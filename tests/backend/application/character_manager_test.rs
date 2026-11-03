use crate::backend::application::helper::{TempDb, setup_db, start_adventure_with_default_cast};
use living_voices::ai::integrations::text::chat::Chat;
use living_voices::backend::application::{
    character_manager::CharacterManager, state::SharedState,
};
use living_voices::backend::domain::character::{Character, CharacterStatus};
use living_voices::backend::domain::message::Message;
use living_voices::backend::persistence::message_persistence::insert_message;
use living_voices::config::{Config, TextModel};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

const OLLAMA_URL: &str = "http://localhost:11434/v1";
const CHAT_MODEL: &str = "llama3.1:8b";

#[tokio::test]
async fn full_pipeline_create_character_persists_to_database_and_shared_state_and_state_table() {
    // given
    let (pool, _db) = setup_db().await;
    let state = SharedState::new();
    let character_manager = setup_character_manager(pool.clone(), state.clone());
    let adventure_manager = start_adventure_with_default_cast(&pool, &state).await;
    let adventure = adventure_manager
        .current_adventure()
        .expect("current adventure to be available");

    // when creating a new character
    let character = Character::new(
        Uuid::new_v4(),
        "Mara Venn".to_string(),
        adventure.id,
        "A tide-singer",
        "",
        CharacterStatus::Present,
        false,
    );

    character_manager
        .create_character(character.clone())
        .await
        .unwrap();

    // then: validate if it was persisted to database
    let (count, name): (u16, String) =
        sqlx::query_as("SELECT COUNT(*), name FROM character WHERE adventure_id = ?1")
            .bind(adventure.id.to_string())
            .fetch_one(&pool)
            .await
            .expect("query character");
    assert_eq!(count, 4);

    // then: validate if it was persisted to state
    let stored = stored_npcs(&state);
    assert_eq!(stored.len(), 3);

    // then: validate if it was persisted to state
    let row = sqlx::query("SELECT characters_present FROM state WHERE adventure_id = ?1 LIMIT 1")
        .bind(adventure.id.to_string())
        .fetch_one(&pool)
        .await
        .expect("query state");

    let chars: Vec<String> =
        serde_json::from_str(row.get::<String, _>("characters_present").as_str())
            .unwrap_or_default();
    chars
        .iter()
        .find(|&c| c == "Mara Venn")
        .expect("Mara Venn character to be present");
}

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn create_character_portrait_image_generates_and_persists_the_prompt() {
    let (manager, state, _db, pool) = new_integration_manager().await;
    let adventure_manager = start_adventure_with_default_cast(&pool, &state).await;
    let adventure = adventure_manager
        .current_adventure()
        .expect("current adventure to be available");

    let character = Character::new(
        Uuid::new_v4(),
        "Mara Venn".to_string(),
        adventure.id,
        "A tide-singer",
        "",
        CharacterStatus::Present,
        false,
    )
    .with_appearance("Copper hair, a salt-stained coat, grey eyes");

    manager.create_character(character.clone()).await.unwrap();
    manager
        .create_character_portrait_image(&character)
        .await
        .unwrap();

    let stored: Option<String> =
        sqlx::query_scalar("SELECT appearance_portrait_prompt FROM character WHERE id = ?1")
            .bind(character.id.to_string())
            .fetch_one(&pool)
            .await
            .expect("query character");
    assert!(
        stored
            .as_deref()
            .is_some_and(|prompt| !prompt.trim().is_empty()),
        "portrait prompt was not persisted"
    );

    let stored_character = stored_npcs(&state)
        .into_iter()
        .find(|stored| stored.id == character.id)
        .expect("character in state");
    assert!(
        stored_character
            .appearance_portrait_prompt
            .as_deref()
            .is_some_and(|prompt| !prompt.trim().is_empty()),
        "portrait prompt was not written to state"
    );
}

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn create_character_full_body_image_generates_and_persists_the_prompt() {
    let (manager, state, _db, pool) = new_integration_manager().await;
    let adventure_manager = start_adventure_with_default_cast(&pool, &state).await;
    let adventure = adventure_manager
        .current_adventure()
        .expect("current adventure to be available");

    let character = Character::new(
        Uuid::new_v4(),
        "Mara Venn".to_string(),
        adventure.id,
        "A tide-singer",
        "",
        CharacterStatus::Present,
        false,
    )
    .with_appearance("Copper hair, a salt-stained coat, grey eyes");

    manager.create_character(character.clone()).await.unwrap();
    manager
        .create_character_full_body_image(&character)
        .await
        .unwrap();

    let stored: Option<String> =
        sqlx::query_scalar("SELECT appearance_full_body_prompt FROM character WHERE id = ?1")
            .bind(character.id.to_string())
            .fetch_one(&pool)
            .await
            .expect("query character");
    assert!(
        stored
            .as_deref()
            .is_some_and(|prompt| !prompt.trim().is_empty()),
        "full-body prompt was not persisted"
    );

    let stored_character = stored_npcs(&state)
        .into_iter()
        .find(|stored| stored.id == character.id)
        .expect("character in state");
    assert!(
        stored_character
            .appearance_full_body_prompt
            .as_deref()
            .is_some_and(|prompt| !prompt.trim().is_empty()),
        "full-body prompt was not written to state"
    );
}

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn create_character_pose_image_uses_messages_and_persists_the_prompt() {
    let (manager, state, _db, pool) = new_integration_manager().await;
    let adventure_manager = start_adventure_with_default_cast(&pool, &state).await;
    let adventure = adventure_manager
        .current_adventure()
        .expect("current adventure to be available");

    let character = Character::new(
        Uuid::new_v4(),
        "Mara Venn".to_string(),
        adventure.id,
        "A tide-singer",
        "",
        CharacterStatus::Present,
        false,
    )
    .with_appearance("Copper hair, a salt-stained coat, grey eyes");

    manager.create_character(character.clone()).await.unwrap();

    let mut conn = pool.acquire().await.expect("acquire");
    let message = Message::new(
        adventure.id,
        Some(character.id),
        None,
        character.name.clone(),
        "*reaches up to trim the lamp wick*".to_string(),
        false,
    );
    insert_message(&mut conn, &message)
        .await
        .expect("insert message");
    drop(conn);

    manager
        .create_character_pose_image(&character)
        .await
        .unwrap();

    let stored: Option<String> =
        sqlx::query_scalar("SELECT appearance_current_pose FROM character WHERE id = ?1")
            .bind(character.id.to_string())
            .fetch_one(&pool)
            .await
            .expect("query character");
    assert!(
        stored
            .as_deref()
            .is_some_and(|prompt| !prompt.trim().is_empty()),
        "pose prompt was not persisted"
    );

    let stored_character = stored_npcs(&state)
        .into_iter()
        .find(|stored| stored.id == character.id)
        .expect("character in state");
    assert!(
        stored_character
            .appearance_current_pose
            .as_deref()
            .is_some_and(|prompt| !prompt.trim().is_empty()),
        "pose prompt was not written to state"
    );
}

// ---------------------------------------------------------------- fixtures

fn chat() -> Chat {
    Chat::new(TextModel {
        base_url: OLLAMA_URL.to_string(),
        model: CHAT_MODEL.to_string(),
        temperature: 0.5,
        request_timeout_secs: 120,
        keep_alive: "5m".to_string(),
    })
}

async fn new_integration_manager() -> (CharacterManager<Chat>, SharedState, TempDb, SqlitePool) {
    let state = SharedState::new();
    let (pool, db) = setup_db().await;
    let manager = CharacterManager::new(state.clone(), pool.clone(), chat());
    (manager, state, db, pool)
}

fn setup_character_manager(pool: SqlitePool, state: SharedState) -> CharacterManager<Chat> {
    let text_model_config = Config::new().expect("load config").settings.models.chat;
    let chat_handler = Chat::new(text_model_config);
    CharacterManager::new(state, pool, chat_handler)
}

/// The non-player cast currently held in shared state.
fn stored_npcs(state: &SharedState) -> Vec<Character> {
    state
        .read()
        .unwrap()
        .current_adventure
        .as_ref()
        .map(|a| {
            a.characters
                .iter()
                .filter(|c| !c.is_player)
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}
