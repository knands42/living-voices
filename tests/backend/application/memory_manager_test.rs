use sqlx::sqlite::SqlitePool;
use sqlx::{Row, query, query_scalar};
use uuid::Uuid;

use crate::backend::application::helper::setup_db;
use living_voices::ai::MemoryUpdate;
use living_voices::backend::application::memory_manager::MemoryManager;
use living_voices::backend::application::state::SharedState;
use living_voices::backend::domain::adventure::Adventure;
use living_voices::backend::domain::message::Message;
use living_voices::backend::persistence::adventure_persistence::insert_adventure;
use living_voices::backend::persistence::message_persistence::insert_message;
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

async fn setup_adventure(pool: &SqlitePool) -> Adventure {
    let adventure = Adventure::new(
        "Test adventure".into(),
        "Description".into(),
        "Guide".into(),
        "",
    );
    let mut conn = pool.acquire().await.expect("acquire");
    insert_adventure(&mut conn, &adventure)
        .await
        .expect("insert adventure");
    adventure
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

async fn load_manager(pool: &SqlitePool, adventure: &Adventure) -> MemoryManager<MemoryUpdate> {
    let state = SharedState::new();
    state
        .write()
        .expect("lock")
        .set_adventure(adventure.clone());
    MemoryManager::new(state, pool.clone(), distiller())
}

async fn seed_messages(pool: &SqlitePool, adventure_id: Uuid, count: u16) -> Vec<u16> {
    let mut conn = pool.acquire().await.expect("acquire");
    let mut seqs = Vec::new();
    for i in 1..=count {
        let message = Message::new(
            adventure_id,
            None,
            None,
            format!("Speaker {i}"),
            format!("line {i}"),
            false,
            vec![],
        );
        let stored = insert_message(&mut conn, &message)
            .await
            .expect("insert message");
        seqs.push(stored.seq.expect("assigned seq"));
    }
    seqs
}

// ---------------------------------------------------------------- no-op paths (no model call)

#[tokio::test]
async fn compress_is_a_noop_without_a_current_adventure() {
    // Given a migrated database and no adventure loaded in the state
    let (pool, _db) = setup_db().await;
    let manager = MemoryManager::new(SharedState::new(), pool.clone(), distiller());

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
    let adventure = setup_adventure(&pool).await;
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
    let adventure = setup_adventure(&pool).await;
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
async fn compress_persists_facts_state_and_advances_the_watermark() {
    // Given an active act with two uncovered messages
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool).await;
    let seqs = seed_messages(&pool, adventure.id, 2).await;
    let act_id = seed_active_act(&pool, adventure.id, None).await;
    let manager = load_manager(&pool, &adventure).await;

    // When compressing through the real memory model
    manager
        .compress_and_update()
        .await
        .expect("compress succeeds");

    // Then the act watermark advanced to the last covered seq and a running digest exists
    let mut conn = pool.acquire().await.expect("acquire");
    let row = query("SELECT digest_to_seq, running_digest FROM act WHERE id = ?1")
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

    // And at least one fact is linked to the act, and a single state row exists
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

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn compress_resumes_after_the_watermark() {
    // Given three messages where the first is already covered by the watermark
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool).await;
    let seqs = seed_messages(&pool, adventure.id, 3).await;
    let act_id = seed_active_act(&pool, adventure.id, Some(seqs[0])).await;
    let manager = load_manager(&pool, &adventure).await;

    // When compressing
    manager
        .compress_and_update()
        .await
        .expect("compress succeeds");

    // Then the watermark moves to the last seq (only the uncovered tail was distilled)
    let mut conn = pool.acquire().await.expect("acquire");
    let watermark: Option<i64> = query_scalar("SELECT digest_to_seq FROM act WHERE id = ?1")
        .bind(act_id.to_string())
        .fetch_one(&mut *conn)
        .await
        .expect("read watermark");
    assert_eq!(watermark, Some(*seqs.last().expect("last seq") as i64));
}
