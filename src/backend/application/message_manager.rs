use crate::backend::PersistenceError;
use crate::backend::application::context::ContextBuilder;
use crate::backend::application::director::Director;
use crate::backend::application::memory_manager::{MemoryManager, MemoryManagerError};
use crate::backend::application::port::chat_port::{
    ChatHandler, ChatReply, ChatResponseError, NarratorHandler,
};
use crate::backend::application::port::memory_port::{ActCloser, MemoryDistiller};
use crate::backend::application::state::{SharedState, StateError};
use crate::backend::domain::character::Character;
use crate::backend::domain::message::Message;
use crate::backend::persistence::memory_persistence::{
    get_active_candidates, update_active_act_last_closure_score,
};
use crate::backend::persistence::message_persistence::{
    count_since_last_narration, insert_message,
};
use crate::config::MemorySettings;
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
}

#[derive(Clone, Debug)]
pub struct MessageManager<D: ChatHandler, N: NarratorHandler, M: MemoryDistiller, C: ActCloser> {
    state: SharedState,
    pool: SqlitePool,
    chat_handler: D,
    narrator_handler: N,
    director: Director,
    memory: MemoryManager<M, C>,
    memory_settings: MemorySettings,
}

impl<
    D: ChatHandler,
    N: NarratorHandler,
    M: MemoryDistiller + Clone + 'static,
    C: ActCloser + Clone + 'static,
> MessageManager<D, N, M, C>
{
    pub fn new(
        state: SharedState,
        pool: SqlitePool,
        chat_handler: D,
        narrator_handler: N,
        director: Director,
        memory: MemoryManager<M, C>,
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

        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        let present_names = get_active_candidates(&mut conn, &adventure.id.to_string()).await?;
        let active_characters = adventure.active_characters(&present_names);
        let messages_since_narration = count_since_last_narration(&mut conn, adventure.id).await?;

        let chat_reply = self
            .send(
                &mut conn,
                &adventure,
                &incoming,
                &active_characters,
                messages_since_narration as u8,
            )
            .await?;
        drop(conn);

        let reply_message = adventure.message_from_reply(&chat_reply.replier, &chat_reply.reply);
        let stored_reply = self.create_message(reply_message).await?;

        if let Some(score) = chat_reply.act_closure {
            let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
            update_active_act_last_closure_score(&mut conn, adventure.id, score as u16).await?;
            drop(conn);

            if score >= 5 {
                // TODO: narrate the act transition (new narrator prompt) after close.
                self.memory.close_current_act().await?;
                return Ok(stored_reply);
            }
        }

        self.maybe_compress_memory().await;

        // TODO: check if make sense to call for another llm message (multiple characters talk per turn)

        Ok(stored_reply)
    }

    async fn send(
        &self,
        conn: &mut SqliteConnection,
        adventure: &crate::backend::domain::adventure::Adventure,
        incoming: &Message,
        active_characters: &[&Character],
        messages_since_narration: u8,
    ) -> Result<ChatReply, MessageManagerError> {
        if self.director.should_narrator_speak(
            active_characters,
            incoming,
            messages_since_narration,
        ) {
            let context =
                ContextBuilder::build_narrator_chat_prompt(conn, adventure, &self.director).await?;
            let reply = self.narrator_handler.narrator_respond(&context).await?;
            return Ok(ChatReply {
                replier: Message::NARRATOR_SPEAKER_NAME.to_string(),
                reply,
                act_closure: None,
            });
        }

        if let Some(character) = self.director.pick_character(active_characters, incoming) {
            let context = ContextBuilder::build_single_character_chat_prompt(
                conn,
                adventure,
                &self.director,
                character,
            )
            .await?;
            let reply = self.chat_handler.character_respond(&context).await?;
            return Ok(ChatReply {
                replier: character.name.clone(),
                reply,
                act_closure: None,
            });
        }

        let context =
            ContextBuilder::build_autonomous_chat_prompt(conn, adventure, &self.director).await?;
        Ok(self.chat_handler.autonomous_respond(&context).await?)
    }

    async fn maybe_compress_memory(&self) {
        match self
            .memory
            .needs_compression(self.memory_settings.compress_after_threshold)
            .await
        {
            Ok(false) => {}
            Ok(true) => {
                let memory = self.memory.clone();
                tokio::spawn(async move {
                    if let Err(err) = memory.compress_and_update().await {
                        eprintln!("memory compression failed: {err}");
                    }
                });
            }
            Err(err) => eprintln!("memory compression check failed: {err}"),
        }
    }
}
