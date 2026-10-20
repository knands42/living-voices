use sqlx::{Row, SqliteConnection};
use uuid::Uuid;

use crate::backend::persistence::error::PersistenceError;

/// How many messages sit after `after_seq` in the log.
pub async fn count_uncovered(
    conn: &mut SqliteConnection,
    adventure_id: Uuid,
    after_seq: u64,
) -> Result<u64, PersistenceError> {
    let row = sqlx::query("SELECT COUNT(*) AS c FROM message WHERE adventure_id = ?1 AND seq > ?2")
        .bind(adventure_id.to_string())
        .bind(after_seq as i64)
        .fetch_one(&mut *conn)
        .await?;

    Ok(row.get::<i64, _>("c") as u64)
}
