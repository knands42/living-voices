use crate::backend::application::helper::setup_db;
use crate::backend::persistence::helper::seed_adventure;
use living_voices::backend::domain::memory::{Fact, Permanence};
use living_voices::backend::persistence::fact_persistence::{get_facts, upsert_facts};

#[tokio::test]
async fn upsert_facts_stores_the_whole_snapshot() {
    let (pool, _db) = setup_db().await;
    let adventure = seed_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");

    let facts = vec![
        Fact::new("Mira", "fears deep water", Permanence::Permanent),
        Fact::new("the crown", "is lost", Permanence::Plot),
    ];
    upsert_facts(&mut conn, adventure.id, &facts)
        .await
        .expect("upsert facts");

    let stored = get_facts(&mut conn, adventure.id).await.expect("get facts");
    assert_eq!(stored, facts);

    // exactly one row per adventure
    let count: u16 = sqlx::query_scalar("SELECT COUNT(*) FROM fact WHERE adventure_id = ?1")
        .bind(adventure.id.to_string())
        .fetch_one(&mut *conn)
        .await
        .expect("count");
    assert_eq!(count, 1);
}

#[tokio::test]
async fn upsert_facts_replaces_the_previous_snapshot() {
    let (pool, _db) = setup_db().await;
    let adventure = seed_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");

    upsert_facts(
        &mut conn,
        adventure.id,
        &[Fact::new("Mira", "fears deep water", Permanence::Permanent)],
    )
    .await
    .expect("first upsert");

    let replacement = vec![Fact::new("the crown", "is recovered", Permanence::Plot)];
    upsert_facts(&mut conn, adventure.id, &replacement)
        .await
        .expect("second upsert");

    let stored = get_facts(&mut conn, adventure.id).await.expect("get facts");
    assert_eq!(stored, replacement);
}

#[tokio::test]
async fn get_facts_is_empty_without_a_row() {
    let (pool, _db) = setup_db().await;
    let adventure = seed_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");

    let stored = get_facts(&mut conn, adventure.id).await.expect("get facts");
    assert!(stored.is_empty());
}
