use sqlx::sqlite::SqlitePool;

use crate::backend::application::helper::setup_db;
use living_voices::backend::domain::adventure::Adventure;
use living_voices::backend::domain::message::Message;
use living_voices::backend::persistence::adventure_persistence::insert_adventure;
use living_voices::backend::persistence::message_persistence::{fetch_since, insert_message};

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

#[tokio::test]
async fn insert_and_fetch_round_trip_keeps_the_message() {
    // Given a migrated database with an adventure
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");

    // When inserting a player message
    let message = Message::new(
        adventure.id,
        None,
        None,
        "Alice".to_string(),
        "come here",
        false,
        vec![],
    );
    let stored = insert_message(&mut conn, &message)
        .await
        .expect("insert message");

    // Then a seq is assigned on insert
    assert!(stored.seq.is_some());

    // And reading it back restores the text, speaker and ooc flag
    let messages = fetch_since(&mut conn, adventure.id, 0)
        .await
        .expect("fetch messages");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].speaker_name, "Alice");
    assert_eq!(messages[0].text, "come here");
    assert!(!messages[0].ooc);
}

#[tokio::test]
async fn fetch_since_excludes_messages_at_or_below_the_cursor() {
    // Given three stored messages
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");

    let mut seqs = Vec::new();
    for i in 1..=3 {
        let message = Message::new(
            adventure.id,
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

    // When fetching after the first message's seq
    let messages = fetch_since(&mut conn, adventure.id, seqs[0])
        .await
        .expect("fetch messages");

    // Then only the later two messages come back, in order
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].text, "line 2");
    assert_eq!(messages[1].text, "line 3");
}
