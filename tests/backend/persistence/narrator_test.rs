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
async fn adventure_insert_creates_a_default_narrator() {
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool).await;

    let (name, count): (String, u16) = sqlx::query_as(
        "SELECT name, (SELECT COUNT(*) FROM narrator WHERE adventure_id = ?1)
         FROM narrator WHERE adventure_id = ?1",
    )
    .bind(adventure.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("a default narrator row must exist");

    assert_eq!(count, 1);
    assert_eq!(name, "Narrator");
}

#[tokio::test]
async fn upsert_narrator_updates_the_single_row_in_place() {
    let (pool, _db) = setup_db().await;
    let adventure = setup_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");

    upsert_narrator(&mut conn, adventure.id, &adventure.narrator)
        .await
        .expect("upsert narrator");

    let (name, adventure_id, count): (String, String, u16) = sqlx::query_as(
        "SELECT name, adventure_id, (SELECT COUNT(*) FROM narrator WHERE adventure_id = ?1)
         FROM narrator WHERE adventure_id = ?1",
    )
    .bind(adventure.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("row");

    assert_eq!(count, 1);
    assert_eq!(name, "The Chronicler");
    assert_eq!(adventure_id, adventure.id.to_string());
}
