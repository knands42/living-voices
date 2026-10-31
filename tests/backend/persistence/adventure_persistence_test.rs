use crate::backend::application::helper::setup_db;
use living_voices::backend::persistence::adventure_persistence::insert_adventure;
use uuid::Uuid;

#[tokio::test]
async fn insert_adventure_persists_core_columns() {
    let (pool, _db) = setup_db().await;
    let mut conn = pool.acquire().await.expect("acquire");
    let adventure_id = Uuid::new_v4();
    insert_adventure(
        &mut conn,
        adventure_id,
        "Tides of Glass",
        "A drowned city",
        "tense",
        vec![],
    )
    .await
    .expect("insert");

    let (title, headline, tone): (String, String, String) =
        sqlx::query_as("SELECT title, headline, tone FROM adventure WHERE id = ?1")
            .bind(adventure_id.to_string())
            .fetch_one(&pool)
            .await
            .expect("row");
    assert_eq!(title, "Tides of Glass");
    assert_eq!(headline, "A drowned city");
    assert_eq!(tone, "tense");
}

#[tokio::test]
async fn insert_adventure_normalizes_and_attaches_tags() {
    let (pool, _db) = setup_db().await;
    let mut conn = pool.acquire().await.expect("acquire");
    let adventure_id = Uuid::new_v4();
    insert_adventure(
        &mut conn,
        adventure_id,
        "A",
        "B",
        "",
        vec!["Coastal".into(), "Mystery".into()],
    )
    .await
    .expect("insert");

    let tags: Vec<String> =
        sqlx::query_scalar("SELECT tag FROM adventure_tag WHERE adventure_id = ?1 ORDER BY tag")
            .bind(adventure_id.to_string())
            .fetch_all(&pool)
            .await
            .expect("tags");
    assert_eq!(tags, vec!["coastal".to_string(), "mystery".to_string()]);
}
