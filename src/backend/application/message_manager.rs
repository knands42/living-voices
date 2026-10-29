use crate::backend::PersistenceError;
use crate::backend::application::context::ContextBuilder;
use crate::backend::application::director::Director;
use crate::backend::application::memory_manager::{MemoryManager, MemoryManagerError};
use crate::backend::application::port::chat_port::{
    ChatHandler, ChatReply, ChatResponseError, NarratorHandler,
};
use crate::backend::application::port::memory_port::{ActCloser, MemoryDistiller};
use crate::backend::application::state::{SharedState, StateError};
use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::character::Character;
use crate::backend::domain::message::Message;
use crate::backend::persistence::act_persistence::{
    get_active_act, get_acts, update_active_act_last_closure_score,
};
use crate::backend::persistence::message_persistence::{
    count_since_last_narration, fetch_since, insert_message,
};
use crate::backend::persistence::state_persistence::get_active_candidates;
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
    memory: MemoryManager<MEMORY, ACT>,
    memory_settings: MemorySettings,
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
        memory: MemoryManager<MEMORY, ACT>,
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

        self.check_and_update_current_act(chat_reply, &adventure)
            .await?;
        self.check_and_compress_memory().await;

        // TODO: check if make sense to call for another llm message (multiple characters talk per turn)

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

    async fn check_and_compress_memory(&self) {
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

    async fn check_and_update_current_act(
        &self,
        chat_reply: ChatReply,
        adventure: &Adventure,
    ) -> Result<(), MessageManagerError> {
        if let Some(score) = chat_reply.act_closure {
            let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
            update_active_act_last_closure_score(&mut conn, adventure.id, score as u16).await?;

            if score >= 5 {
                let current_act = get_active_act(&mut conn, adventure.id).await?;
                if let Some(act) = current_act {
                    let uncovered_messages =
                        fetch_since(&mut conn, adventure.id, act.digest_to_seq).await?;
                    ContextBuilder::build_act_transition_prompt(
                        &mut conn,
                        adventure,
                        &uncovered_messages,
                    )
                    .await?;

                    self.memory
                        .close_current_act(&act, &uncovered_messages)
                        .await?;
                } else {
                    log::error!("No active act found for adventure {}", adventure.id);
                }
            }

            drop(conn);
        }

        Ok(())
    }
}
