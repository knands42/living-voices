use std::collections::HashMap;

use sqlx::SqliteConnection;
use uuid::Uuid;

use crate::backend::{
    domain::character::Character,
    persistence::{
        error::PersistenceError,
        tags::{ensure_tag, normalize},
    },
};

pub async fn insert_character(
    conn: &mut SqliteConnection,
    adventure_id: Option<Uuid>,
    character: &Character,
) -> Result<(), PersistenceError> {
    sqlx::query(
        "INSERT INTO character (id, adventure_id, name, description, backstory, status, portrait_path)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(character.id.to_string())
    .bind(adventure_id.map(|id| id.to_string()))
    .bind(&character.name)
    .bind(&character.description)
    .bind(&character.backstory)
    .bind(&character.status.as_str())
    .bind(character.portrait_path.as_deref())
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
