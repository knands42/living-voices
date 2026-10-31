use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

use crate::backend::application::helper::setup_db;
use living_voices::backend::persistence::adventure_persistence::insert_adventure;
use living_voices::backend::persistence::narrator_persistence::insert_default_narrator;

async fn setup_adventure(pool: &SqlitePool) -> Uuid {
    let mut conn = pool.acquire().await.expect("acquire");
    insert_adventure(&mut conn, Uuid::new_v4(), "A", "B", "", vec![])
        .await
        .expect("insert adventure")
        .id
}

#[tokio::test]
async fn insert_narrator_persists_name_and_links_to_adventure() {
    let (pool, _db) = setup_db().await;
    let adv_id = setup_adventure(&pool).await;
    let narrator_id = Uuid::new_v4();
    let mut conn = pool.acquire().await.expect("acquire");

    insert_default_narrator(&mut conn, narrator_id, adv_id)
        .await
        .expect("insert narrator");

    let (name, linked_adventure_id): (String, String) =
        sqlx::query_as("SELECT name, adventure_id FROM narrator WHERE id = ?1")
            .bind(narrator_id.to_string())
            .fetch_one(&pool)
            .await
            .expect("row");
    assert_eq!(name, "Narrator");
    assert_eq!(linked_adventure_id, adv_id.to_string());
}
