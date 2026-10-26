use sqlx::{Acquire, SqlitePool};
use thiserror::Error;

use crate::backend::PersistenceError;
use crate::backend::application::context::ContextBuilder;
use crate::backend::application::port::memory_port::{
    ActCloser, ClosedAct, ClosedActError, ClosedActStatus, DistillError, DistilledFact,
    DistilledMemory, DistilledState, FactPermanence, MemoryDistiller,
};
use crate::backend::application::state::{SharedState, StateError};
use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::memory::{Act, ActStatus, Fact, Permanence, State};
use crate::backend::persistence::memory_persistence::{
    close_act, get_active_act, insert_act, insert_facts, update_act_running_digest, upsert_state,
};
use crate::backend::persistence::message_persistence::{count_since_last_narration, fetch_since};

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
}

impl<D: MemoryDistiller, C: ActCloser> MemoryManager<D, C> {
    pub fn new(state: SharedState, pool: SqlitePool, distiller: D, closer: C) -> Self {
        Self {
            state,
            pool,
            distiller,
            closer,
        }
    }

    pub async fn needs_compression(&self, threshold: u8) -> Result<bool, MemoryManagerError> {
        let adventure_id = {
            let guard = self.state.read()?;
            let Some(adventure) = guard.current_adventure.as_ref() else {
                return Ok(false);
            };
            adventure.id
        };

        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        let count = count_since_last_narration(&mut conn, adventure_id).await?;
        Ok(count >= threshold)
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
        let Some(act) = get_active_act(&mut conn, adventure.id).await? else {
            return Ok(());
        };

        let after_seq = act.digest_to_seq.unwrap_or(0);
        let uncovered = fetch_since(&mut conn, adventure.id, after_seq + 1).await?;
        let Some(last) = uncovered.last() else {
            return Ok(());
        };
        let digest_to_seq = last
            .seq
            .ok_or_else(|| MemoryManagerError::MissingSequence(last.text.clone()))?;

        let prompt =
            ContextBuilder::build_distill_prompt(&mut conn, &adventure, &uncovered).await?;
        let distilled = self.distiller.distill(&prompt).await?;

        self.persist_distilled(&mut conn, &adventure, &act, distilled, digest_to_seq)
            .await
    }

    pub async fn close_current_act(&self) -> Result<(), MemoryManagerError> {
        let adventure = {
            let guard = self.state.read()?;
            let Some(adventure) = guard.current_adventure.as_ref() else {
                return Ok(());
            };
            adventure.clone()
        };

        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        let Some(act) = get_active_act(&mut conn, adventure.id).await? else {
            return Ok(());
        };

        let after_seq = act.digest_to_seq.unwrap_or(0);
        let uncovered = fetch_since(&mut conn, adventure.id, after_seq + 1).await?;
        let Some(last) = uncovered.last() else {
            return Ok(());
        };
        let digest_to_seq = last
            .seq
            .ok_or_else(|| MemoryManagerError::MissingSequence(last.text.clone()))?;

        // TODO: Do I really need to distill here?
        let distilled = {
            let prompt =
                ContextBuilder::build_distill_prompt(&mut conn, &adventure, &uncovered).await?;
            self.distiller.distill(&prompt).await?
        };

        let closed = {
            let prompt =
                ContextBuilder::build_act_close_prompt(&mut conn, &adventure, &uncovered).await?;
            self.closer.close_act(&prompt).await?
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

        let facts = Self::to_facts(adventure.id, act.id, &distilled.facts);
        insert_facts(&mut tx, &facts).await?;

        let state = Self::to_state(adventure.id, &distilled.state);
        upsert_state(&mut tx, &state).await?;

        update_act_running_digest(&mut tx, act.id, &distilled.running_digest, to_seq).await?;
        let e = ClosedActStatus::from(closed.status);

        close_act(
            &mut tx,
            act.id,
            &closed.index_line,
            &closed.digest,
            &Self::map_status(closed.status).to_string(),
            to_seq,
        )
        .await?;

        let next = Act::new(
            adventure.id,
            act.ordinal + 1,
            "TODO: author the next act's goal",
        )
        .with_status(ActStatus::Active);
        let mut next = next;
        next.from_seq = to_seq + 1;
        insert_act(&mut tx, &next).await?;

        tx.commit().await.map_err(PersistenceError::from)?;
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

        let facts = Self::to_facts(adventure.id, act.id, &distilled.facts);
        insert_facts(&mut tx, &facts).await?;

        let state = Self::to_state(adventure.id, &distilled.state);
        upsert_state(&mut tx, &state).await?;

        update_act_running_digest(&mut tx, act.id, &distilled.running_digest, digest_to_seq)
            .await?;

        tx.commit().await.map_err(PersistenceError::from)?;
        Ok(())
    }

    fn to_facts(
        adventure_id: uuid::Uuid,
        act_id: uuid::Uuid,
        facts: &[DistilledFact],
    ) -> Vec<Fact> {
        facts
            .iter()
            .map(|f| {
                Fact::new(
                    adventure_id,
                    f.subject.clone(),
                    f.fact.clone(),
                    act_id,
                    Self::map_permanence(f.permanence),
                )
            })
            .collect()
    }

    fn to_state(adventure_id: uuid::Uuid, snapshot: &DistilledState) -> State {
        State::new(adventure_id)
            .with_conditions(snapshot.conditions.clone())
            .with_characters_present(snapshot.characters_present.clone())
            .with_open_threads(snapshot.open_threads.clone())
            .with_day(snapshot.day.clone())
            .with_place(snapshot.place.clone())
    }

    fn map_permanence(permanence: FactPermanence) -> Permanence {
        match permanence {
            FactPermanence::Permanent => Permanence::Permanent,
            FactPermanence::Plot => Permanence::Plot,
        }
    }

    fn map_status(status: ClosedActStatus) -> ActStatus {
        match status {
            ClosedActStatus::Resolved => ActStatus::Resolved,
            ClosedActStatus::Failed => ActStatus::Failed,
            ClosedActStatus::Bypassed => ActStatus::Bypassed,
        }
    }
}
