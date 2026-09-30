use sqlx::SqliteConnection;

use crate::backend::persistence::error::PersistenceError;

pub async fn ensure_tag(conn: &mut SqliteConnection, tag: &str) -> Result<(), PersistenceError> {
    sqlx::query("INSERT OR IGNORE INTO tag (value) VALUES (?1)")
        .bind(normalize(tag))
        .execute(conn)
        .await?;

    Ok(())
}

pub fn normalize(tag: &str) -> String {
    tag.trim().to_lowercase()
}
