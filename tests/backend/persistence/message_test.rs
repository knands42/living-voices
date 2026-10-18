use sqlx::sqlite::SqlitePool;

use crate::backend::application::helper::setup_db;
use living_voices::backend::domain::adventure::Adventure;
use living_voices::backend::domain::message::{Message, MessageKind, MessagePart, MessageParts};
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
async fn insert_and_fetch_round_trip_keeps_both_parts() {
    // Given a migrated database with an adventure
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");

    // When inserting a message with two targeted parts
    let message = Message::new(
        adventure.id,
        None,
        None,
        "Alice".to_string(),
        MessageParts::two(
            MessagePart::new(MessageKind::Say, "come here").with_target("Bob"),
            MessagePart::new(MessageKind::Do, "raise the drawing").with_target("Bob"),
        ),
    );
    insert_message(&mut conn, &message)
        .await
        .expect("insert message");

    // Then reading it back restores both parts with their own text and target
    let messages = fetch_since(&mut conn, adventure.id, 0)
        .await
        .expect("fetch messages");
    assert_eq!(messages.len(), 1);

    let parts: Vec<&MessagePart> = messages[0].parts.iter().collect();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].kind, MessageKind::Say);
    assert_eq!(parts[0].text, "come here");
    assert_eq!(parts[0].target_name.as_deref(), Some("Bob"));
    assert_eq!(parts[1].kind, MessageKind::Do);
    assert_eq!(parts[1].text, "raise the drawing");
    assert_eq!(parts[1].target_name.as_deref(), Some("Bob"));
}

#[tokio::test]
async fn insert_rejects_target_on_kind_without_target() {
    // Given a migrated database with an adventure
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");

    // When inserting a think part addressed to someone
    let message = Message::new(
        adventure.id,
        None,
        None,
        "Alice".to_string(),
        MessageParts::one(
            MessagePart::new(MessageKind::Think, "what is he doing").with_target("Bob"),
        ),
    );
    let result = insert_message(&mut conn, &message).await;

    // Then the DB CHECK rejects it: only say/whisper/do take a target
    assert!(result.is_err());
}
