use sqlx::sqlite::SqlitePool;
use sqlx::{Row, query, query_scalar};
use uuid::Uuid;

use crate::backend::application::helper::{create_adventure, create_messages, setup_db};
use living_voices::ai::{ActClose, MemoryUpdate};
use living_voices::backend::application::memory_manager::MemoryManager;
use living_voices::backend::application::state::SharedState;
use living_voices::backend::domain::adventure::Adventure;
use living_voices::config::{Config, TextModel};

// ---------------------------------------------------------------- helpers

fn memory_model() -> TextModel {
    let config = Config::new().expect("load config");
    config.settings.models.memory
}

/// Build the distiller the app wires in, from the configured memory model.
fn distiller() -> MemoryUpdate {
    MemoryUpdate::new(memory_model())
}

/// Build the act closer the app wires in, from the configured memory model.
fn closer() -> ActClose {
    ActClose::new(memory_model())
}

/// Seed an active act with an optional digest watermark and return its id.
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

// ---------------------------------------------------------------- no-op paths (no model call)

#[tokio::test]
async fn compress_is_a_noop_without_a_current_adventure() {
    // Given a migrated database and no adventure loaded in the state
    let (pool, _db) = setup_db().await;
    let manager = MemoryManager::new(SharedState::new(), pool.clone(), distiller(), closer());

    // When compressing
    let result = manager.compress_and_update().await;

    // Then it succeeds without touching the database
    assert!(result.is_ok());
    let mut conn = pool.acquire().await.expect("acquire");
    let facts: u16 = query_scalar("SELECT COUNT(*) FROM fact")
        .fetch_one(&mut *conn)
        .await
        .expect("count facts");
    assert_eq!(facts, 0);
}

#[tokio::test]
async fn compress_is_a_noop_without_an_active_act() {
    // Given an adventure with messages but no active act
    let (pool, _db) = setup_db().await;
    let adventure = create_adventure(&pool, &[]).await;
    seed_messages(&pool, adventure.id, 2).await;
    let manager = load_manager(&pool, &adventure).await;

    // When compressing
    let result = manager.compress_and_update().await;

    // Then it succeeds without calling the model or writing memory
    assert!(result.is_ok());
    let mut conn = pool.acquire().await.expect("acquire");
    let facts: u16 = query_scalar("SELECT COUNT(*) FROM fact")
        .fetch_one(&mut *conn)
        .await
        .expect("count facts");
    assert_eq!(facts, 0);
}

#[tokio::test]
async fn compress_is_a_noop_when_every_message_is_already_covered() {
    // Given two messages whose seqs are already below the act watermark
    let (pool, _db) = setup_db().await;
    let adventure = create_adventure(&pool, &[]).await;
    let seqs = seed_messages(&pool, adventure.id, 2).await;
    let act_id = seed_active_act(&pool, adventure.id, Some(*seqs.last().expect("last"))).await;
    let manager = load_manager(&pool, &adventure).await;

    // When compressing
    let result = manager.compress_and_update().await;

    // Then it succeeds without advancing the watermark or writing facts
    assert!(result.is_ok());
    let mut conn = pool.acquire().await.expect("acquire");
    let watermark: Option<i64> = query_scalar("SELECT digest_to_seq FROM act WHERE id = ?1")
        .bind(act_id.to_string())
        .fetch_one(&mut *conn)
        .await
        .expect("read watermark");
    assert_eq!(watermark, Some(*seqs.last().expect("last") as i64));
    let facts: u16 = query_scalar("SELECT COUNT(*) FROM fact")
        .fetch_one(&mut *conn)
        .await
        .expect("count facts");
    assert_eq!(facts, 0);
}

// ---------------------------------------------------------------- real model path

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn compress_distills_the_uncovered_tail_persists_memory_and_advances_the_watermark() {
    // Given three messages where the first is already covered by the watermark,
    // so only the uncovered tail (seqs 2 and 3) should be distilled.
    let (pool, _db) = setup_db().await;
    let adventure = create_adventure(&pool, &[]).await;
    let seqs = seed_messages(&pool, adventure.id, 3).await;
    let act_id = seed_active_act(&pool, adventure.id, Some(seqs[0])).await;
    let manager = load_manager(&pool, &adventure).await;

    // When compressing through the real memory model
    manager
        .compress_and_update()
        .await
        .expect("compress succeeds");

    let mut conn = pool.acquire().await.expect("acquire");

    // Then the watermark advances to the last seq (resumed after the covered head)
    // and a running digest is written.
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
    // The immutable act span is never touched by compression.
    assert_eq!(row.get::<Option<i64>, _>("from_seq"), None);
    assert_eq!(row.get::<Option<i64>, _>("to_seq"), None);

    // And at least one fact is linked to the act, with a single live state row.
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
