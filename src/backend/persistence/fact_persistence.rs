use sqlx::sqlite::SqliteRow;
use sqlx::{Error, FromRow, Row, SqliteConnection};
use uuid::Uuid;

use crate::backend::{domain::memory::Fact, persistence::error::PersistenceError};

struct FactRow {
    facts: String,
}

impl FromRow<'_, SqliteRow> for FactRow {
    fn from_row(row: &SqliteRow) -> Result<FactRow, Error> {
        Ok(Self {
            facts: row.get("facts"),
        })
    }
}

fn parse_facts(text: &str) -> Result<Vec<Fact>, PersistenceError> {
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(text).map_err(PersistenceError::ParsingError)
}

pub async fn insert_default_facts(
    conn: &mut SqliteConnection,
    id: Uuid,
    adventure_id: Uuid,
) -> Result<Vec<Fact>, PersistenceError> {
    sqlx::query(
        "INSERT INTO fact (id, adventure_id) VALUES (?1, ?2)
         ON CONFLICT(adventure_id) DO NOTHING",
    )
    .bind(id.to_string())
    .bind(adventure_id.to_string())
    .execute(&mut *conn)
    .await?;

    Ok(Vec::new())
}

/// Replace the adventure's whole fact snapshot with `facts`.
pub async fn upsert_facts(
    conn: &mut SqliteConnection,
    adventure_id: Uuid,
    facts: &[Fact],
) -> Result<(), PersistenceError> {
    let encoded = serde_json::to_string(facts)?;

    sqlx::query(
        "INSERT INTO fact (id, adventure_id, facts) VALUES (?1, ?2, ?3)
         ON CONFLICT(adventure_id) DO UPDATE SET facts = excluded.facts",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(adventure_id.to_string())
    .bind(encoded)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

pub async fn get_facts(
    conn: &mut SqliteConnection,
    adventure_id: Uuid,
) -> Result<Vec<Fact>, PersistenceError> {
    let row = sqlx::query_as::<_, FactRow>("SELECT facts FROM fact WHERE adventure_id = ?1")
        .bind(adventure_id.to_string())
        .fetch_optional(&mut *conn)
        .await?;

    match row {
        Some(row) => parse_facts(&row.facts),
        None => Ok(Vec::new()),
    }
}
