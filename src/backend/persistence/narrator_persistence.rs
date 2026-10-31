use sqlx::SqliteConnection;
use uuid::Uuid;

use crate::backend::{domain::narrator::Narrator, persistence::error::PersistenceError};

pub async fn insert_default_narrator(
    conn: &mut SqliteConnection,
    id: Uuid,
    adventure_id: Uuid,
) -> Result<(), PersistenceError> {
    sqlx::query("INSERT INTO narrator (id, adventure_id) VALUES (?1, ?2)")
        .bind(id.to_string())
        .bind(adventure_id.to_string())
        .execute(conn)
        .await?;

    Ok(())
}
