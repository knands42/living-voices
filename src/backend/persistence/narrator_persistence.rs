use sqlx::SqliteConnection;
use uuid::Uuid;

use crate::backend::{domain::narrator::Narrator, persistence::error::PersistenceError};

pub async fn insert_narrator(
    conn: &mut SqliteConnection,
    adventure_id: Uuid,
    narrator: &Narrator,
) -> Result<(), PersistenceError> {
    sqlx::query("INSERT INTO narrator (id, adventure_id, name) VALUES (?1, ?2, ?3)")
        .bind(narrator.id.to_string())
        .bind(adventure_id.to_string())
        .bind(&narrator.name)
        .execute(conn)
        .await?;

    Ok(())
}
