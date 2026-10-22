use sqlx::{Row, SqliteConnection};
use uuid::Uuid;

use crate::backend::{domain::memory::Act, persistence::error::PersistenceError};

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

pub async fn get_active_candidates(
    conn: &mut SqliteConnection,
    adventure_id: &str,
) -> Result<Vec<String>, PersistenceError> {
    let row = sqlx::query("SELECT characters_present AS c FROM state WHERE adventure_id = ?1")
        .bind(adventure_id)
        .fetch_one(&mut *conn)
        .await?;

    parse_characters_present(row.get("characters_present"))
}

pub async fn get_act(
    conn: &mut SqliteConnection,
    adventure_id: &str,
) -> Result<Vec<Act>, PersistenceError> {
    todo!()
}

fn parse_characters_present(chars: &str) -> Result<Vec<String>, PersistenceError> {
    serde_json::from_str(chars).map_err(PersistenceError::ParsingError)
}
