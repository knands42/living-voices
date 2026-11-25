use sqlx::SqliteConnection;

use crate::backend::PersistenceError;
use crate::backend::application::context::ContextData;
use crate::backend::application::port::chat_port::{ChatPort, NarratorPort};
use crate::backend::application::port::memory_port::{ActCloserPort, MemoryDistillerPort};
use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::memory::Act;
use crate::backend::domain::message::Message;
use crate::backend::persistence::act_persistence::get_active_act;
use crate::backend::persistence::message_persistence::{
    count_since_last_narration, fetch_last_n, fetch_since,
};
use crate::backend::persistence::state_persistence::get_active_candidates;

use super::message_manager::{MessageManager, MessageManagerError};

pub(super) struct MessageContext {
    pub(super) adventure: Adventure,
    pub(super) present_names: Vec<String>,
    pub(super) messages_since_narration: Option<u8>,
    pub(super) active_act: Option<Act>,
    pub(super) uncovered_messages: Vec<Message>,
}

impl<
    CHAT: ChatPort,
    NARRATOR: NarratorPort,
    MEMORY: MemoryDistillerPort + Clone + 'static,
    ACT: ActCloserPort + Clone + 'static,
> MessageManager<CHAT, NARRATOR, MEMORY, ACT>
{
    pub(super) async fn gather_data(&self) -> Result<MessageContext, MessageManagerError> {
        let adventure = {
            let guard = self.state.read()?;
            guard
                .current_adventure
                .as_ref()
                .ok_or(MessageManagerError::NoAdventure)?
                .clone()
        };

        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        let present_names = get_active_candidates(&mut conn, &adventure.id.to_string()).await?;
        let messages_since_narration = count_since_last_narration(&mut conn, adventure.id).await?;
        let active_act = get_active_act(&mut conn, adventure.id).await?;
        let uncovered_messages = self.load_uncovered(&mut conn, &active_act).await?;

        Ok(MessageContext {
            adventure,
            present_names,
            messages_since_narration,
            active_act,
            uncovered_messages,
        })
    }

    pub(super) async fn load_context(
        &self,
        adventure: &Adventure,
    ) -> Result<ContextData, MessageManagerError> {
        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        let data = ContextData::load(&mut conn, adventure.id).await?;

        Ok(data)
    }

    async fn load_uncovered(
        &self,
        conn: &mut SqliteConnection,
        active_act: &Option<Act>,
    ) -> Result<Vec<Message>, MessageManagerError> {
        let window = self.memory_manager.recent_messages_window();
        load_uncovered_messages(conn, active_act, window).await
    }
}

async fn load_uncovered_messages(
    conn: &mut SqliteConnection,
    active_act: &Option<Act>,
    window: u16,
) -> Result<Vec<Message>, MessageManagerError> {
    let Some(act) = active_act else {
        log::debug!("no active act; nothing uncovered to carry");
        return Ok(vec![]);
    };

    let uncovered = fetch_since(conn, act.adventure_id, act.digest_to_seq).await?;

    if act.ordinal != 0 && uncovered.len() < window as usize {
        return Ok(fetch_last_n(conn, act.adventure_id, window).await?);
    }

    Ok(uncovered)
}

#[cfg(test)]
mod load_uncovered_tests {
    use super::*;
    use crate::backend::persistence::db::open_pool;
    use crate::backend::persistence::message_persistence::insert_messages;
    use crate::backend::persistence::migrator::run_migrations;
    use sqlx::SqlitePool;
    use std::path::{Path, PathBuf};
    use uuid::Uuid;

    struct TempDb(PathBuf);
    impl Drop for TempDb {
        fn drop(&mut self) {
            let base = self.0.to_string_lossy().into_owned();
            for suffix in ["", "-wal", "-shm"] {
                let _ = std::fs::remove_file(format!("{base}{suffix}"));
            }
        }
    }

    async fn setup() -> (SqlitePool, TempDb, Uuid) {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("load_uncovered_test_{}.db", Uuid::new_v4()));
        let db = TempDb(path);
        let db_str = db.0.to_string_lossy().into_owned();

        let migrating = open_pool(&db_str).await.expect("open pool");
        run_migrations(&migrating).await.expect("run migrations");
        let pool = open_pool(&db_str).await.expect("reopen pool");

        let adventure_id = Uuid::new_v4();
        let mut conn = pool.acquire().await.expect("acquire");
        sqlx::query("INSERT INTO adventure (id, title) VALUES (?1, ?2)")
            .bind(adventure_id.to_string())
            .bind("Test adventure")
            .execute(&mut *conn)
            .await
            .expect("insert adventure");

        (pool, db, adventure_id)
    }

    async fn seed_messages(pool: &SqlitePool, adventure_id: Uuid, count: usize) -> Vec<Message> {
        let mut conn = pool.acquire().await.expect("acquire");
        let mut stored = Vec::with_capacity(count);
        for i in 0..count {
            let message = Message::new(adventure_id, None, None, format!("message {i}"), false);
            stored.extend(
                insert_messages(&mut conn, std::slice::from_ref(&message))
                    .await
                    .expect("insert message"),
            );
        }
        stored
    }

    fn last_seq(messages: &[Message]) -> u16 {
        messages
            .last()
            .and_then(|message| message.seq)
            .expect("a stored seq")
    }

    #[tokio::test]
    async fn returns_empty_without_an_active_act() {
        let (pool, _db, _adventure_id) = setup().await;
        let mut conn = pool.acquire().await.expect("acquire");

        let messages = load_uncovered_messages(&mut conn, &None, 50)
            .await
            .expect("load uncovered");

        assert!(messages.is_empty());
    }

    #[tokio::test]
    async fn first_act_with_a_thin_tail_is_not_backfilled() {
        let (pool, _db, adventure_id) = setup().await;
        let stored = seed_messages(&pool, adventure_id, 10).await;
        let mut conn = pool.acquire().await.expect("acquire");

        // first act (ordinal 0) with a watermark past every message: nothing uncovered
        let mut act = Act::new(adventure_id, 0, "goal");
        act.digest_to_seq = last_seq(&stored);

        let messages = load_uncovered_messages(&mut conn, &Some(act), 50)
            .await
            .expect("load uncovered");

        // the first act never borrows from earlier history
        assert!(messages.is_empty());
    }

    #[tokio::test]
    async fn fresh_later_act_backfills_the_recent_window() {
        let (pool, _db, adventure_id) = setup().await;
        let stored = seed_messages(&pool, adventure_id, 10).await;
        let mut conn = pool.acquire().await.expect("acquire");

        // a later act (ordinal 1) just started: its watermark is at its own first
        // message, so it has no uncovered tail
        let mut act = Act::new(adventure_id, 1, "goal");
        act.digest_to_seq = last_seq(&stored);

        let messages = load_uncovered_messages(&mut conn, &Some(act), 5)
            .await
            .expect("load uncovered");

        // it backfills the last `window` turns from the previous acts
        assert_eq!(messages.len(), 5);
        assert_eq!(messages.last().expect("a message").text, "message 9");
    }

    #[tokio::test]
    async fn later_act_with_a_full_tail_uses_its_own_tail() {
        let (pool, _db, adventure_id) = setup().await;
        let stored = seed_messages(&pool, adventure_id, 10).await;
        let mut conn = pool.acquire().await.expect("acquire");

        // a later act whose own uncovered tail already meets the window
        let mut act = Act::new(adventure_id, 1, "goal");
        act.digest_to_seq = stored[3].seq.expect("a stored seq");

        let messages = load_uncovered_messages(&mut conn, &Some(act), 3)
            .await
            .expect("load uncovered");

        // the six messages after the watermark are above the window of 3: no backfill
        assert_eq!(messages.len(), 6);
        assert_eq!(messages.first().expect("a message").text, "message 4");
    }
}
