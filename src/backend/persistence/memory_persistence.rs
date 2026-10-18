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

    let summaries = rows
        .iter()
        .map(decode_summary)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(summaries)
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

    let facts = fact_rows
        .iter()
        .map(decode_fact)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(facts)
}

// TODO: Standardize to the same pattern used on `message_persistence`
fn decode_summary(row: &sqlx::sqlite::SqliteRow) -> Result<Summary, PersistenceError> {
    let id = from_str_to_uuid(row.get::<String, _>("id"))?;
    let adventure_id = from_str_to_uuid(row.get::<String, _>("adventure_id"))?;
    Ok(Summary {
        id,
        adventure_id,
        text: row.get("text"),
        from_seq: row.get("from_seq"),
        to_seq: row.get("to_seq"),
        created_at: from_str_to_timestamp(&row.get::<String, _>("created_at"))?,
    })
}

fn decode_fact(row: &sqlx::sqlite::SqliteRow) -> Result<Fact, PersistenceError> {
    let id = from_str_to_uuid(row.get::<String, _>("id"))?;
    let adventure_id = from_str_to_uuid(row.get::<String, _>("adventure_id"))?;
    let source_summary_id = from_str_to_uuid(row.get::<String, _>("source_summary_id"))?;
    Ok(Fact {
        id,
        adventure_id,
        source_summary_id,
        text: row.get("text"),
        created_at: from_str_to_timestamp(&row.get::<String, _>("created_at"))?,
    })
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
