use crate::backend::application::helper::{TempDb, create_adventure, setup_db};
use living_voices::backend::application::{
    adventure_manager::AdventureManager, state::SharedState,
};
use living_voices::backend::domain::adventure::Adventure;
use living_voices::backend::persistence::memory_persistence::{get_active_candidates, get_state};

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
