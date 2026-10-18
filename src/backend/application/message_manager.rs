use crate::backend::PersistenceError;
use crate::backend::application::director::Director;
use crate::backend::application::history::HistoryBuilder;
use crate::backend::application::port::chat_port::{Candidate, ChatHandler, ChatResponseError};
use crate::backend::application::state::{SharedState, StateError};
use crate::backend::domain::message::Message;
use crate::backend::persistence::message_persistence::insert_message;
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
pub struct MessageManager<D: ChatHandler> {
    state: SharedState,
    pool: SqlitePool,
    chat_handler: D,
    director: Director,
}

impl<D: ChatHandler> MessageManager<D> {
    pub fn new(state: SharedState, pool: SqlitePool, chat_handler: D, director: Director) -> Self {
        Self {
            state,
            pool,
            chat_handler,
            director,
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
        let history = HistoryBuilder::build(&mut conn, adventure.id).await?;
        drop(conn);

        let user_message = incoming.parts.render(incoming.speaker_name.as_str());

        let reply = self
            .chat_handler
            .autonomous_respond(
                adventure.tone.as_str(),
                &candidates,
                &history,
                &user_message,
            )
            .await?;

        let reply_message = adventure.message_from_reply(&reply.replier, &reply.reply);
        let stored_reply = self.create_message(reply_message).await?;

        // TODO: trigger compression?

        Ok(stored_reply)
    }

    async fn send_and_expect_narrator_to_reply() -> Result<Message, MessageManagerError> {
        todo!()
    }
    async fn send_and_expect_character_to_reply() -> Result<Message, MessageManagerError> {
        todo!()
    }
}
