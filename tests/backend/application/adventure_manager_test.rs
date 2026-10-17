use crate::backend::application::helper::TempDb;
use living_voices::backend::application::{
    adventure_manager::AdventureManager, state::SharedState,
};
use living_voices::backend::{open_pool, run_migrations};
use uuid::Uuid;

async fn new_manager_with(state: SharedState) -> (AdventureManager, TempDb) {
    let path = std::env::temp_dir().join(format!("living_voices_test_{}.db", Uuid::new_v4()));
    let db = TempDb(path);
    let path_str = db.0.to_string_lossy().into_owned();

    let migrating = open_pool(&path_str).await.expect("open pool");
    run_migrations(migrating).await.expect("run migrations");

    let pool = open_pool(&path_str).await.expect("reopen pool");
    (AdventureManager::new(state, pool), db)
}

async fn new_manager() -> (AdventureManager, TempDb) {
    new_manager_with(SharedState::new()).await
}

#[tokio::test]
async fn create_adventure_returns_adventure_with_given_fields() {
    let (manager, _db) = new_manager().await;

    let adventure = manager
        .create_adventure("Tides of Glass".to_string(), "A drowned city".to_string(), String::from("NARRATOR"))
        .await
        .unwrap();

    assert_eq!(adventure.title, "Tides of Glass");
    assert_eq!(adventure.description, "A drowned city");
}

#[tokio::test]
async fn create_adventure_stores_adventure_in_state() {
    let (manager, _db) = new_manager().await;

    let adventure = manager
        .create_adventure("Tides of Glass".to_string(), "A drowned city".to_string(), String::from("NARRATOR"))
        .await
        .unwrap();

    let stored = manager.current_adventure();
    assert_eq!(stored.map(|a| a.id), Some(adventure.id));
}

#[tokio::test]
async fn create_adventure_replaces_previous_adventure_in_state() {
    let (manager, _db) = new_manager().await;

    let first = manager
        .create_adventure("First".to_string(), "One".to_string(), String::from("NARRATOR"))
        .await
        .unwrap();
    let second = manager
        .create_adventure("Second".to_string(), "Two".to_string(), String::from("NARRATOR"))
        .await
        .unwrap();

    let stored = manager.current_adventure();
    assert_eq!(stored.map(|a| a.id), Some(second.id));
    assert_ne!(first.id, second.id);
}

#[tokio::test]
async fn create_adventure_generates_unique_ids_across_calls() {
    let (manager, _db) = new_manager().await;

    let first = manager
        .create_adventure("First".to_string(), "One".to_string(), String::from("NARRATOR"))
        .await
        .unwrap();
    let second = manager
        .create_adventure("Second".to_string(), "Two".to_string(), String::from("NARRATOR"))
        .await
        .unwrap();

    assert_ne!(first.id, second.id);
}

#[tokio::test]
async fn state_is_shared_with_the_state_passed_to_new() {
    let state = SharedState::new();
    let (manager, _db) = new_manager_with(state.clone()).await;

    let adventure = manager
        .create_adventure("Tides of Glass".to_string(), "Drowned".to_string(), String::from("NARRATOR"))
        .await
        .unwrap();

    assert_eq!(
        state
            .read()
            .unwrap()
            .current_adventure
            .as_ref()
            .map(|a| a.id),
        Some(adventure.id)
    );
}

#[tokio::test]
async fn create_adventure_leaves_stored_adventure_in_shared_state() {
    let (manager, _db) = new_manager().await;

    let created = manager
        .create_adventure("Tides of Glass".to_string(), "A drowned city".to_string(), String::from("NARRATOR"))
        .await
        .unwrap();

    assert_eq!(created.lore.text, "");
    assert!(created.characters.is_empty());
}
