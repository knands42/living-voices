use sqlx::query;
use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

use crate::backend::application::helper::setup_db;
use living_voices::backend::domain::adventure::Adventure;
use living_voices::backend::domain::memory::{Fact, Permanence, State};
use living_voices::backend::domain::message::Message;
use living_voices::backend::persistence::adventure_persistence::insert_adventure;
use living_voices::backend::persistence::memory_persistence::{
    count_messages_per_act, get_act, get_active_act, get_active_candidates, get_state,
    insert_facts, update_act_running_digest, upsert_state,
};
use living_voices::backend::persistence::message_persistence::insert_message;

async fn setup_adventure(pool: &SqlitePool) -> Adventure {
    let adventure = Adventure::new("A".into(), "B".into(), "Guide".into(), "");
    let mut conn = pool.acquire().await.expect("acquire");
    insert_adventure(&mut conn, &adventure)
        .await
        .expect("insert adventure");
    adventure
}

/// Seed an act with a given ordinal, status and optional span, returning its id.
async fn seed_act(
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

async fn seed_message(pool: &SqlitePool, adventure_id: Uuid, text: &str) -> u16 {
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

// ---------------------------------------------------------------- state

#[tokio::test]
async fn upsert_state_inserts_then_updates_the_single_row() {
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");

    // Insert
    let mut state = State::new(adventure.id);
    state.day = "Day 1".into();
    state.place = "the shore".into();
    state.characters_present = vec!["Mira".into()];
    upsert_state(&mut conn, &state).await.expect("insert state");

    let loaded = get_state(&mut conn, adventure.id)
        .await
        .expect("get")
        .expect("some state");
    assert_eq!(loaded.day, String::from("Day 1"));
    assert_eq!(loaded.place, String::from("the shore"));
    assert_eq!(loaded.characters_present, vec!["Mira".to_string()]);

    // Update in place (same adventure_id => conflict path)
    let mut next = State::new(adventure.id);
    next.day = "Day 2".into();
    next.characters_present = vec!["Mira".into(), "Kael".into()];
    upsert_state(&mut conn, &next).await.expect("update state");

    let loaded = get_state(&mut conn, adventure.id)
        .await
        .expect("get")
        .expect("some state");
    assert_eq!(loaded.day, String::from("Day 2"));
    assert_eq!(loaded.place, String::from("unknown"));
    assert_eq!(
        loaded.characters_present,
        vec!["Mira".to_string(), "Kael".to_string()]
    );

    // Still exactly one row
    let count: u16 = sqlx::query_scalar("SELECT COUNT(*) FROM state WHERE adventure_id = ?1")
        .bind(adventure.id.to_string())
        .fetch_one(&mut *conn)
        .await
        .expect("count");
    assert_eq!(count, 1);
}

#[tokio::test]
async fn get_active_candidates_reads_characters_present() {
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");

    let mut state = State::new(adventure.id);
    state.characters_present = vec!["Mira".into(), "Kael".into()];
    upsert_state(&mut conn, &state).await.expect("upsert");

    let names = get_active_candidates(&mut conn, &adventure.id.to_string())
        .await
        .expect("candidates");
    assert_eq!(names, vec!["Mira".to_string(), "Kael".to_string()]);
}

// ---------------------------------------------------------------- facts

#[tokio::test]
async fn insert_facts_writes_every_row() {
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool).await;
    let act_id = seed_act(&pool, adventure.id, 1, "active", None, None).await;
    let mut conn = pool.acquire().await.expect("acquire");

    let facts = vec![
        Fact::new(
            adventure.id,
            "Mira",
            "fears deep water",
            act_id,
            Permanence::Permanent,
        ),
        Fact::new(
            adventure.id,
            "the crown",
            "is lost",
            act_id,
            Permanence::Plot,
        ),
    ];
    insert_facts(&mut conn, &facts).await.expect("insert facts");

    let count: u16 = sqlx::query_scalar("SELECT COUNT(*) FROM fact WHERE act_id = ?1")
        .bind(act_id.to_string())
        .fetch_one(&mut *conn)
        .await
        .expect("count");
    assert_eq!(count, 2);
}

#[tokio::test]
async fn insert_facts_is_a_noop_for_an_empty_slice() {
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");

    insert_facts(&mut conn, &[]).await.expect("noop");

    let count: u16 = sqlx::query_scalar("SELECT COUNT(*) FROM fact WHERE adventure_id = ?1")
        .bind(adventure.id.to_string())
        .fetch_one(&mut *conn)
        .await
        .expect("count");
    assert_eq!(count, 0);
}

// ---------------------------------------------------------------- acts

#[tokio::test]
async fn get_active_act_returns_only_the_active_one() {
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool).await;
    seed_act(&pool, adventure.id, 1, "resolved", Some(1), Some("done")).await;
    let active_id = seed_act(&pool, adventure.id, 2, "active", None, None).await;
    let mut conn = pool.acquire().await.expect("acquire");

    let act = get_active_act(&mut conn, adventure.id)
        .await
        .expect("get")
        .expect("an active act");
    assert_eq!(act.id, active_id);
    assert_eq!(act.ordinal, 2);
}

#[tokio::test]
async fn get_active_act_is_none_when_none_active() {
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool).await;
    seed_act(&pool, adventure.id, 1, "resolved", Some(1), Some("done")).await;
    let mut conn = pool.acquire().await.expect("acquire");

    assert!(
        get_active_act(&mut conn, adventure.id)
            .await
            .expect("get")
            .is_none()
    );
}

#[tokio::test]
async fn get_act_returns_every_act() {
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool).await;
    seed_act(&pool, adventure.id, 1, "resolved", Some(1), Some("done")).await;
    seed_act(&pool, adventure.id, 2, "active", None, None).await;
    let mut conn = pool.acquire().await.expect("acquire");

    let acts = get_act(&mut conn, adventure.id).await.expect("get acts");
    assert_eq!(acts.len(), 2);
}

#[tokio::test]
async fn update_act_running_digest_sets_digest_and_watermark() {
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool).await;
    let act_id = seed_act(&pool, adventure.id, 1, "active", None, None).await;
    let mut conn = pool.acquire().await.expect("acquire");

    update_act_running_digest(&mut conn, act_id, "They fled north.", 42)
        .await
        .expect("update");

    let act = get_active_act(&mut conn, adventure.id)
        .await
        .expect("get")
        .expect("active");
    assert_eq!(act.running_digest.as_deref(), Some("They fled north."));
    assert_eq!(act.digest_to_seq, Some(42));
    // The immutable span is untouched.
    assert_eq!(act.from_seq, 0);
    assert_eq!(act.to_seq, 0);
}

// ---------------------------------------------------------------- counts

#[tokio::test]
async fn count_uncovered_per_act_counts_messages_from_the_active_act_start() {
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool).await;
    let first = seed_message(&pool, adventure.id, "line 1").await;
    seed_message(&pool, adventure.id, "line 2").await;
    seed_message(&pool, adventure.id, "line 3").await;

    // Active act starts at the second message.
    seed_act(&pool, adventure.id, 1, "active", Some(first + 1), None).await;
    let mut conn = pool.acquire().await.expect("acquire");

    let count = count_messages_per_act(&mut conn, adventure.id)
        .await
        .expect("count");
    assert_eq!(count, 2);
}
