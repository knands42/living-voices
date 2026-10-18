use sqlx::SqliteConnection;
use uuid::Uuid;

use crate::backend::{
    domain::adventure::Adventure,
    persistence::{
        db::from_timestamp_to_str,
        error::PersistenceError,
        tags::{ensure_tag, normalize},
    },
};

pub async fn insert_adventure(
    conn: &mut SqliteConnection,
    adventure: &Adventure,
) -> Result<(), PersistenceError> {
    sqlx::query(
        "INSERT INTO adventure (id, title, description, genre, tone, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(adventure.id.to_string())
    .bind(&adventure.title)
    .bind(&adventure.description)
    .bind(&adventure.genre)
    .bind(&adventure.tone)
    .bind(from_timestamp_to_str(&adventure.created_at))
    .bind(from_timestamp_to_str(&adventure.updated_at))
    .execute(&mut *conn)
    .await?;

    for tag in &adventure.tags {
        ensure_tag(conn, tag).await?;
        attach_tag(conn, adventure.id, tag).await?;
    }

    Ok(())
}

pub async fn attach_tag(
    conn: &mut SqliteConnection,
    adventure_id: Uuid,
    tag: &str,
) -> Result<(), PersistenceError> {
    sqlx::query("INSERT OR IGNORE INTO adventure_tag (adventure_id, tag) VALUES (?1, ?2)")
        .bind(adventure_id.to_string())
        .bind(normalize(tag))
        .execute(conn)
        .await?;

    Ok(())
}
