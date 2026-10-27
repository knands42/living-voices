use crate::backend::application::helper::{TempDb, create_adventure, setup_db};
use living_voices::backend::application::{
    adventure_manager::AdventureManager, state::SharedState,
};
use living_voices::backend::domain::adventure::Adventure;
use living_voices::backend::persistence::state_persistence::{get_active_candidates, get_state};

async fn new_manager_with(state: SharedState) -> (AdventureManager, TempDb) {
    let (pool, db) = setup_db().await;
    (AdventureManager::new(state, pool), db)
}

async fn new_manager() -> (AdventureManager, TempDb) {
    new_manager_with(SharedState::new()).await
}

#[tokio::test]
async fn adventure_insert_creates_default_state() {
    // when: creating an adventure
    let (pool, _db) = setup_db().await;
    let adventure = create_adventure(&pool, &[]).await;
    let mut conn = pool.acquire().await.expect("acquire");

    // then: the adventure has an initial state
    let state = get_state(&mut conn, adventure.id)
        .await
        .expect("get")
        .expect("an adventure must have its initial state row");

    assert_eq!(state.adventure_id, adventure.id);
    assert_eq!(state.day, String::from("unknown"));
    assert_eq!(state.place, String::from("unknown"));
    assert!(state.characters_present.is_empty());
    assert!(state.open_threads.is_empty());
    assert!(state.conditions.is_empty());

    // And active candidates read cleanly as an empty scene, not an error.
    let names = get_active_candidates(&mut conn, &adventure.id.to_string())
        .await
        .expect("candidates must read, not error, on a fresh adventure");
    assert!(names.is_empty());
}

#[tokio::test]
async fn adventure_insert_creates_default_narrator() {
    // when: creating an adventure
    let (pool, _db) = setup_db().await;
    let adventure = create_adventure(&pool, &[]).await;

    // then: exactly one narrator exists for it, matching the adventure's own narrator
    let (id, name, count): (String, String, u16) = sqlx::query_as(
        "SELECT id, name, (SELECT COUNT(*) FROM narrator WHERE adventure_id = ?1)
         FROM narrator WHERE adventure_id = ?1",
    )
    .bind(adventure.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("an adventure must have its default narrator row");

    assert_eq!(count, 1);
    assert_eq!(id, adventure.narrator.id.to_string());
    assert_eq!(name, adventure.narrator.name);
}

#[tokio::test]
async fn create_adventure_creates_a_default_player_character() {
    let (pool, _db) = setup_db().await;
    let manager = AdventureManager::new(SharedState::new(), pool.clone());

    let adventure = manager
        .create_adventure(
            "Tides".into(),
            "A drowned city".into(),
            "Guide".into(),
            "tense",
        )
        .await
        .expect("create adventure");

    // The returned adventure carries exactly one player character.
    let players: Vec<_> = adventure
        .characters
        .iter()
        .filter(|c| c.is_player)
        .collect();
    assert_eq!(players.len(), 1);
    assert_eq!(players[0].name, "You");

    // And it is persisted and flagged as the player in the DB.
    let (name, is_player, count): (String, bool, u16) = sqlx::query_as(
        "SELECT name, is_player,
                (SELECT COUNT(*) FROM character WHERE adventure_id = ?1 AND is_player = 1)
         FROM character WHERE adventure_id = ?1 AND is_player = 1",
    )
    .bind(adventure.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("a persisted player character must exist");

    assert_eq!(count, 1);
    assert!(is_player);
    assert_eq!(name, "You");
}
