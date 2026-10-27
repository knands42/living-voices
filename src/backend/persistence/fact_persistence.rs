use sqlx::{QueryBuilder, SqliteConnection};

use crate::backend::persistence::db::from_timestamp_to_str;
use crate::backend::{domain::memory::Fact, persistence::error::PersistenceError};

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
