use sqlx::{Connection, QueryBuilder, Row, SqliteConnection};
use uuid::Uuid;

use crate::backend::{
    domain::memory::{Fact, Summary},
    persistence::{db::from_timestamp_to_str, error::PersistenceError},
};

/// The highest `to_seq` covered by any summary for this adventure, or `None`
pub async fn last_covered(conn: &mut SqliteConnection, adventure_id: Uuid) -> Option<u64> {
    let row = sqlx::query("SELECT MAX(to_seq) AS m FROM summary WHERE adventure_id = ?1")
        .bind(adventure_id.to_string())
        .fetch_optional(&mut *conn)
        .await
        .ok()??;

    row.get::<Option<i64>, _>("m").map(|v| v as u64)
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
    .bind(summary.from_seq)
    .bind(summary.to_seq)
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
