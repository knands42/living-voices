use std::collections::HashMap;

use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

use crate::backend::application::helper::setup_db;
use living_voices::backend::domain::adventure::Adventure;
use living_voices::backend::domain::character::{Character, CharacterStatus};
use living_voices::backend::persistence::adventure_persistence::insert_adventure;
use living_voices::backend::persistence::characters_persistence::{
    insert_character, insert_relations,
};

async fn setup_adventure(pool: &SqlitePool) -> Uuid {
    let adventure = Adventure::new("A".into(), "B".into(), "Guide".into(), "");
    let mut conn = pool.acquire().await.expect("acquire");
    insert_adventure(&mut conn, &adventure)
        .await
        .expect("insert adventure");
    adventure.id
}

#[tokio::test]
async fn insert_character_persists_core_columns() {
    let (pool, _db) = setup_db().await;
    let adventure_id = setup_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");

    let character = Character::new("Mara".into(), Some(adventure_id), "A tide-singer".into())
        .with_backstory("Raised in the drowned quarter".into())
        .with_status(CharacterStatus::Away);
    insert_character(&mut conn, Some(adventure_id), &character)
        .await
        .expect("insert");

    let (name, backstory, status): (String, String, String) =
        sqlx::query_as("SELECT name, backstory, status FROM character WHERE id = ?1")
            .bind(character.id.to_string())
            .fetch_one(&pool)
            .await
            .expect("row");
    assert_eq!(name, "Mara");
    assert_eq!(backstory, "Raised in the drowned quarter");
    assert_eq!(status, "away");
}

#[tokio::test]
async fn insert_character_persists_traits_in_order() {
    let (pool, _db) = setup_db().await;
    let adventure_id = setup_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");

    let mut character = Character::new("Mara".into(), Some(adventure_id), "A tide-singer".into());
    character.add_trait("speaks in tidal registers".into());
    character.add_trait("hums before speaking".into());
    insert_character(&mut conn, Some(adventure_id), &character)
        .await
        .expect("insert");

    let rows: Vec<(u16, String)> = sqlx::query_as(
        "SELECT position, value FROM character_trait WHERE character_id = ?1 ORDER BY position",
    )
    .bind(character.id.to_string())
    .fetch_all(&pool)
    .await
    .expect("traits");
    assert_eq!(rows[0], (0, "speaks in tidal registers".to_string()));
    assert_eq!(rows[1], (1, "hums before speaking".to_string()));
}

#[tokio::test]
async fn insert_character_normalizes_tags() {
    let (pool, _db) = setup_db().await;
    let adventure_id = setup_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");

    let mut character = Character::new("Mara".into(), Some(adventure_id), "A tide-singer".into());
    character.add_tag("Protagonist".into());
    insert_character(&mut conn, Some(adventure_id), &character)
        .await
        .expect("insert");

    let tags: Vec<String> =
        sqlx::query_scalar("SELECT tag FROM character_tag WHERE character_id = ?1")
            .bind(character.id.to_string())
            .fetch_all(&pool)
            .await
            .expect("tags");
    assert_eq!(tags, vec!["protagonist".to_string()]);
}

#[tokio::test]
async fn insert_relations_links_characters() {
    let (pool, _db) = setup_db().await;
    let adventure_id = setup_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");

    let mara = Character::new("Mara".into(), Some(adventure_id), "singer".into());
    let otho = Character::new("Otho".into(), Some(adventure_id), "diver".into());
    insert_character(&mut conn, Some(adventure_id), &mara)
        .await
        .expect("insert mara");
    insert_character(&mut conn, Some(adventure_id), &otho)
        .await
        .expect("insert otho");

    let mut relations = HashMap::new();
    relations.insert(otho.id, "Brother".to_string());
    insert_relations(&mut conn, mara.id, &relations)
        .await
        .expect("relations");

    let relation: String = sqlx::query_scalar(
        "SELECT relation FROM character_relation
         WHERE character_id = ?1 AND related_character_id = ?2",
    )
    .bind(mara.id.to_string())
    .bind(otho.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("relation");
    assert_eq!(relation, "Brother");
}
