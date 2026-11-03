use crate::backend::application::helper::setup_db;
use living_voices::backend::application::{
    adventure_manager::{AdventureManager, AdventureManagerError},
    state::SharedState,
};
use living_voices::backend::domain::memory::{Act, ActStatus};
use living_voices::backend::persistence::act_persistence::get_acts;
use living_voices::backend::persistence::fact_persistence::get_facts;
use living_voices::backend::persistence::state_persistence::get_state;

#[tokio::test]
async fn create_adventure_creates_default_entities_and_initial_shared_state() {
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

    // then: the adventure carries what the caller passed in
    assert_eq!(adventure.title, "Test adventure");
    assert_eq!(adventure.headline, "A drowned city");
    assert_eq!(adventure.tone, "tense");

    // and: it has a default narrator
    assert_eq!(adventure.narrator.adventure_id, adventure.id);
    assert_eq!(adventure.narrator.name, "Narrator");
    assert!(!adventure.narrator.tone.trim().is_empty());

    // and: it has an empty default lore
    assert_eq!(adventure.lore.text, "");

    // and: the only character is the default player named "You"
    assert_eq!(adventure.characters.len(), 1);
    let player = &adventure.characters[0];
    assert_eq!(player.name, "You");
    assert!(player.is_player);

    let mut conn = pool.acquire().await.expect("acquire");

    // and: the adventure has its single initial state row with unknown defaults
    let state_row = get_state(&mut conn, adventure.id)
        .await
        .expect("get")
        .expect("an adventure must have its initial state row");
    assert_eq!(state_row.adventure_id, adventure.id);
    assert_eq!(state_row.day, "unknown");
    assert_eq!(state_row.place, "unknown");
    assert!(state_row.characters_present.is_empty());
    assert!(state_row.open_threads.is_empty());
    assert!(state_row.character_conditions.is_empty());

    // and: the adventure has exactly one initial act, unstarted and empty
    let acts = get_acts(&mut conn, adventure.id).await.expect("get acts");
    assert_eq!(acts.len(), 1);
    let act_row = acts
        .first()
        .expect("an adventure must have its initial act row");
    assert_eq!(act_row.adventure_id, adventure.id);
    assert_eq!(act_row.ordinal, 0);
    assert_eq!(act_row.goal, Act::DEFAULT_ACT_GOAL);
    assert_eq!(act_row.status, ActStatus::Unstarted);
    assert!(act_row.index_line.is_none());
    assert!(act_row.digest.is_none());
    assert!(act_row.running_digest.is_none());
    assert_eq!(act_row.from_seq, 0);
    assert_eq!(act_row.to_seq, 0);
    assert_eq!(act_row.last_closure_score, 0);

    // and: a fresh adventure carries no facts yet
    let facts = get_facts(&mut conn, adventure.id, act_row.id)
        .await
        .expect("get facts");
    assert!(facts.is_empty());

    // and: creating seeds the shared state with the same entities
    let snapshot = adventure_manager.current_snapshot();
    assert_eq!(
        snapshot.current_adventure.expect("adventure in state").id,
        adventure.id
    );
    assert_eq!(snapshot.current_act.expect("act in state").id, act_row.id);
    assert_eq!(
        snapshot.current_state.expect("state in state").id,
        state_row.id
    );
    assert_eq!(snapshot.player.expect("player in state").id, player.id);
    assert!(snapshot.current_facts.is_empty());
}

#[tokio::test]
async fn start_a_new_adventure_should_also_update_shared_state() {
    // given: creating an adventure
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

    // when: the initial act has a goal and the adventure is started
    let default_act = adventure_manager
        .current_snapshot()
        .current_act
        .expect("an act");

    let updated_act = Act {
        id: default_act.id,
        adventure_id: adventure.id,
        ordinal: default_act.ordinal,
        goal: "Reach the frozen tavern".to_string(),
        trigger: None,
        status: ActStatus::Active,
        index_line: default_act.index_line,
        digest: default_act.digest,
        running_digest: default_act.running_digest,
        digest_to_seq: default_act.digest_to_seq,
        from_seq: default_act.from_seq,
        to_seq: default_act.to_seq,
        last_closure_score: default_act.last_closure_score,
        created_at: default_act.created_at,
        updated_at: default_act.updated_at,
    };
    let _ = adventure_manager.upsert_act(&updated_act).await;

    adventure_manager
        .start_adventure(adventure.id)
        .await
        .expect("start adventure");

    // then: start repopulates the shared state from persisted entities
    let snapshot = adventure_manager.current_snapshot();
    assert_eq!(
        snapshot.current_adventure.expect("current adventure").id,
        adventure.id
    );
    let current_act = snapshot.current_act.expect("current act");
    assert_eq!(current_act.goal, "Reach the frozen tavern");
    assert_eq!(current_act.status, ActStatus::Active);
    assert!(snapshot.current_facts.is_empty());
}

#[tokio::test]
async fn start_a_new_adventure_without_an_act_goal() {
    // given: creating an adventure
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

    // when: the active act still has the default placeholder goal
    let error = adventure_manager
        .start_adventure(adventure.id)
        .await
        .expect_err("an adventure without an act goal must not start");

    // then: starting fails because the act has no real goal
    match error {
        AdventureManagerError::AdventureIncomplete(message) => {
            assert_eq!(message, "no active act found");
        }
        other => panic!("expected an incomplete adventure, got {other:?}"),
    }
}
