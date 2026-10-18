use crate::backend::PersistenceError;
use crate::backend::domain::memory::{Fact, Summary};
use sqlx::SqliteConnection;
use uuid::Uuid;

pub async fn last_covered(conn: &mut SqliteConnection, adventure_id: Uuid) -> Option<u64> {
    Some(1)
}

pub async fn insert_summary(
    conn: &mut SqliteConnection,
    summary: &Summary,
) -> Result<(), PersistenceError> {
    Ok(())
}

pub async fn insert_facts(
    conn: &mut SqliteConnection,
    facts: &[Fact],
) -> Result<(), PersistenceError> {
    Ok(())
}
