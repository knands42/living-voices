use sqlx::SqlitePool;
use thiserror::Error;

use crate::backend::PersistenceError;
use crate::backend::application::director::Director;
use crate::backend::application::memory_manager::{MemoryManager, MemoryManagerError};
use crate::backend::application::port::chat_port::{ChatPort, ChatResponseError, NarratorPort};
use crate::backend::application::port::memory_port::{ActCloserPort, MemoryDistillerPort};
use crate::backend::application::state::{SharedState, StateError};
use crate::backend::domain::message::Message;
use crate::backend::persistence::message_persistence::insert_messages;

#[derive(Error, Debug)]
pub enum MessageManagerError {
    #[error(transparent)]
    State(#[from] StateError),
    #[error(transparent)]
    DatabaseError(#[from] PersistenceError),
    #[error(transparent)]
    Chat(#[from] ChatResponseError),
    #[error(transparent)]
    Memory(#[from] MemoryManagerError),
    #[error("the model returned an empty reply")]
    EmptyReply,
    #[error("no adventure is currently loaded")]
    NoAdventure,
}

#[derive(Clone, Debug)]
pub struct MessageManager<
    CHAT: ChatPort,
    NARRATOR: NarratorPort,
    MEMORY: MemoryDistillerPort,
    ACT: ActCloserPort,
> {
    pub(super) state: SharedState,
    pub(super) pool: SqlitePool,
    pub(super) chat_handler: CHAT,
    pub(super) narrator_handler: NARRATOR,
    pub(super) director: Director,
    pub(super) memory_manager: MemoryManager<MEMORY, ACT>,
}

impl<
    CHAT: ChatPort,
    NARRATOR: NarratorPort,
    MEMORY: MemoryDistillerPort + Clone + 'static,
    ACT: ActCloserPort + Clone + 'static,
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
        log::info!(
            "turn start: adventure={} character={:?} narrator={:?}",
            incoming.adventure_id,
            incoming.character_id,
            incoming.narrator_id
        );
        self.create_message(incoming.clone()).await?;

        let mut input_data = self.gather_data().await?;
        let chat_reply = self.respond(&input_data).await?;
        let (stored_reply, act_closed) = self.process_respond(chat_reply, &mut input_data).await?;
        if !act_closed {
            self.run_follow_ups(stored_reply.clone()).await?;
        }

        Ok(stored_reply)
    }

    pub async fn request_new_message(&self) -> Result<Message, MessageManagerError> {
        log::info!("requesting a new message");

        let mut input_data = self.gather_data().await?;
        let chat_reply = self.respond(&input_data).await?;
        let (stored_reply, act_closed) = self.process_respond(chat_reply, &mut input_data).await?;
        if !act_closed {
            self.run_follow_ups(stored_reply.clone()).await?;
        }

        Ok(stored_reply)
    }

    pub(super) async fn create_message(
        &self,
        message: Message,
    ) -> Result<Message, MessageManagerError> {
        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        let mut stored = insert_messages(&mut conn, std::slice::from_ref(&message)).await?;

        Ok(stored.remove(0))
    }
}
