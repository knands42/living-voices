use crate::backend::application::helper::setup_db;
use crate::backend::persistence::helper::{seed_act, seed_adventure};
use living_voices::backend::domain::memory::{Fact, Permanence};
use living_voices::backend::persistence::fact_persistence::insert_facts;

#[tokio::test]
async fn insert_facts_writes_every_row() {
    let (pool, _db) = setup_db().await;
    let adventure = seed_adventure(&pool).await;
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
    let adventure = seed_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");

    insert_facts(&mut conn, &[]).await.expect("noop");

    let count: u16 = sqlx::query_scalar("SELECT COUNT(*) FROM fact WHERE adventure_id = ?1")
        .bind(adventure.id.to_string())
        .fetch_one(&mut *conn)
        .await
        .expect("count");
    assert_eq!(count, 0);
}
