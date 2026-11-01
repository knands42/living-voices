use sqlx::sqlite::SqliteRow;
use sqlx::{Error, FromRow, QueryBuilder, Row, SqliteConnection};
use uuid::Uuid;

use crate::backend::persistence::db::{
    from_str_to_timestamp, from_str_to_uuid, from_timestamp_to_str,
};
use crate::backend::{
    domain::memory::{Fact, Permanence},
    persistence::error::PersistenceError,
};

struct FactRow {
    id: String,
    adventure_id: String,
    subject: String,
    fact: String,
    act_id: String,
    permanence: String,
    created_at: String,
}

impl FromRow<'_, SqliteRow> for FactRow {
    fn from_row(row: &SqliteRow) -> Result<FactRow, Error> {
        Ok(Self {
            id: row.get("id"),
            adventure_id: row.get("adventure_id"),
            subject: row.get("subject"),
            fact: row.get("fact"),
            act_id: row.get("act_id"),
            permanence: row.get("permanence"),
            created_at: row.get("created_at"),
        })
    }
}

pub async fn insert_facts(
    conn: &mut SqliteConnection,
    facts: &[Fact],
) -> Result<(), PersistenceError> {
    if facts.is_empty() {
        return Ok(());
    }

    let mut qb = QueryBuilder::new(
        "INSERT INTO fact (id, adventure_id, subject, fact, act_id, permanence, created_at) ",
    );

    qb.push_values(facts, |mut row, fact| {
        row.push_bind(fact.id.to_string())
            .push_bind(fact.adventure_id.to_string())
            .push_bind(&fact.subject)
            .push_bind(&fact.fact)
            .push_bind(fact.act_id.to_string())
            .push_bind(fact.permanence.as_str())
            .push_bind(from_timestamp_to_str(&fact.created_at));
    });

    qb.build().execute(&mut *conn).await?;
    Ok(())
}

pub async fn get_facts(
    conn: &mut SqliteConnection,
    adventure_id: Uuid,
    act_id: Uuid,
) -> Result<Vec<Fact>, PersistenceError> {
    let rows = sqlx::query_as::<_, FactRow>(
        "SELECT id, adventure_id, subject, fact, act_id, permanence, created_at
         FROM fact
         WHERE adventure_id = ?1 AND act_id = ?2
         ORDER BY created_at ASC",
    )
    .bind(adventure_id.to_string())
    .bind(act_id.to_string())
    .fetch_all(&mut *conn)
    .await?;

    rows.into_iter().map(Fact::try_from).collect()
}

impl TryFrom<FactRow> for Fact {
    type Error = PersistenceError;

    fn try_from(row: FactRow) -> Result<Self, Self::Error> {
        let permanence = Permanence::from_str(&row.permanence).ok_or_else(|| {
            PersistenceError::InvalidAdventure {
                adventure: row.adventure_id.clone(),
                reason: format!("unknown fact permanence `{}`", row.permanence),
            }
        })?;

        Ok(Fact {
            id: from_str_to_uuid(&row.id)?,
            adventure_id: from_str_to_uuid(&row.adventure_id)?,
            subject: row.subject,
            fact: row.fact,
            act_id: from_str_to_uuid(&row.act_id)?,
            permanence,
            created_at: from_str_to_timestamp(&row.created_at)?,
        })
    }
}
