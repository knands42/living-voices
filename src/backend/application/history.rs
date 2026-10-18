use sqlx::SqliteConnection;
use uuid::Uuid;

use crate::backend::domain::memory::{Fact, Summary};
use crate::backend::domain::message::Message;
use crate::backend::persistence::error::PersistenceError;
use crate::backend::persistence::memory_persistence::{
    last_covered, latest_facts, latest_summaries,
};
use crate::backend::persistence::message_persistence::fetch_since;

const SUMMARY_LIMIT: u64 = 5;
const FACT_LIMIT: u64 = 5;

/// Assembles the prompt-context ("history") block handed to the chat provider.
pub struct HistoryBuilder;

impl HistoryBuilder {
    pub async fn build(
        conn: &mut SqliteConnection,
        adventure_id: Uuid,
    ) -> Result<String, PersistenceError> {
        let summaries = latest_summaries(conn, adventure_id, SUMMARY_LIMIT).await?;
        let facts = latest_facts(conn, adventure_id, FACT_LIMIT).await?;

        let after_seq = last_covered(conn, adventure_id).await.unwrap_or(0);
        let uncovered = fetch_since(conn, adventure_id, after_seq).await?;

        let mut history = String::new();
        history.push_str(&Summary::render(&summaries).unwrap_or_default());
        history.push_str(&Fact::render(&facts).unwrap_or_default());
        history.push_str(&Message::render(&uncovered).unwrap_or_default());

        Ok(history)
    }
}
