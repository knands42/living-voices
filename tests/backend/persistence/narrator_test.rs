use sqlx::sqlite::SqlitePool;

use crate::backend::application::helper::setup_db;
use living_voices::backend::domain::adventure::Adventure;
use living_voices::backend::persistence::adventure_persistence::insert_adventure;
use living_voices::backend::persistence::narrator_persistence::upsert_narrator;

async fn setup_adventure(pool: &SqlitePool) -> Adventure {
    let adventure = Adventure::new("A".into(), "B".into(), "The Chronicler".into(), "");
    let mut conn = pool.acquire().await.expect("acquire");
    insert_adventure(&mut conn, &adventure)
        .await
        .expect("insert adventure");
    adventure
}

#[tokio::test]
async fn insert_narrator_persists_name_and_links_to_adventure() {
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");

    upsert_narrator(&mut conn, adventure.id, &adventure.narrator)
        .await
        .expect("insert narrator");

    let (name, adventure_id): (String, String) =
        sqlx::query_as("SELECT name, adventure_id FROM narrator WHERE id = ?1")
            .bind(adventure.narrator.id.to_string())
            .fetch_one(&pool)
            .await
            .expect("row");
    assert_eq!(name, "The Chronicler");
    assert_eq!(adventure_id, adventure.id.to_string());
}
