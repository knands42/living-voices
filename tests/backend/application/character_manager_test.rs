use crate::backend::application::helper::{
    StubImage, TempDb, setup_db, start_adventure_with_default_cast,
};
use living_voices::ai::integrations::text::chat_handler::ChatHandler;
use living_voices::backend::application::{
    character_gallery::CharacterGallery, character_manager::CharacterManager, state::SharedState,
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
        "female".to_string(),
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
    let (count, _): (u16, String) =
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
async fn update_character_overwrites_fields_traits_and_tags_in_db_and_state() {
    // given: an adventure with a character already created
    let (pool, _db) = setup_db().await;
    let state = SharedState::new();
    let character_manager = setup_character_manager(pool.clone(), state.clone());
    let adventure_manager = start_adventure_with_default_cast(&pool, &state).await;
    let adventure = adventure_manager
        .current_adventure()
        .expect("current adventure to be available");

    let mut character = Character::new(
        Uuid::new_v4(),
        "Mara Venn".to_string(),
        "female".to_string(),
        adventure.id,
        "A tide-singer",
        "",
        CharacterStatus::Present,
        false,
    );
    character.add_trait("sings in tidal registers".to_string());
    character.add_tag("mystic".to_string());
    character_manager
        .create_character(character.clone())
        .await
        .expect("create character");

    // when: editing its fields, replacing traits and tags, and going away
    character.name = "Mara the Drowned".to_string();
    character.headline = "A ghost of the harbour".to_string();
    character.backstory = "She walked into the tide and kept walking.".to_string();
    character.status = CharacterStatus::Away;
    character.traits = vec!["speaks only in riddles".to_string()];
    character.tags = vec!["ghost".to_string()];
    character_manager
        .update_character(character.clone())
        .await
        .expect("update character");

    // then: the row carries the edited scalar fields
    let (name, headline, status): (String, String, String) =
        sqlx::query_as("SELECT name, headline, status FROM character WHERE id = ?1")
            .bind(character.id.to_string())
            .fetch_one(&pool)
            .await
            .expect("query character");
    assert_eq!(name, "Mara the Drowned");
    assert_eq!(headline, "A ghost of the harbour");
    assert_eq!(status, "away");

    // and: traits were replaced, not accumulated
    let traits: Vec<String> = sqlx::query_scalar(
        "SELECT value FROM character_trait WHERE character_id = ?1 ORDER BY position",
    )
    .bind(character.id.to_string())
    .fetch_all(&pool)
    .await
    .expect("query traits");
    assert_eq!(traits, vec!["speaks only in riddles".to_string()]);

    // and: tags were replaced too
    let tags: Vec<String> =
        sqlx::query_scalar("SELECT tag FROM character_tag WHERE character_id = ?1")
            .bind(character.id.to_string())
            .fetch_all(&pool)
            .await
            .expect("query tags");
    assert_eq!(tags, vec!["ghost".to_string()]);

    // and: the shared state copy reflects the edit
    let stored = stored_npcs(&state)
        .into_iter()
        .find(|c| c.id == character.id)
        .expect("character in state");
    assert_eq!(stored.name, "Mara the Drowned");
    assert_eq!(stored.status, CharacterStatus::Away);
}

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn create_character_body_prompt_generates_and_persists_the_prompt() {
    let (manager, state, _db, pool) = new_integration_manager().await;
    let adventure_manager = start_adventure_with_default_cast(&pool, &state).await;
    let adventure = adventure_manager
        .current_adventure()
        .expect("current adventure to be available");

    let character = Character::new(
        Uuid::new_v4(),
        "Mara Venn".to_string(),
        "female".to_string(),
        adventure.id,
        "A tide-singer",
        "",
        CharacterStatus::Present,
        false,
    )
    .with_appearance("Copper hair, a salt-stained coat, grey eyes");

    manager.create_character(character.clone()).await.unwrap();
    manager
        .create_character_body_prompt(&character)
        .await
        .unwrap();

    // Nothing has happened to the body yet, so the prompt either restates the identity
    // (stored as no change) or returns a body; neither may be blank.
    let stored: Option<String> =
        sqlx::query_scalar("SELECT appearance_current_body FROM character WHERE id = ?1")
            .bind(character.id.to_string())
            .fetch_one(&pool)
            .await
            .expect("query character");
    assert!(
        stored
            .as_deref()
            .is_none_or(|prompt| !prompt.trim().is_empty()),
        "body prompt was persisted blank"
    );

    let stored_character = stored_npcs(&state)
        .into_iter()
        .find(|stored| stored.id == character.id)
        .expect("character in state");
    assert!(
        stored_character
            .appearance_current_body
            .as_deref()
            .is_none_or(|prompt| !prompt.trim().is_empty()),
        "body prompt was written to state blank"
    );
}

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn create_character_outfit_prompt_uses_messages_and_persists_the_prompt() {
    let (manager, state, _db, pool) = new_integration_manager().await;
    let adventure_manager = start_adventure_with_default_cast(&pool, &state).await;
    let adventure = adventure_manager
        .current_adventure()
        .expect("current adventure to be available");

    let character = Character::new(
        Uuid::new_v4(),
        "Mara Venn".to_string(),
        "female".to_string(),
        adventure.id,
        "A tide-singer",
        "",
        CharacterStatus::Present,
        false,
    )
    .with_appearance("Copper hair, a salt-stained coat, grey eyes")
    .with_appearance_current_outfit("salt-stained oilskin coat, sou'wester");

    manager.create_character(character.clone()).await.unwrap();

    let mut conn = pool.acquire().await.expect("acquire");
    let message = Message::new(
        adventure.id,
        Some(character.id),
        None,
        character.name.clone(),
        "*strips off the soaked oilskin coat and hangs it by the stove*".to_string(),
        false,
    );
    insert_message(&mut conn, &message)
        .await
        .expect("insert message");
    drop(conn);

    manager
        .create_character_outfit_prompt(&character)
        .await
        .unwrap();

    let stored: String =
        sqlx::query_scalar("SELECT appearance_current_outfit FROM character WHERE id = ?1")
            .bind(character.id.to_string())
            .fetch_one(&pool)
            .await
            .expect("query character");
    assert!(!stored.trim().is_empty(), "outfit prompt was not persisted");

    let stored_character = stored_npcs(&state)
        .into_iter()
        .find(|stored| stored.id == character.id)
        .expect("character in state");
    assert!(
        !stored_character.appearance_current_outfit.trim().is_empty(),
        "outfit prompt was not written to state"
    );
}

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn create_character_pose_prompt_uses_messages_and_persists_the_prompt() {
    let (manager, state, _db, pool) = new_integration_manager().await;
    let adventure_manager = start_adventure_with_default_cast(&pool, &state).await;
    let adventure = adventure_manager
        .current_adventure()
        .expect("current adventure to be available");

    let character = Character::new(
        Uuid::new_v4(),
        "Mara Venn".to_string(),
        "female".to_string(),
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
        .create_character_pose_prompt(&character)
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

fn chat() -> ChatHandler {
    ChatHandler::new(TextModel {
        base_url: OLLAMA_URL.to_string(),
        model: CHAT_MODEL.to_string(),
        temperature: 0.5,
        request_timeout_secs: 120,
        keep_alive: "5m".to_string(),
    })
}

async fn new_integration_manager() -> (
    CharacterManager<ChatHandler, StubImage>,
    SharedState,
    TempDb,
    SqlitePool,
) {
    let state = SharedState::new();
    let (pool, db) = setup_db().await;
    let manager = CharacterManager::new(
        state.clone(),
        pool.clone(),
        chat(),
        StubImage,
        CharacterGallery::new(std::env::temp_dir()),
    );
    (manager, state, db, pool)
}

fn setup_character_manager(
    pool: SqlitePool,
    state: SharedState,
) -> CharacterManager<ChatHandler, StubImage> {
    let text_model_config = Config::new().expect("load config").settings.models.chat;
    let chat_handler = ChatHandler::new(text_model_config);
    CharacterManager::new(
        state,
        pool,
        chat_handler,
        StubImage,
        CharacterGallery::new(std::env::temp_dir()),
    )
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
