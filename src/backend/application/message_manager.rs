use crate::backend::PersistenceError;
use crate::backend::application::port::chat_port::{ChatHandler, ChatResponseError};
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
}

impl<D: ChatHandler> MessageManager<D> {
    pub fn new(state: SharedState, pool: SqlitePool, chat_handler: D) -> Self {
        Self {
            state,
            pool,
            chat_handler,
        }
    }

    /// Persist a single message
    async fn create_message(&self, message: Message) -> Result<Message, MessageManagerError> {
        let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
        let stored = insert_message(&mut conn, &message).await?;
        Ok(stored)
    }

    /// Run one chat turn end to end:
    pub async fn send_message(&self, incoming: Message) -> Result<ChatView, MessageManagerError> {
        // Current adventure — the whole turn is scoped to it.
        let adventure_id = {
            let guard = self.state.read()?;
            guard
                .current_adventure
                .as_ref()
                .ok_or(MessageManagerError::NoAdventure)?
                .id
        };

        // 2. Persist the incoming message.
        let stored_incoming = self.create_message(incoming).await?;

        // 3. Build the context the LLM should see.
        //    This is the step that answers your question: send the summary +
        //    facts for everything up to `last_covered`, then ONLY the raw
        //    messages with seq > last_covered. Never the covered raw messages —
        //    they are already represented by the summary.
        //
        //    TODO (your core logic): assemble (system, history) where
        //      - system = persona/story prompt + rendered summary + facts
        //      - history = uncovered messages (fetch_since(last_covered))
        //                  mapped to ChatMessage via `From<&Message>`
        //    NOTE: two persistence helpers do not exist yet and are yours to add
        //    (shape is a schema decision):
        //      - a loader for the latest summary + its facts for an adventure
        //      - a `fetch_last_n(adventure_id, CHAT_VIEW_LIMIT)` for step 7
        let history: Vec<crate::ai::ChatMessage> =
            todo!("fetch uncovered messages, map to Vec<ChatMessage>");
        let user_message: &str = todo!("render the incoming message as the user line");
        // TODO: source these from the adventure/character (voice settings).
        let narrator_tone: &str = todo!("narrator tone for this adventure");
        let _character_tone: &str = todo!("character tone for the speaking NPC");

        // 4. Ask the handler. The speaker is already chosen (later: by the
        //    Director). For now this defaults to the narrator; when the Director
        //    lands, branch here between `narrator_respond` and
        //    `character_respond(character_tone, ...)`.
        //    Returns the plain line text — the caller already knows who speaks.
        let reply_text = self
            .chat_handler
            .narrator_respond(narrator_tone, &history, user_message)
            .await?;

        // 5. Persist the reply as a Message.
        //    TODO: construct the assistant Message from `reply_text` and the
        //    known speaker, then persist with create_message.
        let _ = &reply_text;
        let reply: Message = todo!("build assistant Message from reply_text + known speaker");
        let stored_reply = self.create_message(reply).await?;

        // 6. Nudge the distiller. The timer already runs the real schedule, so
        //    this is optional. TODO: decide whether a turn also triggers a
        //    check, or you leave it entirely to DistillScheduler.

        // 7. Return the UI view: the last ~100 RAW messages + the fresh reply.
        //    TODO: let messages = fetch_last_n(adventure_id, CHAT_VIEW_LIMIT);
        let messages: Vec<Message> = todo!("fetch last CHAT_VIEW_LIMIT raw messages");
        Ok(ChatView {
            messages,
            reply: stored_reply,
        })
    }
}
