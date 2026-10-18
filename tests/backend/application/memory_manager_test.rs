use sqlx::sqlite::SqlitePool;
use sqlx::{Row, query, query_scalar};
use uuid::Uuid;

use crate::backend::application::helper::setup_db;
use living_voices::ai::{ChatMessage, MemoryUpdate};
use living_voices::backend::application::memory_manager::MemoryManager;
use living_voices::backend::application::state::SharedState;
use living_voices::backend::domain::adventure::Adventure;
use living_voices::backend::domain::memory::Summary;
use living_voices::backend::domain::message::{Message, MessageKind, MessagePart, MessageParts};
use living_voices::backend::persistence::adventure_persistence::insert_adventure;
use living_voices::backend::persistence::memory_persistence::{insert_summary, last_covered};
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

async fn load_adventure(pool: &SqlitePool, adventure: &Adventure) -> MemoryManager<MemoryUpdate> {
    let state = SharedState::new();
    state
        .write()
        .expect("lock")
        .set_adventure(adventure.clone());
    MemoryManager::new(state, pool.clone(), distiller())
}

async fn seed_messages(pool: &SqlitePool, adventure_id: Uuid, count: u64) -> Vec<i64> {
    let mut conn = pool.acquire().await.expect("acquire");
    let mut seqs = Vec::new();
    for i in 1..=count {
        let message = Message::new(
            adventure_id,
            None,
            None,
            format!("Speaker {i}"),
            MessageParts::one(MessagePart::new(MessageKind::Say, &format!("line {i}"))),
        );
        let stored = insert_message(&mut conn, &message)
            .await
            .expect("insert message");
        seqs.push(stored.seq.expect("assigned seq") as i64);
    }
    seqs
}

// ---------------------------------------------------------------- tests

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn compress_persists_summary_and_facts_for_uncovered_messages() {
    // Given an adventure with two messages and no summary yet
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool).await;
    let seqs = seed_messages(&pool, adventure.id, 2).await;
    let manager = load_adventure(&pool, &adventure).await;

    // When compressing through the real memory model
    manager.compress().await.expect("compress succeeds");

    // Then one summary covers both messages and has facts linked to it
    let mut conn = pool.acquire().await.expect("acquire");
    let row = query("SELECT id, text, from_seq, to_seq FROM summary WHERE adventure_id = ?1")
        .bind(adventure.id.to_string())
        .fetch_one(&mut *conn)
        .await
        .expect("summary row");

    assert_eq!(row.get::<i64, _>("from_seq"), seqs[0]);
    assert_eq!(row.get::<i64, _>("to_seq"), *seqs.last().expect("last seq"));
    assert!(!row.get::<String, _>("text").trim().is_empty());

    let summary_id: String = row.get("id");
    let fact_count: i64 = query_scalar("SELECT COUNT(*) FROM fact WHERE source_summary_id = ?1")
        .bind(&summary_id)
        .fetch_one(&mut *conn)
        .await
        .expect("fact count");
    assert!(fact_count >= 1, "expected at least one fact");
}

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn compress_resumes_after_the_last_covered_seq() {
    // Given three messages where the first one is already covered
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool).await;
    let seqs = seed_messages(&pool, adventure.id, 3).await;

    let mut conn = pool.acquire().await.expect("acquire");
    let earlier = Summary::new(
        adventure.id,
        "Earlier summary".into(),
        seqs[0] as u64,
        seqs[0] as u64,
    );
    insert_summary(&mut conn, &earlier)
        .await
        .expect("insert summary");
    drop(conn);

    let manager = load_adventure(&pool, &adventure).await;

    // When compressing
    manager.compress().await.expect("compress succeeds");

    // Then only the uncovered tail is summarized and the cursor moves to the last seq
    let mut conn = pool.acquire().await.expect("acquire");
    assert_eq!(
        last_covered(&mut conn, adventure.id).await,
        Some(*seqs.last().expect("last seq") as u64)
    );

    let rows =
        query("SELECT from_seq, to_seq FROM summary WHERE adventure_id = ?1 ORDER BY from_seq")
            .bind(adventure.id.to_string())
            .fetch_all(&mut *conn)
            .await
            .expect("summary rows");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].get::<i64, _>("from_seq"), seqs[1]);
    assert_eq!(
        rows[1].get::<i64, _>("to_seq"),
        *seqs.last().expect("last seq")
    );
}

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn compress_is_a_noop_without_a_current_adventure() {
    // Given a migrated database and no adventure loaded in the state
    let (pool, _db) = setup_db().await;
    let manager = MemoryManager::new(SharedState::new(), pool.clone(), distiller());

    // When compressing
    let result = manager.compress().await;

    // Then it succeeds without touching the database
    assert!(result.is_ok());
    let mut conn = pool.acquire().await.expect("acquire");
    let count: i64 = query_scalar("SELECT COUNT(*) FROM summary")
        .fetch_one(&mut *conn)
        .await
        .expect("count summaries");
    assert_eq!(count, 0);
}

#[tokio::test]
#[ignore = "requires a running Ollama server"]
async fn compress_is_a_noop_when_every_message_is_covered() {
    // Given two messages already covered by an existing summary
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool).await;
    let seqs = seed_messages(&pool, adventure.id, 2).await;

    let mut conn = pool.acquire().await.expect("acquire");
    let earlier = Summary::new(
        adventure.id,
        "Earlier summary".into(),
        seqs[0] as u64,
        seqs[1] as u64,
    );
    insert_summary(&mut conn, &earlier)
        .await
        .expect("insert summary");
    drop(conn);

    let manager = load_adventure(&pool, &adventure).await;

    // When compressing
    let result = manager.compress().await;

    // Then it succeeds without calling the model and the summary is left untouched
    assert!(result.is_ok());
    let mut conn = pool.acquire().await.expect("acquire");
    let rows = query("SELECT id FROM summary WHERE adventure_id = ?1")
        .bind(adventure.id.to_string())
        .fetch_all(&mut *conn)
        .await
        .expect("summary rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get::<String, _>("id"), earlier.id.to_string());
}

// ---------------------------------------------------------------- formatting

#[test]
fn chat_message_from_single_part_formats_speaker_kind_and_target() {
    // Given a message with one targeted part
    let message = Message::new(
        Uuid::new_v4(),
        None,
        None,
        "Alice".to_string(),
        MessageParts::one(MessagePart::new(MessageKind::Say, "Open the door").with_target("Bob")),
    );

    // When rendering it for the memory model
    let chat = ChatMessage::from(&message);

    // Then the line reads `[<character> say] to [<target>]: <message>`
    assert_eq!(chat.content, "[Alice say] to [Bob]: Open the door");
}

#[test]
fn chat_message_from_two_parts_joins_segments() {
    // Given a message that speaks to a target while doing something
    let message = Message::new(
        Uuid::new_v4(),
        None,
        None,
        "Alice".to_string(),
        MessageParts::two(
            MessagePart::new(MessageKind::Say, "come here, drawing my sword").with_target("Bob"),
            MessagePart::new(MessageKind::Do, "raise the paper").with_target("Bob"),
        ),
    );

    // When rendering it for the memory model
    let chat = ChatMessage::from(&message);

    // Then both actions appear, each segment with its own text, joined with ` + `
    assert_eq!(
        chat.content,
        "[Alice say] to [Bob]: come here, drawing my sword + [Alice do] to [Bob]: raise the paper"
    );
}
