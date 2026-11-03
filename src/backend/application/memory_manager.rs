use sqlx::{Acquire, SqliteConnection, SqlitePool};
use thiserror::Error;
use uuid::Uuid;

use crate::backend::PersistenceError;
use crate::backend::application::context::ContextBuilder;
use crate::backend::application::port::memory_port::{
    ActCloser, ClosedAct, ClosedActError, DistillError, DistilledFact, DistilledMemory,
    MemoryDistiller,
};
use crate::backend::application::state::{SharedState, StateError};
use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::memory::{Act, ActStatus};
use crate::backend::domain::message::Message;
use crate::backend::persistence::act_persistence::{
    activate_next_act, close_act, get_active_act, update_act_running_digest,
};
use crate::backend::persistence::fact_persistence::insert_facts;
use crate::backend::persistence::message_persistence::fetch_since;
use crate::backend::persistence::state_persistence::upsert_state;
use crate::config::MemorySettings;

#[derive(Error, Debug)]
pub enum MemoryManagerError {
    #[error("failed to compress the memory {0}")]
    CompressionFailed(#[from] DistillError),
    #[error("failed to close the act {0}")]
    ActClosureError(#[from] ClosedActError),
    #[error("state error: {0}")]
    State(#[from] StateError),
    #[error("database error: {0}")]
    DatabaseError(#[from] PersistenceError),
    #[error("message for {0} fetched from the store without a sequence number")]
    MissingSequence(String),
}

#[derive(Clone, Debug)]
pub struct MemoryManager<D: MemoryDistiller, C: ActCloser> {
    state: SharedState,
    pool: SqlitePool,
    distiller: D,
    closer: C,
    memory_settings: MemorySettings,
}

impl<D: MemoryDistiller, C: ActCloser> MemoryManager<D, C> {
    pub fn new(
        state: SharedState,
        pool: SqlitePool,
        distiller: D,
        closer: C,
        memory_settings: MemorySettings,
    ) -> Self {
        Self {
            state,
            pool,
            distiller,
            closer,
            memory_settings,
        }
    }

    pub fn needs_compression(
        &self,
        uncovered_messages_len: usize,
    ) -> Result<bool, MemoryManagerError> {
        Ok(uncovered_messages_len >= self.memory_settings.compress_after_threshold as usize)
    }

    /// Compress every message not yet covered by the latest summary.
    pub async fn compress_and_update(&self) -> Result<(), MemoryManagerError> {
        let adventure = {
            let guard = self.state.read()?;
            let Some(adventure) = guard.current_adventure.as_ref() else {
                return Ok(());
            };
            adventure.clone()
        };

        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        let Some((act, uncovered, digest_to_seq)) =
            self.load_uncovered(&mut conn, adventure.id).await?
        else {
            return Ok(());
        };

        let prompt =
            ContextBuilder::build_distill_prompt(&mut conn, &adventure, &uncovered).await?;
        let distilled = self.distiller.distill(&prompt).await?;

        self.persist_distilled(&mut conn, &adventure, &act, distilled, digest_to_seq)
            .await
    }

    pub async fn close_current_act(
        &self,
        act: &Act,
        uncovered_messages: &[Message],
    ) -> Result<(), MemoryManagerError> {
        let adventure = {
            let guard = self.state.read()?;
            let Some(adventure) = guard.current_adventure.as_ref() else {
                return Ok(());
            };
            adventure.clone()
        };

        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;

        // Distill first so facts/state can be up to date
        let distilled = {
            let prompt =
                ContextBuilder::build_distill_prompt(&mut conn, &adventure, &uncovered_messages)
                    .await?;
            self.distiller.distill(&prompt).await?
        };

        let closed = {
            let prompt =
                ContextBuilder::build_act_close_prompt(&mut conn, &adventure, &uncovered_messages)
                    .await?;
            self.closer.close_act(&prompt).await?
        };

        let digest_to_seq = if let Some(last) = uncovered_messages.last() {
            last.seq
                .ok_or_else(|| MemoryManagerError::MissingSequence(last.text.clone()))?
        } else {
            0
        };

        self.persist_close(
            &mut conn,
            &adventure,
            &act,
            distilled,
            closed,
            digest_to_seq,
        )
        .await
    }

    async fn persist_close(
        &self,
        conn: &mut sqlx::SqliteConnection,
        adventure: &Adventure,
        act: &Act,
        distilled: DistilledMemory,
        closed: ClosedAct,
        to_seq: u16,
    ) -> Result<(), MemoryManagerError> {
        let mut tx = conn.begin().await.map_err(PersistenceError::from)?;

        let facts = DistilledFact::into_facts(adventure.id, act.id, distilled.facts);
        insert_facts(&mut tx, &facts).await?;

        let state = distilled.state.into_state(adventure.id);
        upsert_state(&mut tx, &state).await?;

        update_act_running_digest(&mut tx, act.id, &distilled.running_digest, to_seq).await?;

        close_act(
            &mut tx,
            act.id,
            &closed.index_line,
            &closed.digest,
            &ActStatus::from(closed.status).to_string(),
            to_seq,
        )
        .await?;

        let new_sequence = to_seq + 1;
        let next_act = activate_next_act(&mut tx, adventure.id, new_sequence).await?;

        tx.commit().await.map_err(PersistenceError::from)?;

        {
            let mut guard = self.state.write()?;
            guard.current_act = next_act;
            guard.current_state = Some(state);
            guard.current_facts = facts;
        }
        Ok(())
    }

    async fn persist_distilled(
        &self,
        conn: &mut sqlx::SqliteConnection,
        adventure: &Adventure,
        act: &Act,
        distilled: DistilledMemory,
        digest_to_seq: u16,
    ) -> Result<(), MemoryManagerError> {
        let mut tx = conn.begin().await.map_err(PersistenceError::from)?;

        let facts = DistilledFact::into_facts(adventure.id, act.id, distilled.facts);
        insert_facts(&mut tx, &facts).await?;

        let state = distilled.state.into_state(adventure.id);
        upsert_state(&mut tx, &state).await?;

        update_act_running_digest(&mut tx, act.id, &distilled.running_digest, digest_to_seq)
            .await?;

        tx.commit().await.map_err(PersistenceError::from)?;

        let refreshed_act = get_active_act(conn, adventure.id).await?;
        {
            let mut guard = self.state.write()?;
            guard.current_act = refreshed_act;
            guard.current_state = Some(state);
            guard.current_facts.extend(facts);
        }
        Ok(())
    }

    async fn load_uncovered(
        &self,
        conn: &mut SqliteConnection,
        adventure_id: Uuid,
    ) -> Result<Option<(Act, Vec<Message>, u16)>, MemoryManagerError> {
        let Some(act) = get_active_act(conn, adventure_id).await? else {
            return Ok(None);
        };

        let uncovered = fetch_since(conn, adventure_id, act.digest_to_seq).await?;
        let Some(last) = uncovered.last() else {
            return Ok(None);
        };

        let digest_to_seq = last
            .seq
            .ok_or_else(|| MemoryManagerError::MissingSequence(last.text.clone()))?;

        Ok(Some((act, uncovered, digest_to_seq)))
    }
}
