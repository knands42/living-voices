use sqlx::SqliteConnection;
use uuid::Uuid;

use crate::backend::{domain::lore::Lore, persistence::error::PersistenceError};

pub async fn insert_lore(
    conn: &mut SqliteConnection,
    adventure_id: Uuid,
    lore: &Lore,
) -> Result<(), PersistenceError> {
    sqlx::query("INSERT INTO lore (id, adventure_id, text) VALUES (?1, ?2, ?3)")
        .bind(lore.id.to_string())
        .bind(adventure_id.to_string())
        .bind(&lore.text)
        .execute(conn)
        .await?;

    Ok(())
}
