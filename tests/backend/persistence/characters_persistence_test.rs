use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

use crate::backend::application::helper::setup_db;
use living_voices::backend::domain::character::{Character, CharacterStatus};
use living_voices::backend::persistence::adventure_persistence::insert_adventure;
use living_voices::backend::persistence::characters_persistence::{
    insert_character, update_appearance_current_outfit, update_appearance_current_pose,
    update_appearance_identity_prompt, update_character,
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
    character.add_trait("refers to everyone as \"love\"".into());
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
    assert_eq!(rows[0], (0, "refers to everyone as \"love\"".to_string()));
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
async fn relations_are_free_text_on_the_character_row() {
    let (pool, _db) = setup_db().await;
    let adventure_id = setup_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");

    let mut mara = Character::new(
        Uuid::new_v4(),
        "Mara".into(),
        adventure_id,
        "singer",
        "",
        CharacterStatus::Present,
        false,
    );
    mara.add_relation("Brother of Otho".to_string());
    insert_character(&mut conn, adventure_id, &mara)
        .await
        .expect("insert mara");

    let inserted: String = sqlx::query_scalar("SELECT relations FROM character WHERE id = ?1")
        .bind(mara.id.to_string())
        .fetch_one(&pool)
        .await
        .expect("relations column");
    assert_eq!(inserted, r#"["Brother of Otho"]"#);

    // An edit through update_character has to survive too, not just the insert.
    mara.relations = vec!["Father of the Duke".to_string()];
    update_character(&mut conn, &mara)
        .await
        .expect("update character");

    let updated: String = sqlx::query_scalar("SELECT relations FROM character WHERE id = ?1")
        .bind(mara.id.to_string())
        .fetch_one(&pool)
        .await
        .expect("relations column");
    assert_eq!(updated, r#"["Father of the Duke"]"#);
}

#[tokio::test]
async fn message_example_is_free_text_and_round_trips() {
    let (pool, _db) = setup_db().await;
    let adventure_id = setup_adventure(&pool).await;
    let mut conn = pool.acquire().await.expect("acquire");

    // given a character with voice samples containing the characters that break
    // a naive JSON payload
    let mut mara = Character::new(
        Uuid::new_v4(),
        "Mara".into(),
        adventure_id,
        "singer",
        "",
        CharacterStatus::Present,
        false,
    );
    mara.message_example = Some("\"The tide's early.\"\nYou: \"Is that bad?\"".to_string());

    // when
    insert_character(&mut conn, adventure_id, &mara)
        .await
        .expect("insert mara");

    // then the raw text lands in its column, quotes and all
    let inserted: Option<String> =
        sqlx::query_scalar("SELECT message_example FROM character WHERE id = ?1")
            .bind(mara.id.to_string())
            .fetch_one(&pool)
            .await
            .expect("message_example column");
    assert_eq!(
        inserted.as_deref(),
        Some("\"The tide's early.\"\nYou: \"Is that bad?\"")
    );

    // when the author rewrites it
    mara.message_example = Some("\"Nothing out here but weather.\"".to_string());
    update_character(&mut conn, &mara)
        .await
        .expect("update character");

    // then the edit survives too, not just the insert
    let updated: Option<String> =
        sqlx::query_scalar("SELECT message_example FROM character WHERE id = ?1")
            .bind(mara.id.to_string())
            .fetch_one(&pool)
            .await
            .expect("message_example column");
    assert_eq!(
        updated.as_deref(),
        Some("\"Nothing out here but weather.\"")
    );
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

    update_appearance_identity_prompt(&mut conn, character.id, "identity prompt")
        .await
        .expect("update identity prompt");
    update_appearance_current_outfit(&mut conn, character.id, "outfit prompt")
        .await
        .expect("update outfit prompt");
    update_appearance_current_pose(&mut conn, character.id, "pose prompt")
        .await
        .expect("update pose prompt");

    let (identity, outfit, pose): (Option<String>, Option<String>, Option<String>) =
        sqlx::query_as(
            "SELECT appearance_identity_prompt, appearance_current_outfit,
                    appearance_current_pose
             FROM character WHERE id = ?1",
        )
        .bind(character.id.to_string())
        .fetch_one(&pool)
        .await
        .expect("query character");

    assert_eq!(identity.as_deref(), Some("identity prompt"));
    assert_eq!(outfit.as_deref(), Some("outfit prompt"));
    assert_eq!(pose.as_deref(), Some("pose prompt"));
}
