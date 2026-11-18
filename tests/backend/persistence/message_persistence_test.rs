use sqlx::query;
use uuid::Uuid;

use crate::backend::application::helper::setup_db;
use crate::backend::persistence::helper::seed_adventure;
use living_voices::backend::domain::message::Message;
use living_voices::backend::persistence::message_persistence::{fetch_since, insert_messages};

#[tokio::test]
async fn insert_and_fetch_round_trip_keeps_the_message() {
    // Given a migrated database with an adventure and its narrator
    let (pool, _db) = setup_db().await;
    let adventure = seed_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");
    let narrator_id = Uuid::new_v4();
    query("INSERT INTO narrator (id, adventure_id) VALUES (?1, ?2)")
        .bind(narrator_id.to_string())
        .bind(adventure.id.to_string())
        .execute(&mut *conn)
        .await
        .expect("insert narrator");

    // When inserting a narrator message
    let message = Message::new(adventure.id, None, Some(narrator_id), "come here", false);
    let stored = insert_messages(&mut conn, std::slice::from_ref(&message))
        .await
        .expect("insert message")
        .remove(0);

    // Then a seq is assigned on insert
    assert!(stored.seq.is_some());

    // And reading it back restores the text, speaker and ooc flag
    let messages = fetch_since(&mut conn, adventure.id, 0)
        .await
        .expect("fetch messages");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].narrator_id, Some(narrator_id));
    assert!(messages[0].character_id.is_none());
    assert_eq!(messages[0].text, "come here");
    assert!(!messages[0].ooc);
}

#[tokio::test]
async fn fetch_since_excludes_messages_at_or_below_the_cursor() {
    // Given three stored messages
    let (pool, _db) = setup_db().await;
    let adventure = seed_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");

    let batch: Vec<Message> = (1..=3)
        .map(|i| Message::new(adventure.id, None, None, format!("line {i}"), false))
        .collect();

    // When inserting them as one batch
    let stored = insert_messages(&mut conn, &batch)
        .await
        .expect("insert messages");

    // Then each one comes back with its own seq, in the order they were written
    let seqs: Vec<u16> = stored
        .iter()
        .map(|message| message.seq.expect("assigned seq"))
        .collect();
    assert_eq!(seqs.len(), 3);
    assert!(seqs.windows(2).all(|pair| pair[0] < pair[1]));

    // When fetching after the first message's seq
    let messages = fetch_since(&mut conn, adventure.id, seqs[0])
        .await
        .expect("fetch messages");

    // Then only the later two messages come back, in order
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].text, "line 2");
    assert_eq!(messages[1].text, "line 3");
}

#[tokio::test]
async fn seq_starts_at_one_for_each_adventure() {
    // given two adventures
    let (pool, _db) = setup_db().await;
    let first = seed_adventure(&pool).await;
    let second = seed_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");

    // when each adventure gets a batch of two messages
    let first_batch: Vec<Message> = (1..=2)
        .map(|i| Message::new(first.id, None, None, format!("first {i}"), false))
        .collect();
    let second_batch: Vec<Message> = (1..=2)
        .map(|i| Message::new(second.id, None, None, format!("second {i}"), false))
        .collect();
    let first_stored = insert_messages(&mut conn, &first_batch)
        .await
        .expect("insert first batch");
    let second_stored = insert_messages(&mut conn, &second_batch)
        .await
        .expect("insert second batch");

    // then each adventure's seqs start at one and keep their input order
    let first_seqs: Vec<u16> = first_stored
        .iter()
        .map(|message| message.seq.expect("assigned seq"))
        .collect();
    let second_seqs: Vec<u16> = second_stored
        .iter()
        .map(|message| message.seq.expect("assigned seq"))
        .collect();
    assert_eq!(first_seqs, [1, 2]);
    assert_eq!(second_seqs, [1, 2]);
    assert_eq!(first_stored[0].text, "first 1");
    assert_eq!(first_stored[1].text, "first 2");
    assert_eq!(second_stored[0].text, "second 1");
    assert_eq!(second_stored[1].text, "second 2");
}
