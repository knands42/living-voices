use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

use crate::backend::application::helper::setup_db;
use living_voices::backend::application::context::HistoryBuilder;
use living_voices::backend::domain::adventure::Adventure;
use living_voices::backend::domain::memory::{Fact, Summary};
use living_voices::backend::domain::message::{Message, MessageKind, MessagePart, MessageParts};
use living_voices::backend::persistence::adventure_persistence::insert_adventure;
use living_voices::backend::persistence::memory_persistence::{insert_facts, insert_summary};
use living_voices::backend::persistence::message_persistence::insert_message;

// ---------------------------------------------------------------- helpers

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

/// Insert `count` plain Say messages and return their assigned seqs.
async fn seed_messages(pool: &SqlitePool, adventure_id: Uuid, count: u64) -> Vec<u64> {
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
        seqs.push(stored.seq.expect("assigned seq"));
    }
    seqs
}

// ---------------------------------------------------------------- tests

#[tokio::test]
async fn build_returns_empty_string_when_nothing_is_stored() {
    // Given an adventure with no summaries, facts, or messages
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");

    // When building the history block
    let history = HistoryBuilder::build(&mut conn, adventure.id, 5, 5)
        .await
        .expect("build history");

    // Then it is empty
    assert!(history.is_empty());
}

#[tokio::test]
async fn build_includes_summaries_facts_and_uncovered_messages() {
    // Given a summary (covering seq 1) with a fact, and a later message left uncovered
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool).await;
    let seqs = seed_messages(&pool, adventure.id, 2).await;

    let mut conn = pool.acquire().await.expect("acquire");
    let summary = Summary::new(adventure.id, "The ship sank.".into(), seqs[0], seqs[0]);
    insert_summary(&mut conn, &summary)
        .await
        .expect("insert summary");
    let facts = vec![Fact::new(
        adventure.id,
        summary.id,
        "Mira fears deep water.".into(),
    )];
    insert_facts(&mut conn, &facts).await.expect("insert facts");

    // When building the history block
    let history = HistoryBuilder::build(&mut conn, adventure.id, 5, 5)
        .await
        .expect("build history");

    // Then the summary and fact sections are present, and the uncovered tail message
    assert!(history.contains("Story so far: "));
    assert!(history.contains("The ship sank."));
    assert!(history.contains("Known facts:"));
    assert!(history.contains("Mira fears deep water."));
    assert!(history.contains("line 2"));
    // And the covered message is not repeated as a raw line
    assert!(!history.contains("line 1"));
}

#[tokio::test]
async fn build_excludes_messages_already_covered_by_a_summary() {
    // Given three messages where the first two are covered by a summary
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool).await;
    let seqs = seed_messages(&pool, adventure.id, 3).await;

    let mut conn = pool.acquire().await.expect("acquire");
    let summary = Summary::new(adventure.id, "Covered opening.".into(), seqs[0], seqs[1]);
    insert_summary(&mut conn, &summary)
        .await
        .expect("insert summary");

    // When building the history block
    let history = HistoryBuilder::build(&mut conn, adventure.id, 5, 5)
        .await
        .expect("build history");

    // Then only the uncovered tail message appears as a raw line
    assert!(history.contains("line 3"));
    assert!(!history.contains("line 1"));
    assert!(!history.contains("line 2"));
}
