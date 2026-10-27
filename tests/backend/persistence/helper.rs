//! Shared DB-level seed helpers for the persistence tests.

use sqlx::query;
use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

use living_voices::backend::domain::adventure::Adventure;
use living_voices::backend::domain::message::Message;
use living_voices::backend::persistence::adventure_persistence::insert_adventure;
use living_voices::backend::persistence::message_persistence::insert_message;

pub async fn seed_adventure(pool: &SqlitePool) -> Adventure {
    let adventure = Adventure::new("A".into(), "B".into(), "Guide".into(), "");
    let mut conn = pool.acquire().await.expect("acquire");
    insert_adventure(&mut conn, &adventure)
        .await
        .expect("insert adventure");
    adventure
}

pub async fn seed_act(
    pool: &SqlitePool,
    adventure_id: Uuid,
    ordinal: u16,
    status: &str,
    from_seq: Option<u16>,
    digest: Option<&str>,
) -> Uuid {
    let act_id = Uuid::new_v4();
    let mut conn = pool.acquire().await.expect("acquire");
    query(
        "INSERT INTO act (id, adventure_id, ordinal, goal, status, from_seq, digest)
         VALUES (?1, ?2, ?3, 'reach the surface', ?4, ?5, ?6)",
    )
    .bind(act_id.to_string())
    .bind(adventure_id.to_string())
    .bind(ordinal)
    .bind(status)
    .bind(from_seq.map(|s| s as i64))
    .bind(digest)
    .execute(&mut *conn)
    .await
    .expect("insert act");
    act_id
}

pub async fn seed_message(pool: &SqlitePool, adventure_id: Uuid, text: &str) -> u16 {
    let mut conn = pool.acquire().await.expect("acquire");
    let message = Message::new(
        adventure_id,
        None,
        None,
        "Speaker".into(),
        text,
        false,
        vec![],
    );
    insert_message(&mut conn, &message)
        .await
        .expect("insert message")
        .seq
        .expect("seq")
}
