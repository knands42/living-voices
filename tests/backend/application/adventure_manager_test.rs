use crate::backend::application::helper::setup_db;
use living_voices::backend::application::{
    adventure_manager::AdventureManager, state::SharedState,
};
use living_voices::backend::persistence::act_persistence::get_acts;
use living_voices::backend::persistence::state_persistence::{get_active_candidates, get_state};

#[tokio::test]
async fn full_pipeline_a_new_adventure_create_default_entities_and_initial_shared_state() {
    // when: creating an adventure
    let (pool, _db) = setup_db().await;
    let state = SharedState::new();
    let adventure_manager = AdventureManager::new(state.clone(), pool.clone());
    let adventure = adventure_manager
        .create_adventure(
            "Test adventure".into(),
            "A drowned city".into(),
            "tense".into(),
        )
        .await
        .expect("create adventure");

    // then: the adventure has an initial state
    let mut conn = pool.acquire().await.expect("acquire");
    let state_row = get_state(&mut conn, adventure.id)
        .await
        .expect("get")
        .expect("an adventure must have its initial state row");
    // TODO: Assertions

    let act_row = get_acts(&mut conn, adventure.id)
        .await
        .expect("get an act")
        .first()
        .expect("an adventure must have its initial act row");
    // TODO: Assertions

    // TODO: Assertions for lore, narrator, default player, facts

    // TODO: Assertions for shared state
}
