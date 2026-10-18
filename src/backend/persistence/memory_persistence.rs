use sqlx::{Connection, QueryBuilder, Row, SqliteConnection};
use uuid::Uuid;

use crate::backend::persistence::db::from_str_to_uuid;
use crate::backend::{
    domain::memory::{Fact, Summary},
    persistence::{
        db::{from_str_to_timestamp, from_timestamp_to_str},
        error::PersistenceError,
    },
};

struct SummaryRow {
    id: String,
    adventure_id: String,
    text: String,
    from_seq: i64,
    to_seq: i64,
    created_at: String,
}

impl SummaryRow {
    fn decode(row: &sqlx::sqlite::SqliteRow) -> Self {
        Self {
            id: row.get("id"),
            adventure_id: row.get("adventure_id"),
            text: row.get("text"),
            from_seq: row.get("from_seq"),
            to_seq: row.get("to_seq"),
            created_at: row.get("created_at"),
        }
    }
}

impl TryFrom<SummaryRow> for Summary {
    type Error = PersistenceError;

    fn try_from(row: SummaryRow) -> Result<Self, Self::Error> {
        Ok(Summary {
            id: from_str_to_uuid(row.id)?,
            adventure_id: from_str_to_uuid(row.adventure_id)?,
            text: row.text,
            from_seq: row.from_seq as u64,
            to_seq: row.to_seq as u64,
            created_at: from_str_to_timestamp(&row.created_at)?,
        })
    }
}

struct FactRow {
    id: String,
    adventure_id: String,
    source_summary_id: String,
    text: String,
    created_at: String,
}

impl FactRow {
    fn decode(row: &sqlx::sqlite::SqliteRow) -> Self {
        Self {
            id: row.get("id"),
            adventure_id: row.get("adventure_id"),
            source_summary_id: row.get("source_summary_id"),
            text: row.get("text"),
            created_at: row.get("created_at"),
        }
    }
}

impl TryFrom<FactRow> for Fact {
    type Error = PersistenceError;

    fn try_from(row: FactRow) -> Result<Self, Self::Error> {
        Ok(Fact {
            id: from_str_to_uuid(row.id)?,
            adventure_id: from_str_to_uuid(row.adventure_id)?,
            source_summary_id: from_str_to_uuid(row.source_summary_id)?,
            text: row.text,
            created_at: from_str_to_timestamp(&row.created_at)?,
        })
    }
}

pub async fn last_covered(conn: &mut SqliteConnection, adventure_id: Uuid) -> Option<u64> {
    let row = sqlx::query("SELECT MAX(to_seq) AS m FROM summary WHERE adventure_id = ?1")
        .bind(adventure_id.to_string())
        .fetch_optional(&mut *conn)
        .await
        .ok()??;

    row.get::<Option<i64>, _>("m").map(|v| v as u64)
}

/// Load the most recent summary for an adventure together with its facts
pub async fn latest_summaries(
    conn: &mut SqliteConnection,
    adventure_id: Uuid,
    limit: u64,
) -> Result<Vec<Summary>, PersistenceError> {
    let rows = sqlx::query(
        "SELECT id, adventure_id, text, from_seq, to_seq, created_at
         FROM summary
         WHERE adventure_id = ?1
         ORDER BY to_seq DESC
         LIMIT ?2",
    )
    .bind(adventure_id.to_string())
    .bind(limit as i64)
    .fetch_all(&mut *conn)
    .await?;

    rows.iter()
        .map(SummaryRow::decode)
        .map(Summary::try_from)
        .collect()
}

pub async fn latest_facts(
    conn: &mut SqliteConnection,
    summary_id: Uuid,
    limit: u64,
) -> Result<Vec<Fact>, PersistenceError> {
    let fact_rows = sqlx::query(
        "SELECT id, adventure_id, source_summary_id, text, created_at
         FROM fact
         WHERE summary_id = ?1
         ORDER BY created_at ASC
         LIMIT ?2",
    )
    .bind(summary_id.to_string())
    .bind(limit as i64)
    .fetch_all(&mut *conn)
    .await?;

    fact_rows
        .iter()
        .map(FactRow::decode)
        .map(Fact::try_from)
        .collect()
}

/// Insert a summary row.
pub async fn insert_summary(
    conn: &mut SqliteConnection,
    summary: &Summary,
) -> Result<(), PersistenceError> {
    sqlx::query(
        "INSERT INTO summary (id, adventure_id, text, from_seq, to_seq, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind(summary.id.to_string())
    .bind(summary.adventure_id.to_string())
    .bind(&summary.text)
    .bind(summary.from_seq as i64)
    .bind(summary.to_seq as i64)
    .bind(from_timestamp_to_str(&summary.created_at))
    .execute(&mut *conn)
    .await?;

    Ok(())
}

/// Insert every fact linked to its `source_summary_id` in one pass.
pub async fn insert_facts(
    conn: &mut SqliteConnection,
    facts: &[Fact],
) -> Result<(), PersistenceError> {
    if facts.is_empty() {
        return Ok(());
    }

    let mut tx = conn.begin().await?;

    let mut qb = QueryBuilder::new(
        "INSERT INTO fact (id, adventure_id, source_summary_id, text, created_at) ",
    );
    qb.push_values(facts, |mut b, fact| {
        b.push_bind(fact.id.to_string())
            .push_bind(fact.adventure_id.to_string())
            .push_bind(fact.source_summary_id.to_string())
            .push_bind(&fact.text)
            .push_bind(from_timestamp_to_str(&fact.created_at));
    });

    qb.build().execute(&mut *tx).await?;
    tx.commit().await?;

    Ok(())
}
