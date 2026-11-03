use sqlx::sqlite::SqliteRow;
use sqlx::{Error, FromRow, Row, SqliteConnection};
use uuid::Uuid;

use crate::backend::persistence::db::from_str_to_uuid;
use crate::backend::{domain::lore::Lore, persistence::error::PersistenceError};

struct LoreRow {
    id: String,
    adventure_id: String,
    text: String,
    created_at: String,
    updated_at: String,
}

impl FromRow<'_, SqliteRow> for LoreRow {
    fn from_row(row: &'_ SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.get("id"),
            adventure_id: row.get("adventure_id"),
            text: row.get("text"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }
}

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

pub async fn insert_default_lore(
    conn: &mut SqliteConnection,
    id: Uuid,
    adventure_id: Uuid,
) -> Result<Lore, PersistenceError> {
    let row = sqlx::query_as::<_, LoreRow>(
        "INSERT INTO lore (id, adventure_id) VALUES (?1, ?2) RETURNING *",
    )
    .bind(id.to_string())
    .bind(adventure_id.to_string())
    .fetch_one(conn)
    .await?;

    Lore::try_from(row)
}

impl TryFrom<LoreRow> for Lore {
    type Error = PersistenceError;
    fn try_from(row: LoreRow) -> Result<Self, Self::Error> {
        Ok(Lore {
            id: from_str_to_uuid(&row.id)?,
            text: row.text,
        })
    }
}
