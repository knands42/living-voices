use std::collections::HashMap;

use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

use crate::backend::application::helper::setup_db;
use living_voices::backend::domain::character::{Character, CharacterStatus};
use living_voices::backend::persistence::adventure_persistence::insert_adventure;
use living_voices::backend::persistence::characters_persistence::{
    insert_character, insert_relations, update_appearance_current_pose,
    update_appearance_full_body_prompt, update_appearance_portrait_prompt,
};

async fn setup_adventure(pool: &SqlitePool) -> Uuid {
    let mut conn = pool.acquire().await.expect("acquire");
    insert_adventure(&mut conn, Uuid::new_v4(), "A", "B", "", vec![])
        .await
        .expect("insert adventure")
        .id
}

#[tokio::test]
async fn insert_character_persists_core_columns() {
    let (pool, _db) = setup_db().await;
    let adventure_id = setup_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");

    let character = Character::new(
        Uuid::new_v4(),
        "Mara".into(),
        adventure_id,
        "A tide-singer",
        "Raised in the drowned quarter",
        CharacterStatus::Away,
        false,
    );
    insert_character(&mut conn, adventure_id, &character)
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

    let mut character = Character::new(
        Uuid::new_v4(),
        "Mara".into(),
        adventure_id,
        "A tide-singer",
        "",
        CharacterStatus::Present,
        false,
    );
    character.add_trait("speaks in tidal registers".into());
    character.add_trait("hums before speaking".into());
    insert_character(&mut conn, adventure_id, &character)
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

    let mut character = Character::new(
        Uuid::new_v4(),
        "Mara".into(),
        adventure_id,
        "A tide-singer",
        "",
        CharacterStatus::Present,
        false,
    );
    character.add_tag("Protagonist".into());
    insert_character(&mut conn, adventure_id, &character)
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

    let mara = Character::new(
        Uuid::new_v4(),
        "Mara".into(),
        adventure_id,
        "singer",
        "",
        CharacterStatus::Present,
        false,
    );
    let otho = Character::new(
        Uuid::new_v4(),
        "Otho".into(),
        adventure_id,
        "diver",
        "",
        CharacterStatus::Present,
        false,
    );
    insert_character(&mut conn, adventure_id, &mara)
        .await
        .expect("insert mara");
    insert_character(&mut conn, adventure_id, &otho)
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

#[tokio::test]
async fn update_appearance_prompts_writes_each_column() {
    let (pool, _db) = setup_db().await;
    let adventure_id = setup_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");

    let character = Character::new(
        Uuid::new_v4(),
        "Mara".into(),
        adventure_id,
        "A tide-singer",
        "",
        CharacterStatus::Present,
        false,
    );
    insert_character(&mut conn, adventure_id, &character)
        .await
        .expect("insert");

    update_appearance_portrait_prompt(&mut conn, character.id, "portrait prompt")
        .await
        .expect("update portrait prompt");
    update_appearance_full_body_prompt(&mut conn, character.id, "full-body prompt")
        .await
        .expect("update full-body prompt");
    update_appearance_current_pose(&mut conn, character.id, "pose prompt")
        .await
        .expect("update pose prompt");

    let (portrait, full_body, pose): (Option<String>, Option<String>, Option<String>) =
        sqlx::query_as(
            "SELECT appearance_portrait_prompt, appearance_full_body_prompt,
                    appearance_current_pose
             FROM character WHERE id = ?1",
        )
        .bind(character.id.to_string())
        .fetch_one(&pool)
        .await
        .expect("query character");

    assert_eq!(portrait.as_deref(), Some("portrait prompt"));
    assert_eq!(full_body.as_deref(), Some("full-body prompt"));
    assert_eq!(pose.as_deref(), Some("pose prompt"));
}
