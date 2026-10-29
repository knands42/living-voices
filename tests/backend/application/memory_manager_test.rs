use sqlx::sqlite::SqlitePool;
use sqlx::{Row, query, query_scalar};
use uuid::Uuid;

use crate::backend::application::helper::{create_adventure, create_messages, setup_db};
use living_voices::ai::{ActClose, MemoryUpdate};
use living_voices::backend::application::memory_manager::MemoryManager;
use living_voices::backend::application::state::SharedState;
use living_voices::backend::domain::adventure::Adventure;
use living_voices::backend::domain::memory::{Act, ActStatus};
use living_voices::backend::persistence::act_persistence::get_active_act;
use living_voices::backend::persistence::message_persistence::fetch_since;
use living_voices::config::{Config, TextModel};

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

async fn seed_active_act(
    pool: &SqlitePool,
    adventure_id: Uuid,
    digest_to_seq: Option<u16>,
) -> Uuid {
    let act_id = Uuid::new_v4();
    let mut conn = pool.acquire().await.expect("acquire");
    query(
        "INSERT INTO act (id, adventure_id, ordinal, goal, status, digest_to_seq)
         VALUES (?1, ?2, 1, 'reach the surface', 'active', ?3)",
    )
    .bind(act_id.to_string())
    .bind(adventure_id.to_string())
    .bind(digest_to_seq.map(|s| s as i64))
    .execute(&mut *conn)
    .await
    .expect("insert act");
    act_id
}

async fn load_manager(
    pool: &SqlitePool,
    adventure: &Adventure,
) -> MemoryManager<MemoryUpdate, ActClose> {
    let state = SharedState::new();
    state
        .write()
        .expect("lock")
        .set_adventure(adventure.clone());
    MemoryManager::new(state, pool.clone(), distiller(), closer())
}

async fn seed_messages(pool: &SqlitePool, adventure_id: Uuid, count: u16) -> Vec<u16> {
    let authors = vec![None; count as usize];
    create_messages(pool, adventure_id, &authors).await
}

async fn seed_unstarted_act(pool: &SqlitePool, adventure_id: Uuid, ordinal: u16) -> Uuid {
    let act_id = Uuid::new_v4();
    let mut conn = pool.acquire().await.expect("acquire");
    query(
        "INSERT INTO act (id, adventure_id, ordinal, goal, status)
         VALUES (?1, ?2, ?3, 'the next chapter', 'unstarted')",
    )
    .bind(act_id.to_string())
    .bind(adventure_id.to_string())
    .bind(ordinal)
    .execute(&mut *conn)
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

async fn act_watermark(pool: &SqlitePool, act_id: Uuid) -> Option<i64> {
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

// ---------------------------------------------------------------- no-op paths (no model call)

#[tokio::test]
async fn compress_is_a_noop_without_a_current_adventure() {
    // given: a migrated database with no adventure loaded
    let (pool, _db) = setup_db().await;
    let manager = MemoryManager::new(SharedState::new(), pool.clone(), distiller(), closer());

    // when: compressing
    let result = manager.compress_and_update().await;

    // then: it succeeds without touching the database
    assert!(result.is_ok());
    assert_eq!(fact_count(&pool).await, 0);
}

#[tokio::test]
async fn compress_is_a_noop_without_an_active_act() {
    // given: an adventure with messages but no active act
    let (pool, _db) = setup_db().await;
    let adventure = create_adventure(&pool, &[]).await;
    seed_messages(&pool, adventure.id, 2).await;
    let manager = load_manager(&pool, &adventure).await;

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
    let adventure = create_adventure(&pool, &[]).await;
    let seqs = seed_messages(&pool, adventure.id, 2).await;
    let watermark = *seqs.last().expect("last");
    let act_id = seed_active_act(&pool, adventure.id, Some(watermark)).await;
    let manager = load_manager(&pool, &adventure).await;

    // when: compressing
    let result = manager.compress_and_update().await;

    // then: it succeeds without advancing the watermark or writing facts
    assert!(result.is_ok());
    assert_eq!(act_watermark(&pool, act_id).await, Some(watermark as i64));
    assert_eq!(fact_count(&pool).await, 0);
}

// ---------------------------------------------------------------- real model path

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn compress_distills_the_uncovered_tail_persists_memory_and_advances_the_watermark() {
    // given: three messages with the first already covered, so only seqs 2-3 distill
    let (pool, _db) = setup_db().await;
    let adventure = create_adventure(&pool, &[]).await;
    let seqs = seed_messages(&pool, adventure.id, 3).await;
    let act_id = seed_active_act(&pool, adventure.id, Some(seqs[0])).await;
    let manager = load_manager(&pool, &adventure).await;

    // when: compressing through the real memory model
    manager
        .compress_and_update()
        .await
        .expect("compress succeeds");

    let mut conn = pool.acquire().await.expect("acquire");

    // then: the watermark advances to the last seq and a running digest is written
    let row =
        query("SELECT digest_to_seq, running_digest, from_seq, to_seq FROM act WHERE id = ?1")
            .bind(act_id.to_string())
            .fetch_one(&mut *conn)
            .await
            .expect("act row");
    assert_eq!(
        row.get::<Option<i64>, _>("digest_to_seq"),
        Some(*seqs.last().expect("last seq") as i64)
    );
    assert!(
        !row.get::<String, _>("running_digest").trim().is_empty(),
        "running digest should be written"
    );

    // and: the immutable act span is never touched by compression
    assert_eq!(row.get::<Option<i64>, _>("from_seq"), None);
    assert_eq!(row.get::<Option<i64>, _>("to_seq"), None);

    // and: at least one fact is linked and a single live state row exists
    let fact_count: u16 = query_scalar("SELECT COUNT(*) FROM fact WHERE act_id = ?1")
        .bind(act_id.to_string())
        .fetch_one(&mut *conn)
        .await
        .expect("fact count");
    assert!(fact_count >= 1, "expected at least one fact");

    let state_count: u16 = query_scalar("SELECT COUNT(*) FROM state WHERE adventure_id = ?1")
        .bind(adventure.id.to_string())
        .fetch_one(&mut *conn)
        .await
        .expect("state count");
    assert_eq!(state_count, 1);
}

// ---------------------------------------------------------------- close current act

#[tokio::test]
async fn close_current_act_is_a_noop_without_a_current_adventure() {
    // given: a manager with no adventure loaded and a detached act
    let (pool, _db) = setup_db().await;
    let manager = MemoryManager::new(SharedState::new(), pool.clone(), distiller(), closer());
    let act = Act::new(Uuid::new_v4(), 1, "reach the surface");

    // when: closing the act
    let result = manager.close_current_act(&act, &[]).await;

    // then: it succeeds without writing memory
    assert!(result.is_ok());
    assert_eq!(fact_count(&pool).await, 0);
}

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn close_current_act_closes_the_act_and_activates_the_next_one() {
    // given: an active act with messages and an unstarted act waiting next
    let (pool, _db) = setup_db().await;
    let adventure = create_adventure(&pool, &[]).await;
    let seqs = seed_messages(&pool, adventure.id, 2).await;
    let act_id = seed_active_act(&pool, adventure.id, None).await;
    let next_act_id = seed_unstarted_act(&pool, adventure.id, 2).await;
    let manager = load_manager(&pool, &adventure).await;

    let mut conn = pool.acquire().await.expect("acquire");
    let act = get_active_act(&mut conn, adventure.id)
        .await
        .expect("read act")
        .expect("an active act");
    let uncovered = fetch_since(&mut conn, adventure.id, act.digest_to_seq)
        .await
        .expect("uncovered");
    drop(conn);

    // when: closing the act through the real memory model
    manager
        .close_current_act(&act, &uncovered)
        .await
        .expect("close succeeds");

    // then: the act reaches a terminal status with its watermark at the last seq
    assert_ne!(
        act_status(&pool, act_id).await,
        ActStatus::Active.to_string()
    );
    assert_eq!(
        act_watermark(&pool, act_id).await,
        Some(*seqs.last().expect("last seq") as i64)
    );

    // and: the next act becomes active
    assert_eq!(act_status(&pool, next_act_id).await, "active");

    // and: the close distills at least one fact
    let mut conn = pool.acquire().await.expect("acquire");
    let facts: u16 = query_scalar("SELECT COUNT(*) FROM fact WHERE act_id = ?1")
        .bind(act_id.to_string())
        .fetch_one(&mut *conn)
        .await
        .expect("fact count");
    assert!(facts >= 1, "expected at least one fact");
}
