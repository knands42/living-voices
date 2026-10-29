use crate::backend::application::helper::setup_db;
use crate::backend::persistence::helper::{seed_act, seed_adventure, seed_message};
use living_voices::backend::domain::memory::{Act, ActStatus};
use living_voices::backend::persistence::act_persistence::{
    activate_next_act, close_act, count_messages_per_act, get_active_act, get_acts, insert_act,
    update_act_running_digest, update_active_act_last_closure_score,
};

#[tokio::test]
async fn insert_act_persists_a_new_act() {
    let (pool, _db) = setup_db().await;
    let adventure = seed_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");

    let act = Act::new(adventure.id, 1, "recover the crown").with_status(ActStatus::Active);
    insert_act(&mut conn, &act).await.expect("insert act");

    let loaded = get_active_act(&mut conn, adventure.id)
        .await
        .expect("get")
        .expect("active act");
    assert_eq!(loaded.id, act.id);
    assert_eq!(loaded.goal, "recover the crown");
    assert_eq!(loaded.status, ActStatus::Active);
}

#[tokio::test]
async fn get_active_act_returns_only_the_active_one() {
    let (pool, _db) = setup_db().await;
    let adventure = seed_adventure(&pool).await;
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
    let adventure = seed_adventure(&pool).await;
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
    let adventure = seed_adventure(&pool).await;
    seed_act(&pool, adventure.id, 1, "resolved", Some(1), Some("done")).await;
    seed_act(&pool, adventure.id, 2, "active", None, None).await;
    let mut conn = pool.acquire().await.expect("acquire");

    let acts = get_acts(&mut conn, adventure.id).await.expect("get acts");
    assert_eq!(acts.len(), 2);
}

#[tokio::test]
async fn update_act_running_digest_sets_digest_and_watermark() {
    let (pool, _db) = setup_db().await;
    let adventure = seed_adventure(&pool).await;
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
    assert_eq!(act.digest_to_seq, 42);
    // The immutable span is untouched.
    assert_eq!(act.from_seq, 0);
    assert_eq!(act.to_seq, 0);
}

#[tokio::test]
async fn update_active_act_last_closure_score_records_the_score() {
    let (pool, _db) = setup_db().await;
    let adventure = seed_adventure(&pool).await;
    let act_id = seed_act(&pool, adventure.id, 1, "active", None, None).await;
    let mut conn = pool.acquire().await.expect("acquire");

    update_active_act_last_closure_score(&mut conn, adventure.id, 4)
        .await
        .expect("update score");

    let score: u16 = sqlx::query_scalar("SELECT last_closure_score FROM act WHERE id = ?1")
        .bind(act_id.to_string())
        .fetch_one(&mut *conn)
        .await
        .expect("score");
    assert_eq!(score, 4);
}

#[tokio::test]
async fn close_act_writes_the_summary_and_status() {
    let (pool, _db) = setup_db().await;
    let adventure = seed_adventure(&pool).await;
    let act_id = seed_act(&pool, adventure.id, 1, "active", Some(1), None).await;
    let mut conn = pool.acquire().await.expect("acquire");

    close_act(
        &mut conn,
        act_id,
        "ACT 1: the crown recovered",
        "They recovered the crown and fled north.",
        &ActStatus::Resolved.to_string(),
        12,
    )
    .await
    .expect("close act");

    let (index_line, digest, status, to_seq): (Option<String>, Option<String>, String, u16) =
        sqlx::query_as("SELECT index_line, digest, status, to_seq FROM act WHERE id = ?1")
            .bind(act_id.to_string())
            .fetch_one(&mut *conn)
            .await
            .expect("row");
    assert_eq!(index_line.as_deref(), Some("ACT 1: the crown recovered"));
    assert_eq!(
        digest.as_deref(),
        Some("They recovered the crown and fled north.")
    );
    assert_eq!(status, "resolved");
    assert_eq!(to_seq, 12);
}

#[tokio::test]
async fn count_messages_per_act_counts_from_the_active_act_start() {
    let (pool, _db) = setup_db().await;
    let adventure = seed_adventure(&pool).await;
    let first = seed_message(&pool, adventure.id, "line 1").await;
    seed_message(&pool, adventure.id, "line 2").await;
    seed_message(&pool, adventure.id, "line 3").await;

    // The active act starts at the second message.
    seed_act(&pool, adventure.id, 1, "active", Some(first + 1), None).await;
    let mut conn = pool.acquire().await.expect("acquire");

    let count = count_messages_per_act(&mut conn, adventure.id)
        .await
        .expect("count");
    assert_eq!(count, 2);
}

#[tokio::test]
async fn activate_next_act_promotes_the_act_after_the_closed_one() {
    let (pool, _db) = setup_db().await;
    let adventure = seed_adventure(&pool).await;
    // Act 1 has reached a terminal state; act 2 is waiting unstarted.
    seed_act(&pool, adventure.id, 1, "resolved", Some(1), Some("done")).await;
    let next_id = seed_act(&pool, adventure.id, 2, "unstarted", None, None).await;
    let mut conn = pool.acquire().await.expect("acquire");

    activate_next_act(&mut conn, adventure.id)
        .await
        .expect("activate next");

    let active = get_active_act(&mut conn, adventure.id)
        .await
        .expect("get")
        .expect("an active act");
    assert_eq!(active.id, next_id);
    assert_eq!(active.ordinal, 2);
}
