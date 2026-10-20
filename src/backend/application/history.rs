use sqlx::SqliteConnection;
use uuid::Uuid;

use crate::backend::persistence::error::PersistenceError;

/// Assembles the prompt-context ("history") block handed to the chat provider.
pub struct HistoryBuilder;

impl HistoryBuilder {
    pub async fn build(
        conn: &mut SqliteConnection,
        adventure_id: Uuid,
        summary_limit: u64,
        fact_limit: u64,
    ) -> Result<String, PersistenceError> {
        todo!()
    }
}
