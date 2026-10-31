use living_voices::backend::application::{
    character_manager::CharacterManager, state::SharedState,
};
use living_voices::backend::domain::character::{Character, CharacterStatus};
use living_voices::backend::persistence::adventure_persistence::insert_adventure;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::backend::application::helper::{TempDb, setup_db};

async fn new_manager_with(state: SharedState) -> (CharacterManager, TempDb, SqlitePool) {
    let (pool, db) = setup_db().await;
    let manager = CharacterManager::new(state, pool.clone());
    (manager, db, pool)
}

async fn new_manager() -> (CharacterManager, SharedState, TempDb, SqlitePool) {
    let state = SharedState::new();
    let (manager, db, pool) = new_manager_with(state.clone()).await;
    (manager, state, db, pool)
}

async fn persist_adventure(pool: &SqlitePool) -> Uuid {
    let mut conn = pool.acquire().await.expect("acquire conn");
    insert_adventure(
        &mut conn,
        Uuid::new_v4(),
        "Tides of Glass",
        "A drowned city",
        "",
        vec![],
    )
    .await
    .expect("insert adventure")
    .id
}

#[tokio::test]
async fn create_character_stores_character_in_state() {
    let (manager, state, _db, pool) = new_manager().await;
    let adventure_id = persist_adventure(&pool).await;

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

    let stored = state.read().unwrap().characters.clone();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].name, "Mara Venn");
    assert_eq!(stored[0].id, character.id);
}

#[tokio::test]
async fn create_character_appends_characters_to_state() {
    let (manager, state, _db, pool) = new_manager().await;
    let adventure_id = persist_adventure(&pool).await;

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

    let stored = state.read().unwrap().characters.clone();
    assert_eq!(stored.len(), 2);
    assert_eq!(stored[0].name, "Mara");
    assert_eq!(stored[1].name, "Otho");
}

#[tokio::test]
async fn state_is_shared_with_the_state_passed_to_new() {
    let state = SharedState::new();
    let (manager, _db, pool) = new_manager_with(state.clone()).await;
    let adventure_id = persist_adventure(&pool).await;

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

    let stored = state.read().unwrap().characters.clone();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].id, character.id);
}

#[tokio::test]
async fn create_character_persists_character_to_database() {
    let (manager, _state, _db, pool) = new_manager().await;
    let adventure_id = persist_adventure(&pool).await;

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
    let (manager, _state, _db, pool) = new_manager().await;
    let adventure_id = persist_adventure(&pool).await;

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
    let (manager, _state, _db, pool) = new_manager().await;
    let adventure_id = persist_adventure(&pool).await;

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
    let (manager, _state, _db, pool) = new_manager().await;
    let adventure_id = persist_adventure(&pool).await;

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
    let adventure_id = persist_adventure(&pool).await;

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

    let stored = state.read().unwrap().characters.clone();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].name, "Orin");
    assert_eq!(stored[0].adventure_id, adventure_id);
}
