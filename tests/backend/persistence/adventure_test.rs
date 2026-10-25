use sqlx::sqlite::SqlitePool;

use crate::backend::application::helper::setup_db;
use living_voices::backend::domain::adventure::Adventure;
use living_voices::backend::persistence::adventure_persistence::insert_adventure;

#[tokio::test]
async fn insert_adventure_persists_core_columns() {
    let (pool, _db) = setup_db().await;
    let mut adventure = Adventure::new(
        "Tides of Glass".into(),
        "A drowned city".into(),
        "Guide".into(),
        "tense",
    );
    adventure.genre = Some("mystery".into());

    let mut conn = pool.acquire().await.expect("acquire");
    insert_adventure(&mut conn, &adventure)
        .await
        .expect("insert");

    let (title, description, genre, tone): (String, String, Option<String>, String) =
        sqlx::query_as("SELECT title, description, genre, tone FROM adventure WHERE id = ?1")
            .bind(adventure.id.to_string())
            .fetch_one(&pool)
            .await
            .expect("row");
    assert_eq!(title, "Tides of Glass");
    assert_eq!(description, "A drowned city");
    assert_eq!(genre.as_deref(), Some("mystery"));
    assert_eq!(tone, "tense");
}

#[tokio::test]
async fn insert_adventure_normalizes_and_attaches_tags() {
    let (pool, _db) = setup_db().await;
    let mut adventure = Adventure::new("A".into(), "B".into(), "Guide".into(), "");
    adventure.add_tags(vec!["Coastal".into(), "Mystery".into()]);

    let mut conn = pool.acquire().await.expect("acquire");
    insert_adventure(&mut conn, &adventure)
        .await
        .expect("insert");

    let tags: Vec<String> =
        sqlx::query_scalar("SELECT tag FROM adventure_tag WHERE adventure_id = ?1 ORDER BY tag")
            .bind(adventure.id.to_string())
            .fetch_all(&pool)
            .await
            .expect("tags");
    assert_eq!(tags, vec!["coastal".to_string(), "mystery".to_string()]);
}
