use std::collections::HashMap;

use sqlx::sqlite::SqliteRow;
use sqlx::{FromRow, Row, SqliteConnection};
use uuid::Uuid;

use crate::backend::domain::character::CharacterStatus;
use crate::backend::persistence::db::from_str_to_uuid;
use crate::backend::{
    domain::character::Character,
    persistence::{
        error::PersistenceError,
        tags::{ensure_tag, normalize},
    },
};

struct CharacterRow {
    id: String,
    adventure_id: String,
    name: String,
    headline: String,
    backstory: String,
    appearance: Option<String>,
    appearance_identity_prompt: Option<String>,
    appearance_current_outfit: Option<String>,
    appearance_current_pose: Option<String>,
    status: String,
    portrait_path: Option<String>,
    is_player: bool,
}

impl FromRow<'_, SqliteRow> for CharacterRow {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.get("id"),
            adventure_id: row.get("adventure_id"),
            name: row.get("name"),
            headline: row.get("headline"),
            backstory: row.get("backstory"),
            appearance: row.get("appearance"),
            appearance_identity_prompt: row.get("appearance_identity_prompt"),
            appearance_current_outfit: row.get("appearance_current_outfit"),
            appearance_current_pose: row.get("appearance_current_pose"),
            status: row.get("status"),
            portrait_path: row.get("portrait_path"),
            is_player: row.get("is_player"),
        })
    }
}

impl TryFrom<CharacterRow> for Character {
    type Error = PersistenceError;
    fn try_from(value: CharacterRow) -> Result<Self, PersistenceError> {
        let character_id = from_str_to_uuid(value.id.as_str())?;
        let adventure_id = from_str_to_uuid(value.adventure_id.as_str())?;
        let mut character = Character::new(
            character_id,
            value.name,
            adventure_id,
            value.headline,
            value.backstory,
            CharacterStatus::from_str(value.status.as_str()),
            value.is_player,
        );
        character.appearance = value.appearance;
        character.appearance_identity_prompt = value.appearance_identity_prompt;
        character.appearance_current_outfit = value.appearance_current_outfit;
        character.appearance_current_pose = value.appearance_current_pose;
        character.portrait_path = value.portrait_path;
        Ok(character)
    }
}

pub async fn insert_character(
    conn: &mut SqliteConnection,
    adventure_id: Uuid,
    character: &Character,
) -> Result<(), PersistenceError> {
    sqlx::query(
        "INSERT INTO character (id, adventure_id, name, headline, backstory, appearance, appearance_identity_prompt, appearance_current_outfit, appearance_current_pose, status, portrait_path, is_player)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
    )
    .bind(character.id.to_string())
    .bind(adventure_id.to_string())
    .bind(&character.name)
    .bind(&character.headline)
    .bind(&character.backstory)
    .bind(&character.appearance)
    .bind(&character.appearance_identity_prompt)
    .bind(&character.appearance_current_outfit)
    .bind(&character.appearance_current_pose)
    .bind(character.status.as_str())
    .bind(character.portrait_path.as_deref())
    .bind(character.is_player)
    .execute(&mut *conn)
    .await?;

    // Traits are an ordered list, so position is part of the identity
    for (position, trait_) in character.traits.iter().enumerate() {
        insert_trait(conn, character.id, position as u16, trait_).await?;
    }

    for tag in &character.tags {
        ensure_tag(conn, tag).await?;
        attach_tag(conn, character.id, tag).await?;
    }

    Ok(())
}

pub async fn insert_default_player_character(
    conn: &mut SqliteConnection,
    character_id: Uuid,
    adventure_id: Uuid,
) -> Result<Character, PersistenceError> {
    const DEFAULT_NAME: &str = "You";
    let row = sqlx::query_as::<_, CharacterRow>(
        "INSERT INTO character (id, adventure_id, name, is_player)
         VALUES (?1, ?2, ?3, ?4)
         RETURNING *",
    )
    .bind(character_id.to_string())
    .bind(adventure_id.to_string())
    .bind(DEFAULT_NAME)
    .bind(true)
    .fetch_one(&mut *conn)
    .await?;

    Character::try_from(row)
}

pub async fn insert_trait(
    conn: &mut SqliteConnection,
    character_id: Uuid,
    position: u16,
    trait_: &str,
) -> Result<(), PersistenceError> {
    sqlx::query("INSERT INTO character_trait (character_id, position, value) VALUES (?1, ?2, ?3)")
        .bind(character_id.to_string())
        .bind(position)
        .bind(trait_)
        .execute(conn)
        .await?;

    Ok(())
}

pub async fn attach_tag(
    conn: &mut SqliteConnection,
    character_id: Uuid,
    tag: &str,
) -> Result<(), PersistenceError> {
    sqlx::query("INSERT OR IGNORE INTO character_tag (character_id, tag) VALUES (?1, ?2)")
        .bind(character_id.to_string())
        .bind(normalize(tag))
        .execute(conn)
        .await?;

    Ok(())
}

pub async fn insert_relation(
    conn: &mut SqliteConnection,
    character_id: Uuid,
    related_character_id: Uuid,
    relation: &str,
) -> Result<(), PersistenceError> {
    sqlx::query(
        "INSERT INTO character_relation (character_id, related_character_id, relation)
         VALUES (?1, ?2, ?3)",
    )
    .bind(character_id.to_string())
    .bind(related_character_id.to_string())
    .bind(relation)
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn insert_relations(
    conn: &mut SqliteConnection,
    character_id: Uuid,
    relations: &HashMap<Uuid, String>,
) -> Result<(), PersistenceError> {
    for (related_character_id, relation) in relations {
        insert_relation(conn, character_id, *related_character_id, relation).await?;
    }

    Ok(())
}

pub async fn update_appearance_identity_prompt(
    conn: &mut SqliteConnection,
    character_id: Uuid,
    appearance_identity_prompt: &str,
) -> Result<(), PersistenceError> {
    sqlx::query("UPDATE character SET appearance_identity_prompt = ?1 WHERE id = ?2")
        .bind(appearance_identity_prompt)
        .bind(character_id.to_string())
        .execute(conn)
        .await?;

    Ok(())
}

pub async fn update_appearance_current_outfit(
    conn: &mut SqliteConnection,
    character_id: Uuid,
    appearance_current_outfit: &str,
) -> Result<(), PersistenceError> {
    sqlx::query("UPDATE character SET appearance_current_outfit = ?1 WHERE id = ?2")
        .bind(appearance_current_outfit)
        .bind(character_id.to_string())
        .execute(conn)
        .await?;

    Ok(())
}

pub async fn update_appearance_current_pose(
    conn: &mut SqliteConnection,
    character_id: Uuid,
    appearance_current_pose: &str,
) -> Result<(), PersistenceError> {
    sqlx::query("UPDATE character SET appearance_current_pose = ?1 WHERE id = ?2")
        .bind(appearance_current_pose)
        .bind(character_id.to_string())
        .execute(conn)
        .await?;

    Ok(())
}

pub async fn update_character(
    conn: &mut SqliteConnection,
    character: &Character,
) -> Result<(), PersistenceError> {
    sqlx::query(
        "UPDATE character
         SET name = ?1, headline = ?2, backstory = ?3, appearance = ?4,
             appearance_identity_prompt = ?5, appearance_current_outfit = ?6,
             appearance_current_pose = ?7, status = ?8, portrait_path = ?9, is_player = ?10
         WHERE id = ?11",
    )
    .bind(&character.name)
    .bind(&character.headline)
    .bind(&character.backstory)
    .bind(&character.appearance)
    .bind(&character.appearance_identity_prompt)
    .bind(&character.appearance_current_outfit)
    .bind(&character.appearance_current_pose)
    .bind(character.status.as_str())
    .bind(character.portrait_path.as_deref())
    .bind(character.is_player)
    .bind(character.id.to_string())
    .execute(&mut *conn)
    .await?;

    sqlx::query("DELETE FROM character_trait WHERE character_id = ?1")
        .bind(character.id.to_string())
        .execute(&mut *conn)
        .await?;
    for (position, trait_) in character.traits.iter().enumerate() {
        insert_trait(conn, character.id, position as u16, trait_).await?;
    }

    sqlx::query("DELETE FROM character_tag WHERE character_id = ?1")
        .bind(character.id.to_string())
        .execute(&mut *conn)
        .await?;
    for tag in &character.tags {
        ensure_tag(conn, tag).await?;
        attach_tag(conn, character.id, tag).await?;
    }

    Ok(())
}
