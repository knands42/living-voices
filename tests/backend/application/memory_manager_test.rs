use sqlx::query_scalar;
use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

use crate::backend::application::helper::{
    CONVERSATION, seed_conversation, setup_db, start_adventure_with_default_cast,
};
use living_voices::ai::{ActClose, MemoryUpdate};
use living_voices::backend::application::adventure_manager::AdventureManager;

use living_voices::backend::application::memory_manager::MemoryManager;
use living_voices::backend::application::state::SharedState;
use living_voices::backend::domain::memory::{Act, ActStatus};
use living_voices::backend::persistence::act_persistence::{
    get_active_act, update_act_running_digest, upsert_act,
};
use living_voices::config::{Config, MemorySettings, TextModel};

// ---------------------------------------------------------------- compress and distill path
#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn full_pipeline_compress_distills_the_uncovered_tail_persists_memory_and_advances_the_watermark()
 {
    // given: a 3 messages with the first already covered by distill
    let (pool, _db) = setup_db().await;
    let state = SharedState::new();
    let adventure_manager = start_adventure_with_default_cast(&pool, &state).await;
    let adventure = adventure_manager
        .current_adventure()
        .expect("current adventure");
    let memory_manager = setup_memory_manager(&state, &pool);
    let seqs = seed_conversation(&pool, &adventure, CONVERSATION, 3).await;

    // Set the act's digest_to_seq so the second message is already covered
    let mut conn = pool.acquire().await.expect("acquire");
    let active_act = get_active_act(&mut conn, adventure.id)
        .await
        .expect("get active act")
        .expect("has active act");
    update_act_running_digest(&mut conn, active_act.id, "", 1)
        .await
        .expect("update digest_to_seq");
    drop(conn);

    // when: compressing through the real memory model
    memory_manager
        .compress_and_update()
        .await
        .expect("compress succeeds");

    // then: the watermark advances to the last seq and a running digest is written
    assert_eq!(
        act_checkpoint(&pool, active_act.id).await,
        Some(*seqs.last().expect("last seq") as i64)
    );
    assert!(
        !act_running_digest(&pool, active_act.id)
            .await
            .trim()
            .is_empty(),
        "running digest should be written"
    );

    // and: the immutable act span is never touched by compression
    assert_eq!(act_from_seq(&pool, active_act.id).await, 0);
    assert_eq!(act_to_seq(&pool, active_act.id).await, 0);

    // and: at least one fact is linked and a single live state row exists
    assert!(
        fact_count_for_act(&pool, active_act.id).await >= 1,
        "expected at least one fact"
    );
    assert_eq!(state_count(&pool, adventure.id).await, 1);
}

#[tokio::test]
async fn compress_is_a_noop_without_a_current_adventure() {
    // given: a migrated database with no adventure loaded
    let (pool, _db) = setup_db().await;
    let manager = setup_memory_manager(&SharedState::new(), &pool);

    // when: compressing
    let result = manager.compress_and_update().await;

    // then: it succeeds without touching the database
    assert!(result.is_ok());
    assert_eq!(fact_count(&pool).await, 0);
}

#[tokio::test]
async fn compress_is_a_noop_without_an_active_act() {
    // given: an adventure with messages but no active act (not started)
    // built inline, not via the shared helper, because that one starts the act
    let (pool, _db) = setup_db().await;
    let state = SharedState::new();
    let mgr = AdventureManager::new(state.clone(), pool.clone());
    let adventure = mgr
        .create_adventure(
            "Test adventure".into(),
            "A drowned city".into(),
            "tense".into(),
        )
        .await
        .expect("create adventure");
    // Intentionally NOT calling start_adventure — no active act
    seed_conversation(&pool, &adventure, CONVERSATION, 2).await;
    let manager = setup_memory_manager(&SharedState::new(), &pool);

    // when: compressing
    let result = manager.compress_and_update().await;

    // then: it succeeds without writing memory
    assert!(result.is_ok());
    assert_eq!(fact_count(&pool).await, 0);
}

#[tokio::test]
async fn compress_is_a_noop_when_every_message_is_already_covered() {
    // given: two messages whose seqs are already below the act watermark
    let (pool, _db) = setup_db().await;
    let state = SharedState::new();
    let mgr = start_adventure_with_default_cast(&pool, &state).await;
    let adventure = mgr.current_adventure().expect("current adventure");

    let seqs = seed_conversation(&pool, &adventure, CONVERSATION, 2).await;
    let watermark = *seqs.last().expect("last");

    // Set the active act's digest_to_seq so all messages are already covered
    let mut conn = pool.acquire().await.expect("acquire");
    let active_act = get_active_act(&mut conn, adventure.id)
        .await
        .expect("get active act")
        .expect("has active act");
    update_act_running_digest(&mut conn, active_act.id, "", watermark)
        .await
        .expect("update digest_to_seq");
    drop(conn);
    let act_id = active_act.id;

    let manager = setup_memory_manager(&SharedState::new(), &pool);

    // when: compressing
    let result = manager.compress_and_update().await;

    // then: it succeeds without advancing the watermark or writing facts
    assert!(result.is_ok());
    assert_eq!(act_checkpoint(&pool, act_id).await, Some(watermark as i64));
    assert_eq!(fact_count(&pool).await, 0);
}

// ---------------------------------------------------------------- close current act

#[tokio::test]
async fn close_current_act_is_a_noop_without_a_current_adventure() {
    // given: a manager with no adventure loaded and a detached act
    let (pool, _db) = setup_db().await;
    let memory_manager = setup_memory_manager(&SharedState::new(), &pool);
    let act = Act::new(Uuid::new_v4(), 1, "reach the surface");

    // when: closing the act
    let result = memory_manager.close_current_act(&act, &[]).await;

    // then: it succeeds without writing memory
    assert!(result.is_ok());
    assert_eq!(fact_count(&pool).await, 0);
}

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn full_pipeline_close_current_act_and_activates_the_next_one() {
    // given: an active act with messages and an unstarted act waiting next
    let (pool, _db) = setup_db().await;
    let state = SharedState::new();
    let adventure_manager = start_adventure_with_default_cast(&pool, &state).await;
    let adventure = adventure_manager
        .current_adventure()
        .expect("current adventure");
    let memory_manager = setup_memory_manager(&state, &pool);
    let seqs = seed_conversation(&pool, &adventure, CONVERSATION, 3).await;

    let mut conn = pool.acquire().await.expect("acquire");
    let first_act = get_active_act(&mut conn, adventure.id)
        .await
        .expect("read act")
        .expect("an active act");
    let uncovered = {
        use living_voices::backend::persistence::message_persistence::fetch_since;
        fetch_since(&mut conn, adventure.id, first_act.digest_to_seq)
            .await
            .expect("uncovered")
    };

    // when: closing the act through the real memory model
    memory_manager
        .close_current_act(&first_act, &uncovered)
        .await
        .expect("close succeeds");

    let new_active_act = get_active_act(&mut conn, adventure.id)
        .await
        .expect("to query for an active act")
        .expect("an active act");

    drop(conn);

    assert_ne!(new_active_act.id, first_act.id);

    // then: the act reaches a terminal status with its checkpoint at the last seq
    assert_ne!(
        act_status(&pool, first_act.id).await,
        ActStatus::Active.to_string()
    );
    assert_eq!(
        act_checkpoint(&pool, first_act.id).await,
        Some(*seqs.last().expect("last seq") as i64)
    );

    // and: the next act becomes active
    assert_eq!(act_status(&pool, new_active_act.id).await, "active");

    // and: the close distills at least one fact
    assert!(
        fact_count_for_act(&pool, first_act.id).await >= 1,
        "expected at least one fact"
    );

    // and: the shared state is also updated
    assert_eq!(
        adventure_manager
            .current_snapshot()
            .current_act
            .expect("current act")
            .id,
        new_active_act.id
    );
}

// ---------------------------------------------------------------- helpers

fn memory_model() -> TextModel {
    let config = Config::new().expect("load config");
    config.settings.models.memory
}

fn distiller() -> MemoryUpdate {
    MemoryUpdate::new(memory_model())
}

fn closer() -> ActClose {
    ActClose::new(memory_model())
}

fn setup_memory_manager(
    state: &SharedState,
    pool: &SqlitePool,
) -> MemoryManager<MemoryUpdate, ActClose> {
    MemoryManager::new(
        state.clone(),
        pool.clone(),
        distiller(),
        closer(),
        MemorySettings::default(),
    )
}

async fn seed_unstarted_act(pool: &SqlitePool, adventure_id: Uuid, ordinal: u16) -> Uuid {
    let act = Act::new(adventure_id, ordinal, "the next chapter").with_status(ActStatus::Unstarted);
    let act_id = act.id;
    let mut conn = pool.acquire().await.expect("acquire");
    upsert_act(&mut conn, &act)
        .await
        .expect("insert unstarted act");
    act_id
}

async fn fact_count(pool: &SqlitePool) -> u16 {
    let mut conn = pool.acquire().await.expect("acquire");
    query_scalar("SELECT COUNT(*) FROM fact")
        .fetch_one(&mut *conn)
        .await
        .expect("count facts")
}

async fn fact_count_for_act(pool: &SqlitePool, act_id: Uuid) -> u16 {
    let mut conn = pool.acquire().await.expect("acquire");
    query_scalar("SELECT COUNT(*) FROM fact WHERE act_id = ?1")
        .bind(act_id.to_string())
        .fetch_one(&mut *conn)
        .await
        .expect("count facts for act")
}

async fn state_count(pool: &SqlitePool, adventure_id: Uuid) -> u16 {
    let mut conn = pool.acquire().await.expect("acquire");
    query_scalar("SELECT COUNT(*) FROM state WHERE adventure_id = ?1")
        .bind(adventure_id.to_string())
        .fetch_one(&mut *conn)
        .await
        .expect("count state rows")
}

async fn act_running_digest(pool: &SqlitePool, act_id: Uuid) -> String {
    let mut conn = pool.acquire().await.expect("acquire");
    query_scalar::<_, Option<String>>("SELECT running_digest FROM act WHERE id = ?1")
        .bind(act_id.to_string())
        .fetch_one(&mut *conn)
        .await
        .expect("read running digest")
        .unwrap_or_default()
}

async fn act_from_seq(pool: &SqlitePool, act_id: Uuid) -> i64 {
    let mut conn = pool.acquire().await.expect("acquire");
    query_scalar("SELECT from_seq FROM act WHERE id = ?1")
        .bind(act_id.to_string())
        .fetch_one(&mut *conn)
        .await
        .expect("read from_seq")
}

async fn act_to_seq(pool: &SqlitePool, act_id: Uuid) -> i64 {
    let mut conn = pool.acquire().await.expect("acquire");
    query_scalar("SELECT to_seq FROM act WHERE id = ?1")
        .bind(act_id.to_string())
        .fetch_one(&mut *conn)
        .await
        .expect("read to_seq")
}

async fn act_checkpoint(pool: &SqlitePool, act_id: Uuid) -> Option<i64> {
    let mut conn = pool.acquire().await.expect("acquire");
    query_scalar("SELECT digest_to_seq FROM act WHERE id = ?1")
        .bind(act_id.to_string())
        .fetch_one(&mut *conn)
        .await
        .expect("read watermark")
}

async fn act_status(pool: &SqlitePool, act_id: Uuid) -> String {
    let mut conn = pool.acquire().await.expect("acquire");
    query_scalar("SELECT status FROM act WHERE id = ?1")
        .bind(act_id.to_string())
        .fetch_one(&mut *conn)
        .await
        .expect("read status")
}
