use crate::backend::PersistenceError;
use crate::backend::application::context::{ContextBuilder, ContextData};
use crate::backend::application::director::Director;
use crate::backend::application::memory_manager::{MemoryManager, MemoryManagerError};
use crate::backend::application::port::chat_port::{
    ChatHandler, ChatReply, ChatResponseError, NarratorHandler,
};
use crate::backend::application::port::memory_port::{ActCloser, MemoryDistiller};
use crate::backend::application::state::{SharedState, StateError};
use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::character::Character;
use crate::backend::domain::memory::Act;
use crate::backend::domain::message::Message;
use crate::backend::persistence::act_persistence::{
    get_active_act, update_active_act_last_closure_score,
};
use crate::backend::persistence::message_persistence::{
    count_since_last_narration, fetch_last_n, fetch_since, insert_message,
};
use crate::backend::persistence::state_persistence::get_active_candidates;
use sqlx::{SqliteConnection, SqlitePool};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MessageManagerError {
    #[error("state error: {0}")]
    State(#[from] StateError),
    #[error("database error: {0}")]
    DatabaseError(#[from] PersistenceError),
    #[error("chat provider failed: {0}")]
    Chat(#[from] ChatResponseError),
    #[error("memory error: {0}")]
    Memory(#[from] MemoryManagerError),
    #[error("no adventure is currently loaded")]
    NoAdventure,
    #[error("no character is available to respond")]
    NoResponder,
    #[error("message for {0} fetched from the store without a sequence number")]
    MissingSequence(String),
}

struct TurnInput {
    adventure: Adventure,
    present_names: Vec<String>,
    messages_since_narration: Option<u8>,
    active_act: Option<Act>,
    uncovered_messages: Vec<Message>,
}

#[derive(Clone, Debug)]
pub struct MessageManager<
    CHAT: ChatHandler,
    NARRATOR: NarratorHandler,
    MEMORY: MemoryDistiller,
    ACT: ActCloser,
> {
    state: SharedState,
    pool: SqlitePool,
    chat_handler: CHAT,
    narrator_handler: NARRATOR,
    director: Director,
    memory_manager: MemoryManager<MEMORY, ACT>,
}

impl<
    CHAT: ChatHandler,
    NARRATOR: NarratorHandler,
    MEMORY: MemoryDistiller + Clone + 'static,
    ACT: ActCloser + Clone + 'static,
> MessageManager<CHAT, NARRATOR, MEMORY, ACT>
{
    pub fn new(
        state: SharedState,
        pool: SqlitePool,
        chat_handler: CHAT,
        narrator_handler: NARRATOR,
        director: Director,
        memory_manager: MemoryManager<MEMORY, ACT>,
    ) -> Self {
        Self {
            state,
            pool,
            chat_handler,
            narrator_handler,
            director,
            memory_manager,
        }
    }

    pub async fn send_message(&self, incoming: Message) -> Result<Message, MessageManagerError> {
        self.create_message(incoming.clone()).await?;

        let input_data = self.gather_data().await?;
        let chat_reply = self.respond(&incoming, &input_data).await?;
        let stored_reply = self.process_respond(chat_reply, input_data).await?;

        self.run_follow_ups(stored_reply.speaker_name.clone())
            .await?;

        Ok(stored_reply)
    }

    async fn run_follow_ups(&self, mut last_speaker: String) -> Result<(), MessageManagerError> {
        let mut steps = 6;

        for _ in 1..steps {
            let input = self.gather_data().await?;
            let candidates = input.adventure.active_characters(&input.present_names);

            if !self
                .director
                .should_a_conversation_follow_up(&candidates, steps)
            {
                return Ok(());
            }

            let data = {
                let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
                ContextData::load(&mut conn, input.adventure.id).await?
            };
            let prompt = ContextBuilder::build_follow_up_prompt(
                &input.adventure,
                &data,
                &input.uncovered_messages,
            );
            let reply = self.chat_handler.follow_up_respond(&prompt).await?;

            if reply.replier == last_speaker || reply.replier == Message::NARRATOR_SPEAKER_NAME {
                return Ok(());
            }

            let message = input
                .adventure
                .message_from_reply(&reply.replier, &reply.reply);
            let stored = self.create_message(message).await?;

            last_speaker = stored.speaker_name;
            steps += 1;
        }

        Ok(())
    }

    async fn gather_data(&self) -> Result<TurnInput, MessageManagerError> {
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

        Ok(TurnInput {
            adventure,
            present_names,
            messages_since_narration,
            active_act,
            uncovered_messages,
        })
    }

    async fn respond(
        &self,
        incoming: &Message,
        input: &TurnInput,
    ) -> Result<ChatReply, MessageManagerError> {
        let active_characters = input.adventure.active_characters(&input.present_names);
        let data = self.load_context(&input.adventure).await?;

        self.send_to_handler(
            &input.adventure,
            incoming,
            &active_characters,
            input.messages_since_narration,
            &input.uncovered_messages,
            &data,
        )
        .await
    }

    /// Loads the prompt inputs and hands the connection back before the caller
    /// awaits a model. Holding it across the call stalls every other beat on a pool
    /// that is many seconds wide.
    async fn load_context(
        &self,
        adventure: &Adventure,
    ) -> Result<ContextData, MessageManagerError> {
        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        let data = ContextData::load(&mut conn, adventure.id).await?;

        Ok(data)
    }

    async fn process_respond(
        &self,
        chat_reply: ChatReply,
        input: TurnInput,
    ) -> Result<Message, MessageManagerError> {
        let reply_message = input
            .adventure
            .message_from_reply(&chat_reply.replier, &chat_reply.reply);
        let stored_reply = self.create_message(reply_message).await?;

        self.check_and_update_current_act(chat_reply, &input, stored_reply.clone())
            .await?;

        Ok(stored_reply)
    }
}

impl<
    CHAT: ChatHandler,
    NARRATOR: NarratorHandler,
    MEMORY: MemoryDistiller + Clone + 'static,
    ACT: ActCloser + Clone + 'static,
> MessageManager<CHAT, NARRATOR, MEMORY, ACT>
{
    async fn create_message(&self, message: Message) -> Result<Message, MessageManagerError> {
        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        let stored = insert_message(&mut conn, &message).await?;

        Ok(stored)
    }

    async fn send_to_handler(
        &self,
        adventure: &Adventure,
        incoming: &Message,
        active_characters: &[&Character],
        messages_since_narration: Option<u8>,
        uncovered_messages: &[Message],
        data: &ContextData,
    ) -> Result<ChatReply, MessageManagerError> {
        if self.director.should_narrator_speak(
            active_characters,
            incoming,
            messages_since_narration,
        ) {
            let context = ContextBuilder::build_narrator_chat_prompt(
                adventure,
                &self.director,
                data,
                uncovered_messages,
            );
            let reply = self.narrator_handler.narrator_respond(&context).await?;
            return Ok(ChatReply {
                replier: Message::NARRATOR_SPEAKER_NAME.to_string(),
                reply,
                act_closure: None,
            });
        }

        if let Some(character) = self.director.pick_character(active_characters) {
            let context = ContextBuilder::build_single_character_chat_prompt(
                adventure,
                &self.director,
                data,
                character,
                uncovered_messages,
            );
            let reply = self.chat_handler.character_respond(&context).await?;
            return Ok(ChatReply {
                replier: character.name.clone(),
                reply,
                act_closure: None,
            });
        }

        let context = ContextBuilder::build_autonomous_chat_prompt(
            adventure,
            &self.director,
            data,
            uncovered_messages,
        );
        Ok(self.chat_handler.autonomous_respond(&context).await?)
    }

    fn check_and_compress_memory(&self, act: &Act, uncovered_messages: &[Message]) {
        match self
            .memory_manager
            .needs_compression(uncovered_messages.len())
        {
            Ok(false) => {}
            Ok(true) => {
                let memory = self.memory_manager.clone();
                let act = act.clone();
                let uncovered = uncovered_messages.to_vec();
                tokio::spawn(async move {
                    if let Err(err) = memory.compress_and_update(&act, &uncovered).await {
                        eprintln!("memory compression failed: {err}");
                    }
                });
            }
            Err(err) => eprintln!("memory compression check failed: {err}"),
        }
    }

    async fn check_and_update_current_act(
        &self,
        chat_reply: ChatReply,
        input: &TurnInput,
        last_message: Message,
    ) -> Result<(), MessageManagerError> {
        let score = chat_reply.act_closure.unwrap_or(0);

        let updated_act = {
            let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
            update_active_act_last_closure_score(&mut conn, input.adventure.id, score).await?
        };

        if should_close_act(score, &last_message, self.director.get_max_act_messages()) {
            return self.close_current_act(input).await;
        }

        self.state.write()?.current_act = updated_act;

        if let Some(act) = input.active_act.as_ref() {
            self.check_and_compress_memory(act, &input.uncovered_messages);
        }

        Ok(())
    }

    async fn close_current_act(&self, input: &TurnInput) -> Result<(), MessageManagerError> {
        let Some(act) = input.active_act.as_ref() else {
            log::error!("No active act found for adventure {}", input.adventure.id);
            return Ok(());
        };

        let data = self.load_context(&input.adventure).await?;
        let prompt = ContextBuilder::build_act_transition_prompt(&data, &input.uncovered_messages);

        let narration = self.narrator_handler.narrator_respond(&prompt).await?;
        let narration_message = input
            .adventure
            .message_from_reply(Message::NARRATOR_SPEAKER_NAME, &narration);
        let stored = self.create_message(narration_message).await?;

        let mut all_messages = input.uncovered_messages.clone();
        all_messages.push(stored);

        self.memory_manager
            .close_current_act(act, &all_messages)
            .await?;

        Ok(())
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
        return Ok(vec![]);
    };

    let uncovered = fetch_since(conn, act.adventure_id, act.digest_to_seq).await?;

    if act.ordinal != 0 && uncovered.len() < window as usize {
        return Ok(fetch_last_n(conn, act.adventure_id, window).await?);
    }

    Ok(uncovered)
}

fn should_close_act(score: u8, last_message: &Message, act_max_messages: u16) -> bool {
    let reached_good_score = score >= 5;
    let reached_acceptable_score = score == 4;
    let eligible_to_close = last_message
        .seq
        .is_some_and(|seq| u32::from(seq) * 4 / 5 >= u32::from(act_max_messages));

    reached_good_score || (eligible_to_close && reached_acceptable_score)
}

#[cfg(test)]
mod load_uncovered_tests {
    use super::*;
    use crate::backend::persistence::db::open_pool;
    use crate::backend::persistence::message_persistence::insert_message;
    use crate::backend::persistence::migrator::run_migrations;
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

    async fn seed_messages(pool: &SqlitePool, adventure_id: Uuid, count: usize) {
        let mut conn = pool.acquire().await.expect("acquire");
        for i in 0..count {
            let message = Message::new(
                adventure_id,
                None,
                None,
                Message::NARRATOR_SPEAKER_NAME.to_string(),
                format!("message {i}"),
                false,
            );
            insert_message(&mut conn, &message)
                .await
                .expect("insert message");
        }
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
        seed_messages(&pool, adventure_id, 10).await;
        let mut conn = pool.acquire().await.expect("acquire");

        // first act (ordinal 0) with a watermark past every message: nothing uncovered
        let mut act = Act::new(adventure_id, 0, "goal");
        act.digest_to_seq = 10;

        let messages = load_uncovered_messages(&mut conn, &Some(act), 50)
            .await
            .expect("load uncovered");

        // the first act never borrows from earlier history
        assert!(messages.is_empty());
    }

    #[tokio::test]
    async fn fresh_later_act_backfills_the_recent_window() {
        let (pool, _db, adventure_id) = setup().await;
        seed_messages(&pool, adventure_id, 10).await;
        let mut conn = pool.acquire().await.expect("acquire");

        // a later act (ordinal 1) just started: its watermark is at its own first
        // message, so it has no uncovered tail
        let mut act = Act::new(adventure_id, 1, "goal");
        act.digest_to_seq = 10;

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
        seed_messages(&pool, adventure_id, 10).await;
        let mut conn = pool.acquire().await.expect("acquire");

        // a later act whose own uncovered tail already meets the window
        let mut act = Act::new(adventure_id, 1, "goal");
        act.digest_to_seq = 4;

        let messages = load_uncovered_messages(&mut conn, &Some(act), 3)
            .await
            .expect("load uncovered");

        // tail is seqs 5..=10 (six messages), above the window of 3: no backfill
        assert_eq!(messages.len(), 6);
        assert_eq!(messages.first().expect("a message").text, "message 4");
    }
}

#[cfg(test)]
mod should_close_tests {
    use super::*;
    use uuid::Uuid;

    fn beat_at(seq: Option<u16>) -> Message {
        let mut message = Message::new(
            Uuid::new_v4(),
            None,
            None,
            Message::NARRATOR_SPEAKER_NAME.to_string(),
            "a beat",
            false,
        );
        message.seq = seq;
        message
    }

    #[test]
    fn a_score_of_five_closes_wherever_it_lands() {
        // a 5 resolves the act on its own, so position never enters into it
        assert!(should_close_act(5, &beat_at(None), 200));
        assert!(should_close_act(5, &beat_at(Some(1)), 200));
        assert!(should_close_act(5, &beat_at(Some(200)), 200));
    }

    #[test]
    fn a_score_of_four_needs_the_act_to_run_past_its_cap() {
        // a 4 is patient: the cap itself is not enough, it keeps the story flowing
        assert!(!should_close_act(4, &beat_at(Some(100)), 100));

        // the gate opens at 125% of the cap, where seq / 5 * 4 first reaches it
        assert!(!should_close_act(4, &beat_at(Some(124)), 100));
        assert!(should_close_act(4, &beat_at(Some(125)), 100));

        // without a sequence there is no position to judge
        assert!(!should_close_act(4, &beat_at(None), 100));

        // anything under 4 never closes, however long the act runs
        assert!(!should_close_act(3, &beat_at(Some(5_000)), 100));
        assert!(!should_close_act(0, &beat_at(Some(5_000)), 100));
    }
}
