use crate::backend::PersistenceError;
use crate::backend::application::director::Director;
use crate::backend::application::history::HistoryBuilder;
use crate::backend::application::memory_manager::MemoryManager;
use crate::backend::application::port::chat_port::{
    Candidate, ChatHandler, ChatResponseError, NarratorHandler,
};
use crate::backend::application::port::memory_port::MemoryDistiller;
use crate::backend::application::state::{SharedState, StateError};
use crate::backend::domain::message::Message;
use crate::backend::persistence::message_persistence::{
    count_since_last_narration, insert_message,
};
use crate::config::MemorySettings;
use sqlx::SqlitePool;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MessageManagerError {
    #[error("state error: {0}")]
    State(#[from] StateError),
    #[error("database error: {0}")]
    DatabaseError(#[from] PersistenceError),
    #[error("chat provider failed: {0}")]
    Chat(#[from] ChatResponseError),
    #[error("no adventure is currently loaded")]
    NoAdventure,
    #[error("no character is available to respond")]
    NoResponder,
}

#[derive(Clone, Debug)]
pub struct MessageManager<D: ChatHandler, N: NarratorHandler, M: MemoryDistiller> {
    state: SharedState,
    pool: SqlitePool,
    chat_handler: D,
    narrator_handler: N,
    director: Director,
    memory: MemoryManager<M>,
    memory_settings: MemorySettings,
}

impl<D: ChatHandler, N: NarratorHandler, M: MemoryDistiller + Clone + 'static>
    MessageManager<D, N, M>
{
    pub fn new(
        state: SharedState,
        pool: SqlitePool,
        chat_handler: D,
        narrator_handler: N,
        director: Director,
        memory: MemoryManager<M>,
        memory_settings: MemorySettings,
    ) -> Self {
        Self {
            state,
            pool,
            chat_handler,
            narrator_handler,
            director,
            memory,
            memory_settings,
        }
    }

    async fn create_message(&self, message: Message) -> Result<Message, MessageManagerError> {
        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        let stored = insert_message(&mut conn, &message).await?;
        Ok(stored)
    }

    pub async fn send_message(&self, incoming: Message) -> Result<Message, MessageManagerError> {
        let adventure = {
            let guard = self.state.read()?;
            guard
                .current_adventure
                .as_ref()
                .ok_or(MessageManagerError::NoAdventure)?
                .clone()
        };

        self.create_message(incoming.clone()).await?;

        let candidates = Candidate::from_adventure(&adventure);
        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        let history = HistoryBuilder::build(
            &mut conn,
            adventure.id,
            self.memory_settings.summary_limit,
            self.memory_settings.fact_limit,
        )
        .await?;
        let messages_since_narration = count_since_last_narration(&mut conn, adventure.id).await?;
        drop(conn);

        let user_message = incoming.text.as_str();

        let reply =
            if self
                .director
                .should_narrator_speak(&candidates, &incoming, messages_since_narration)
            {
                self.narrator_handler
                    .narrator_respond(adventure.tone.as_str(), &history, &user_message)
                    .await?
            } else if let Some(candidate) = self.director.pick_character(&candidates, &incoming) {
                self.chat_handler
                    .character_respond(candidate, &history, &user_message)
                    .await?
            } else {
                self.chat_handler
                    .autonomous_respond(&candidates, &history, &user_message)
                    .await?
            };

        let reply_message = adventure.message_from_reply(&reply.replier, &reply.reply);
        let stored_reply = self.create_message(reply_message).await?;

        self.maybe_compress_memory().await;

        Ok(stored_reply)
    }

    async fn maybe_compress_memory(&self) {
        match self
            .memory
            .needs_compression(self.memory_settings.compress_threshold)
            .await
        {
            Ok(false) => {}
            Ok(true) => {
                let memory = self.memory.clone();
                tokio::spawn(async move {
                    if let Err(err) = memory.compress().await {
                        eprintln!("memory compression failed: {err}");
                    }
                });
            }
            Err(err) => eprintln!("memory compression check failed: {err}"),
        }
    }
}
