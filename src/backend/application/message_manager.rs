use crate::backend::PersistenceError;
use crate::backend::application::director::Director;
use crate::backend::application::port::chat_port::{Candidate, ChatHandler, ChatResponseError};
use crate::backend::application::state::{SharedState, StateError};
use crate::backend::domain::memory::{Fact, Summary};
use crate::backend::domain::message::{Message, MessageKind, MessagePart, MessageParts};
use crate::backend::persistence::memory_persistence::{
    last_covered, latest_facts, latest_summaries,
};
use crate::backend::persistence::message_persistence::{fetch_last_n, fetch_since, insert_message};
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
pub struct ChatView {
    /// The newest ~100 raw messages, oldest-first.
    pub messages: Vec<Message>,
    /// The assistant line just produced, for a surface that only wants the tail.
    pub reply: Message,
}

impl ChatView {
    pub const CHAT_VIEW_LIMIT: i64 = 100;
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

    /// Persist a single message.
    async fn create_message(&self, message: Message) -> Result<Message, MessageManagerError> {
        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        let stored = insert_message(&mut conn, &message).await?;
        Ok(stored)
    }

    /// Run one chat turn end to end
    pub async fn send_message(&self, incoming: Message) -> Result<ChatView, MessageManagerError> {
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

        let summaries = latest_summaries(&mut conn, adventure.id, 5).await?;
        let facts = latest_facts(&mut conn, adventure.id, 5).await?;

        let after_seq = last_covered(&mut conn, adventure.id).await.unwrap_or(0);
        let uncovered = fetch_since(&mut conn, adventure.id, after_seq).await?;
        drop(conn);

        // TODO: create a history handler

        let mut history = String::new();
        history.push_str(&Summary::render(&summaries).unwrap_or_default());
        history.push_str(&Fact::render(&facts).unwrap_or_default());
        history.push_str(&Message::render(&uncovered).unwrap_or_default());

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

        let (character_id, narrator_id, speaker_name, kind) =
            match adventure.find_character(&reply.replier) {
                Some(character) => (
                    Some(character.id),
                    None,
                    character.name.clone(),
                    MessageKind::Say,
                ),
                None => (
                    None,
                    Some(adventure.narrator.id),
                    Message::NARRATOR_SPEAKER_NAME.to_string(),
                    MessageKind::Narration,
                ),
            };

        let reply_message = Message::new(
            adventure.id,
            character_id,
            narrator_id,
            speaker_name,
            MessageParts::one(MessagePart::new(kind, &reply.reply)),
        );
        let stored_reply = self.create_message(reply_message).await?;

        // TODO: trigger compression?

        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        let messages = fetch_last_n(&mut conn, adventure.id, ChatView::CHAT_VIEW_LIMIT).await?;

        Ok(ChatView {
            messages,
            reply: stored_reply,
        })
    }

    async fn send_and_expect_narrator_to_reply() -> Result<ChatView, MessageManagerError> {
        todo!()
    }
    async fn send_and_expect_character_to_reply() -> Result<ChatView, MessageManagerError> {
        todo!()
    }
    async fn send() -> Result<ChatView, MessageManagerError> {
        todo!()
    }
}
