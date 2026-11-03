use crate::backend::domain::narrator::Narrator;
use crate::backend::persistence::db::from_str_to_uuid;
use crate::backend::persistence::error::PersistenceError;
use sqlx::sqlite::SqliteRow;
use sqlx::{Error, FromRow, Row, SqliteConnection};
use uuid::Uuid;

struct NarratorRow {
    id: String,
    adventure_id: String,
    name: String,
    tone: String,
    created_at: String,
    updated_at: String,
}

impl FromRow<'_, SqliteRow> for NarratorRow {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.get("id"),
            adventure_id: row.get("adventure_id"),
            name: row.get("name"),
            tone: row.get("tone"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }
}

pub async fn insert_default_narrator(
    conn: &mut SqliteConnection,
    id: Uuid,
    adventure_id: Uuid,
) -> Result<Narrator, PersistenceError> {
    let row = sqlx::query_as::<_, NarratorRow>(
        "INSERT INTO narrator (id, adventure_id) VALUES (?1, ?2) RETURNING *",
    )
    .bind(id.to_string())
    .bind(adventure_id.to_string())
    .fetch_one(conn)
    .await?;

    Narrator::try_from(row)
}

impl TryFrom<NarratorRow> for Narrator {
    type Error = PersistenceError;
    fn try_from(value: NarratorRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: from_str_to_uuid(&value.id)?,
            adventure_id: from_str_to_uuid(&value.adventure_id)?,
            name: value.name,
            tone: value.tone,
        })
    }
}
