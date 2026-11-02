use living_voices::ai::integrations::text::chat::Chat;
use living_voices::backend::application::{
    adventure_manager::AdventureManager, character_manager::CharacterManager, state::SharedState,
};
use living_voices::backend::domain::character::{Character, CharacterStatus};
use living_voices::backend::domain::message::Message;
use living_voices::backend::persistence::message_persistence::insert_message;
use living_voices::config::TextModel;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::backend::application::helper::{StubPromptHandler, TempDb, setup_db};

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

async fn new_integration_manager() -> (CharacterManager<Chat>, SharedState, TempDb, SqlitePool) {
    let state = SharedState::new();
    let (pool, db) = setup_db().await;
    let manager = CharacterManager::new(state.clone(), pool.clone(), chat());
    (manager, state, db, pool)
}

async fn new_manager_with(
    state: SharedState,
) -> (CharacterManager<StubPromptHandler>, TempDb, SqlitePool) {
    let (pool, db) = setup_db().await;
    let manager = CharacterManager::new(state, pool.clone(), StubPromptHandler);
    (manager, db, pool)
}

async fn new_manager() -> (
    CharacterManager<StubPromptHandler>,
    SharedState,
    TempDb,
    SqlitePool,
) {
    let state = SharedState::new();
    let (manager, db, pool) = new_manager_with(state.clone()).await;
    (manager, state, db, pool)
}

/// Create an adventure into the given shared state so the CharacterManager
/// (which shares that state) has a current adventure to append cast to.
async fn persist_adventure(state: &SharedState, pool: &SqlitePool) -> Uuid {
    let mgr = AdventureManager::new(state.clone(), pool.clone());
    let adventure = mgr
        .create_adventure("Tides of Glass".into(), "A drowned city".into(), "".into())
        .await
        .expect("create adventure");
    adventure.id
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

#[tokio::test]
async fn create_character_stores_character_in_state() {
    let (manager, state, _db, pool) = new_manager().await;
    let adventure_id = persist_adventure(&state, &pool).await;

    let character = Character::new(
        Uuid::new_v4(),
        "Mara Venn".to_string(),
        adventure_id,
        "A tide-singer",
        "",
        CharacterStatus::Present,
        false,
    );

    manager.create_character(character.clone()).await.unwrap();

    let stored = stored_npcs(&state);
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].name, "Mara Venn");
    assert_eq!(stored[0].id, character.id);
}

#[tokio::test]
async fn create_character_appends_characters_to_state() {
    let (manager, state, _db, pool) = new_manager().await;
    let adventure_id = persist_adventure(&state, &pool).await;

    let first = Character::new(
        Uuid::new_v4(),
        "Mara".to_string(),
        adventure_id,
        "Singer",
        "",
        CharacterStatus::Present,
        false,
    );
    let second = Character::new(
        Uuid::new_v4(),
        "Otho".to_string(),
        adventure_id,
        "Diver",
        "",
        CharacterStatus::Present,
        false,
    );

    manager.create_character(first).await.unwrap();
    manager.create_character(second).await.unwrap();

    let stored = stored_npcs(&state);
    assert_eq!(stored.len(), 2);
    assert_eq!(stored[0].name, "Mara");
    assert_eq!(stored[1].name, "Otho");
}

#[tokio::test]
async fn state_is_shared_with_the_state_passed_to_new() {
    let state = SharedState::new();
    let (manager, _db, pool) = new_manager_with(state.clone()).await;
    let adventure_id = persist_adventure(&state, &pool).await;

    let character = Character::new(
        Uuid::new_v4(),
        "Mara Venn".to_string(),
        adventure_id,
        "A tide-singer",
        "",
        CharacterStatus::Present,
        false,
    );

    manager.create_character(character.clone()).await.unwrap();

    let stored = stored_npcs(&state);
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].id, character.id);
}

#[tokio::test]
async fn create_character_persists_character_to_database() {
    let (manager, state, _db, pool) = new_manager().await;
    let adventure_id = persist_adventure(&state, &pool).await;

    let character = Character::new(
        Uuid::new_v4(),
        "Mara Venn".to_string(),
        adventure_id,
        "A tide-singer",
        "",
        CharacterStatus::Present,
        false,
    );

    manager.create_character(character.clone()).await.unwrap();

    let (count, name): (u16, String) =
        sqlx::query_as("SELECT COUNT(*), name FROM character WHERE id = ?1")
            .bind(character.id.to_string())
            .fetch_one(&pool)
            .await
            .expect("query character");

    assert_eq!(count, 1);
    assert_eq!(name, "Mara Venn");
}

#[tokio::test]
async fn create_character_persists_traits_in_order() {
    let (manager, state, _db, pool) = new_manager().await;
    let adventure_id = persist_adventure(&state, &pool).await;

    let mut character = Character::new(
        Uuid::new_v4(),
        "Mara Venn".to_string(),
        adventure_id,
        "A tide-singer",
        "",
        CharacterStatus::Present,
        false,
    );
    character.add_trait("Speaks only in tidal registers".to_string());
    character.add_trait("Hums before speaking".to_string());

    manager.create_character(character.clone()).await.unwrap();

    let rows: Vec<(u16, String)> = sqlx::query_as(
        "SELECT position, value FROM character_trait WHERE character_id = ?1 ORDER BY position",
    )
    .bind(character.id.to_string())
    .fetch_all(&pool)
    .await
    .expect("query traits");

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], (0, "Speaks only in tidal registers".to_string()));
    assert_eq!(rows[1], (1, "Hums before speaking".to_string()));
}

#[tokio::test]
async fn create_character_persists_tags() {
    let (manager, state, _db, pool) = new_manager().await;
    let adventure_id = persist_adventure(&state, &pool).await;

    let mut character = Character::new(
        Uuid::new_v4(),
        "Mara Venn".to_string(),
        adventure_id,
        "A tide-singer",
        "",
        CharacterStatus::Present,
        false,
    );
    character.add_tag("Protagonist".to_string());
    character.add_tag("Brave".to_string());

    manager.create_character(character.clone()).await.unwrap();

    let rows: Vec<String> =
        sqlx::query_scalar("SELECT tag FROM character_tag WHERE character_id = ?1 ORDER BY tag")
            .bind(character.id.to_string())
            .fetch_all(&pool)
            .await
            .expect("query tags");

    assert_eq!(rows.len(), 2);
    // Tags are normalized (lowercased) before being stored.
    assert!(rows.contains(&"protagonist".to_string()));
    assert!(rows.contains(&"brave".to_string()));
}

#[tokio::test]
async fn create_character_persists_status_and_backstory() {
    let (manager, state, _db, pool) = new_manager().await;
    let adventure_id = persist_adventure(&state, &pool).await;

    let character = Character::new(
        Uuid::new_v4(),
        "Mara Venn".to_string(),
        adventure_id,
        "A tide-singer",
        "Raised in the drowned quarter",
        CharacterStatus::Away,
        false,
    );

    manager.create_character(character.clone()).await.unwrap();

    let (backstory, status): (String, String) =
        sqlx::query_as("SELECT backstory, status FROM character WHERE id = ?1")
            .bind(character.id.to_string())
            .fetch_one(&pool)
            .await
            .expect("query character");

    assert_eq!(backstory, "Raised in the drowned quarter");
    assert_eq!(status, "away");
}

#[tokio::test]
async fn create_character_stores_adventure_id() {
    let (manager, state, _db, pool) = new_manager().await;
    let adventure_id = persist_adventure(&state, &pool).await;

    let character = Character::new(
        Uuid::new_v4(),
        "Orin".to_string(),
        adventure_id,
        "A drifter",
        "",
        CharacterStatus::Present,
        false,
    );

    manager.create_character(character).await.unwrap();

    let stored = stored_npcs(&state);
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].name, "Orin");
    assert_eq!(stored[0].adventure_id, adventure_id);
}

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn create_character_portrait_image_generates_and_persists_the_prompt() {
    let (manager, state, _db, pool) = new_integration_manager().await;
    let adventure_id = persist_adventure(&state, &pool).await;

    let character = Character::new(
        Uuid::new_v4(),
        "Mara Venn".to_string(),
        adventure_id,
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
    let adventure_id = persist_adventure(&state, &pool).await;

    let character = Character::new(
        Uuid::new_v4(),
        "Mara Venn".to_string(),
        adventure_id,
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
    let adventure_id = persist_adventure(&state, &pool).await;

    let character = Character::new(
        Uuid::new_v4(),
        "Mara Venn".to_string(),
        adventure_id,
        "A tide-singer",
        "",
        CharacterStatus::Present,
        false,
    )
    .with_appearance("Copper hair, a salt-stained coat, grey eyes");

    manager.create_character(character.clone()).await.unwrap();

    let mut conn = pool.acquire().await.expect("acquire");
    let message = Message::new(
        adventure_id,
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
