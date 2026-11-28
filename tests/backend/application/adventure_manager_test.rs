use crate::backend::application::helper::setup_db;
use living_voices::backend::application::{
    adventure_manager::{AdventureManager, AdventureManagerError},
    state::SharedState,
};
use living_voices::backend::domain::adventure::AdventureStatus;
use living_voices::backend::domain::memory::{Act, ActStatus, Fact};
use living_voices::backend::domain::message::Message;
use living_voices::backend::persistence::act_persistence::get_acts;
use living_voices::backend::persistence::adventure_persistence::get_adventure;
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
    assert!(!adventure.narrator.instructions.trim().is_empty());

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
    let facts = get_facts(&mut conn, adventure.id).await.expect("get facts");
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
async fn create_adventure_starts_as_a_draft() {
    let (pool, _db) = setup_db().await;
    let state = SharedState::new();
    let adventure_manager = AdventureManager::new(state.clone(), pool.clone());

    let adventure = adventure_manager
        .create_adventure("Draft".into(), "A new tale".into(), "tense".into())
        .await
        .expect("create adventure");

    assert_eq!(adventure.status, AdventureStatus::Draft);
}

#[tokio::test]
async fn update_status_persists_and_refreshes_shared_state() {
    // given: a freshly created draft adventure held in shared state
    let (pool, _db) = setup_db().await;
    let state = SharedState::new();
    let adventure_manager = AdventureManager::new(state.clone(), pool.clone());
    let adventure = adventure_manager
        .create_adventure("Draft".into(), "A new tale".into(), "tense".into())
        .await
        .expect("create adventure");

    // when: publishing it
    adventure_manager
        .update_status(adventure.id, AdventureStatus::Public)
        .await
        .expect("update status");

    // then: the change is persisted
    let mut conn = pool.acquire().await.expect("acquire");
    let reloaded = get_adventure(&mut conn, adventure.id)
        .await
        .expect("reload");
    assert_eq!(reloaded.status, AdventureStatus::Public);

    // and: the in-memory current adventure reflects it too
    let snapshot = adventure_manager.current_snapshot();
    assert_eq!(
        snapshot
            .current_adventure
            .expect("current adventure")
            .status,
        AdventureStatus::Public
    );
}

#[tokio::test]
async fn update_adventure_overwrites_fields_narrator_and_lore() {
    // given: a draft adventure with its default narrator and empty lore
    let (pool, _db) = setup_db().await;
    let state = SharedState::new();
    let adventure_manager = AdventureManager::new(state.clone(), pool.clone());
    let mut adventure = adventure_manager
        .create_adventure("Old title".into(), "Old headline".into(), "calm".into())
        .await
        .expect("create adventure");

    // when: editing every owned field, including the narrator and the lore entity
    adventure.title = "New title".to_string();
    adventure.headline = "New headline".to_string();
    adventure.tone = "tense".to_string();
    adventure.status = AdventureStatus::Private;
    adventure.narrator.name = "The Tide".to_string();
    adventure.narrator.instructions = "ominous".to_string();
    adventure.lore.text = "The city drowned in a single night.".to_string();

    let updated = adventure_manager
        .update_adventure(&adventure)
        .await
        .expect("update adventure");

    // then: the returned adventure carries the edits
    assert_eq!(updated.title, "New title");
    assert_eq!(updated.headline, "New headline");
    assert_eq!(updated.tone, "tense");
    assert_eq!(updated.status, AdventureStatus::Private);
    assert_eq!(updated.narrator.name, "The Tide");
    assert_eq!(updated.narrator.instructions, "ominous");
    assert_eq!(updated.lore.text, "The city drowned in a single night.");

    // and: a fresh read from the database agrees
    let mut conn = pool.acquire().await.expect("acquire");
    let reloaded = get_adventure(&mut conn, adventure.id)
        .await
        .expect("reload");
    assert_eq!(reloaded.title, "New title");
    assert_eq!(reloaded.narrator.name, "The Tide");
    assert_eq!(reloaded.lore.text, "The city drowned in a single night.");
}

#[tokio::test]
async fn load_adventure_fills_shared_state_without_starting_the_adventure() {
    // given: a draft whose only act still has the placeholder goal
    let (pool, _db) = setup_db().await;
    let adventure_manager = AdventureManager::new(SharedState::new(), pool.clone());
    let adventure = adventure_manager
        .create_adventure("Draft".into(), "A new tale".into(), "tense".into())
        .await
        .expect("create adventure");

    // when: loading it into a fresh manager's state
    let other_manager = AdventureManager::new(SharedState::new(), pool.clone());
    let snapshot = other_manager
        .load_adventure(adventure.id)
        .await
        .expect("load adventure");

    // then: the adventure and its first act are in state
    assert_eq!(
        snapshot.current_adventure.expect("adventure").id,
        adventure.id
    );
    let loaded_act = snapshot.current_act.expect("first act");
    assert_eq!(loaded_act.ordinal, 0);

    // and: the act was not activated, in state or in the database
    assert_eq!(loaded_act.status, ActStatus::Unstarted);
    let mut conn = pool.acquire().await.expect("acquire");
    let acts = get_acts(&mut conn, adventure.id).await.expect("get acts");
    assert!(acts.iter().all(|act| act.status == ActStatus::Unstarted));
}

#[tokio::test]
async fn start_adventure_requires_every_act_to_have_a_goal() {
    // given: a draft with a valid first act and a second act left on the placeholder
    let (pool, _db) = setup_db().await;
    let adventure_manager = AdventureManager::new(SharedState::new(), pool.clone());
    let adventure = adventure_manager
        .create_adventure("Draft".into(), "A new tale".into(), "tense".into())
        .await
        .expect("create adventure");

    let mut first = adventure_manager
        .get_acts(adventure.id)
        .await
        .expect("acts")[0]
        .clone();
    first.goal = "Reach the frozen tavern".to_string();
    adventure_manager
        .upsert_act(&first)
        .await
        .expect("save first act");
    adventure_manager
        .upsert_act(&Act::new(adventure.id, 1, Act::DEFAULT_ACT_GOAL))
        .await
        .expect("save second act");

    // when: starting
    let error = adventure_manager
        .start_adventure(adventure.id)
        .await
        .expect_err("a placeholder act must block the start");

    // then: it is refused, and nothing was activated
    assert!(matches!(
        error,
        AdventureManagerError::AdventureIncomplete(_)
    ));
    let acts = adventure_manager
        .get_acts(adventure.id)
        .await
        .expect("acts");
    assert!(acts.iter().all(|act| act.status == ActStatus::Unstarted));

    // and: once the second act has a goal, starting activates the first act only
    let mut second = acts[1].clone();
    second.goal = "Bargain with the still crowd".to_string();
    adventure_manager
        .upsert_act(&second)
        .await
        .expect("fix second act");
    adventure_manager
        .start_adventure(adventure.id)
        .await
        .expect("start adventure");

    let acts = adventure_manager
        .get_acts(adventure.id)
        .await
        .expect("acts");
    assert_eq!(acts[0].status, ActStatus::Active);
    assert_eq!(acts[1].status, ActStatus::Unstarted);
}

#[tokio::test]
async fn save_opening_replaces_the_first_lines_and_keeps_who_spoke() {
    // given: an adventure with a narrator and a player
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
    let player = adventure.characters[0].clone();

    // when: the opening is saved with the narrator and then the player
    let lines = vec![
        Message::new(
            adventure.id,
            None,
            Some(adventure.narrator.id),
            "The tide came in.".to_string(),
            false,
        ),
        Message::new(
            adventure.id,
            Some(player.id),
            None,
            "Not again.".to_string(),
            false,
        ),
    ];
    adventure_manager
        .save_opening(adventure.id, &lines)
        .await
        .expect("save opening");

    // then: both lines are stored in the order they were written
    let messages = adventure_manager
        .get_messages(adventure.id, 10)
        .await
        .expect("messages");
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].text, "The tide came in.");
    assert_eq!(messages[0].narrator_id, Some(adventure.narrator.id));
    assert_eq!(messages[1].text, "Not again.");
    assert_eq!(messages[1].character_id, Some(player.id));

    // when: the opening is saved again with a single line
    adventure_manager
        .save_opening(adventure.id, &lines[..1])
        .await
        .expect("rewrite opening");

    // then: the old lines are gone, not appended to
    let messages = adventure_manager
        .get_messages(adventure.id, 10)
        .await
        .expect("messages");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].text, "The tide came in.");

    // and: once the adventure leaves the draft, the opening is frozen
    adventure_manager
        .update_status(adventure.id, AdventureStatus::Public)
        .await
        .expect("publish");
    let error = adventure_manager
        .save_opening(adventure.id, &lines)
        .await
        .expect_err("only a draft keeps its opening editable");
    assert!(matches!(error, AdventureManagerError::NotDraft(_)));
}

#[tokio::test]
async fn saving_the_scene_state_and_facts_refreshes_the_shared_state() {
    // given: an adventure whose state and facts only hold their defaults
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

    // when: the opening names where and when it begins, with one fact
    let mut scene = adventure_manager
        .get_state(adventure.id)
        .await
        .expect("state")
        .expect("an adventure always has a state row");
    scene.place = "the drowned harbour".to_string();
    scene.day = "the first night of the tide".to_string();
    let facts = vec![Fact::new("the harbour", "keeps the debts of the drowned")];
    adventure_manager
        .upsert_state(&scene)
        .await
        .expect("save state");
    adventure_manager
        .upsert_facts(adventure.id, &facts)
        .await
        .expect("save facts");

    // then: both are stored
    let stored_state = adventure_manager
        .get_state(adventure.id)
        .await
        .expect("state")
        .expect("state row");
    assert_eq!(stored_state.place, "the drowned harbour");
    assert_eq!(stored_state.day, "the first night of the tide");
    assert_eq!(
        adventure_manager
            .get_facts(adventure.id)
            .await
            .expect("facts"),
        facts
    );

    // and: the shared state the managers read from carries them too
    let snapshot = adventure_manager.current_snapshot();
    assert_eq!(
        snapshot.current_state.map(|scene| scene.place),
        Some("the drowned harbour".to_string())
    );
    assert_eq!(snapshot.current_facts, facts);
}

#[tokio::test]
async fn multiline_lore_reopens_without_breaking_the_cast_query() {
    // given: a draft whose lore the story editor saved across blank lines, quotes included
    let (pool, _db) = setup_db().await;
    let state = SharedState::new();
    let adventure_manager = AdventureManager::new(state.clone(), pool.clone());
    let mut adventure = adventure_manager
        .create_adventure(
            "Test adventure".into(),
            "A drowned city".into(),
            "tense".into(),
        )
        .await
        .expect("create adventure");
    adventure.lore.text =
        "The resistance calls it a warehouse.\n\nWaiting to be \"offered\".".to_string();
    adventure_manager
        .update_adventure(&adventure)
        .await
        .expect("save lore");

    // when: the builder opens it again
    let reopened = adventure_manager
        .get_adventure(adventure.id)
        .await
        .expect("reopen");

    // then: the lore comes back intact
    assert_eq!(reopened.lore.text, adventure.lore.text);
}

#[tokio::test]
async fn delete_act_removes_the_saved_act_and_leaves_the_rest_of_the_draft_alone() {
    // given: a draft with its initial act and a second one the builder added
    let (pool, _db) = setup_db().await;
    let adventure_manager = AdventureManager::new(SharedState::new(), pool.clone());
    let adventure = adventure_manager
        .create_adventure("Draft".into(), "A new tale".into(), "tense".into())
        .await
        .expect("create adventure");
    let second = Act::new(adventure.id, 1, "Bargain with the still crowd");
    adventure_manager
        .upsert_act(&second)
        .await
        .expect("save second act");

    // when: the builder removes it
    adventure_manager
        .delete_act(second.id)
        .await
        .expect("delete act");

    // then: only the initial act is still there, through the manager and the database
    let acts = adventure_manager
        .get_acts(adventure.id)
        .await
        .expect("acts");
    assert_eq!(acts.len(), 1);
    assert_eq!(acts[0].ordinal, 0);
    assert_ne!(acts[0].id, second.id);

    let mut conn = pool.acquire().await.expect("acquire");
    let persisted = get_acts(&mut conn, adventure.id).await.expect("get acts");
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].id, acts[0].id);

    // and: the draft still starts once its remaining act has a goal
    let mut remaining = acts[0].clone();
    remaining.goal = "Reach the frozen tavern".to_string();
    adventure_manager
        .upsert_act(&remaining)
        .await
        .expect("save remaining act");
    adventure_manager
        .start_adventure(adventure.id)
        .await
        .expect("a draft without the removed act is still startable");
}
