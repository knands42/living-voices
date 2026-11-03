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
use crate::backend::domain::memory::Act;
use crate::backend::domain::message::Message;
use crate::backend::persistence::act_persistence::{
    get_active_act, update_active_act_last_closure_score,
};
use crate::backend::persistence::message_persistence::{
    count_since_last_narration, fetch_since, insert_message,
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
    messages_since_narration: u8,
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

        let input_data = self.gather_data(&incoming).await?;
        let chat_reply = self.respond(&incoming, &input_data).await?;
        self.settle(chat_reply, input_data).await
    }

    async fn gather_data(&self, incoming: &Message) -> Result<TurnInput, MessageManagerError> {
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

        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        self.send_to_handler(
            &mut conn,
            &input.adventure,
            incoming,
            &active_characters,
            input.messages_since_narration,
            &input.uncovered_messages,
        )
        .await
    }

    async fn settle(
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
        conn: &mut SqliteConnection,
        adventure: &Adventure,
        incoming: &Message,
        active_characters: &[&Character],
        messages_since_narration: u8,
        uncovered_messages: &[Message],
    ) -> Result<ChatReply, MessageManagerError> {
        if self.director.should_narrator_speak(
            active_characters,
            incoming,
            messages_since_narration,
        ) {
            let context = ContextBuilder::build_narrator_chat_prompt(
                conn,
                adventure,
                &self.director,
                uncovered_messages,
            )
            .await?;
            let reply = self.narrator_handler.narrator_respond(&context).await?;
            return Ok(ChatReply {
                replier: Message::NARRATOR_SPEAKER_NAME.to_string(),
                reply,
                act_closure: None,
            });
        }

        if let Some(character) = self.director.pick_character(active_characters) {
            let context = ContextBuilder::build_single_character_chat_prompt(
                conn,
                adventure,
                &self.director,
                character,
                uncovered_messages,
            )
            .await?;
            let reply = self.chat_handler.character_respond(&context).await?;
            return Ok(ChatReply {
                replier: character.name.clone(),
                reply,
                act_closure: None,
            });
        }

        let context = ContextBuilder::build_autonomous_chat_prompt(
            conn,
            adventure,
            &self.director,
            uncovered_messages,
        )
        .await?;
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

        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;

        let should_close = score >= 5
            || matches!(last_message.seq, Some(seq) if seq >= self.director.get_max_act_messages() && score == 4);

        let updated_act =
            update_active_act_last_closure_score(&mut conn, input.adventure.id, score).await?;

        if should_close {
            self.close_current_act(&mut conn, input).await?;
        } else {
            self.state.write()?.current_act = updated_act;

            if let Some(act) = input.active_act.as_ref() {
                self.check_and_compress_memory(act, &input.uncovered_messages);
            }
        }

        Ok(())
    }

    async fn close_current_act(
        &self,
        conn: &mut SqliteConnection,
        input: &TurnInput,
    ) -> Result<(), MessageManagerError> {
        let Some(act) = input.active_act.as_ref() else {
            log::error!("No active act found for adventure {}", input.adventure.id);
            return Ok(());
        };

        let prompt = ContextBuilder::build_act_transition_prompt(
            conn,
            &input.adventure,
            &input.uncovered_messages,
        )
        .await?;

        let narration = self.narrator_handler.narrator_respond(&prompt).await?;
        let narration_message = input
            .adventure
            .message_from_reply(Message::NARRATOR_SPEAKER_NAME, &narration);
        let stored = insert_message(conn, &narration_message).await?;

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
        let Some(act) = active_act else {
            return Ok(vec![]);
        };

        let uncovered = fetch_since(conn, act.adventure_id, act.digest_to_seq).await?;
        Ok(uncovered)
    }
}
