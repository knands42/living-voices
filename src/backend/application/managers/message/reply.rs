use crate::backend::application::context::{ContextBuilder, ContextData};
use crate::backend::application::port::chat_port::{ChatPort, ChatReply, NarratorPort};
use crate::backend::application::port::memory_port::{ActCloserPort, MemoryDistillerPort};
use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::character::Character;
use crate::backend::domain::message::Message;

use super::context::MessageContext;
use super::message_manager::{MessageManager, MessageManagerError};

const REPLY_ATTEMPTS: u8 = 2;

impl<
    CHAT: ChatPort,
    NARRATOR: NarratorPort,
    MEMORY: MemoryDistillerPort + Clone + 'static,
    ACT: ActCloserPort + Clone + 'static,
> MessageManager<CHAT, NARRATOR, MEMORY, ACT>
{
    pub(super) async fn respond(
        &self,
        incoming: &Message,
        input: &MessageContext,
    ) -> Result<ChatReply, MessageManagerError> {
        let active_characters = input.adventure.active_characters(&input.present_names);
        let data = self.load_context(&input.adventure).await?;

        for attempt in 1..=REPLY_ATTEMPTS {
            let reply = self
                .send_to_handler(
                    &input.adventure,
                    incoming,
                    &active_characters,
                    input.messages_since_narration,
                    &input.uncovered_messages,
                    &data,
                )
                .await?;
            if input
                .adventure
                .message_from_reply(&reply.replier, &reply.reply)
                .is_some()
            {
                return Ok(reply);
            }
            log::warn!(
                "empty reply from {} (attempt {attempt}/{REPLY_ATTEMPTS})",
                reply.replier
            );
        }

        Err(MessageManagerError::EmptyReply)
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
            let mut reply = self
                .narrator_handler
                .narrator_chat_respond(&context)
                .await?;
            reply.replier = Message::NARRATOR_SPEAKER_NAME.to_string();
            log::info!("narrator will reply");
            return Ok(reply);
        }

        if let Some(character) = self.director.pick_character(active_characters) {
            let context = ContextBuilder::build_single_character_chat_prompt(
                adventure,
                data,
                character,
                uncovered_messages,
            );
            let mut reply = self.chat_handler.character_respond(&context).await?;
            if reply.replier.trim().is_empty() {
                log::warn!(
                    "character reply had an empty speaker, defaulting to {}",
                    character.name
                );
                reply.replier = character.name.clone();
            }
            log::info!("{} will reply", reply.replier);
            return Ok(reply);
        }

        let context = ContextBuilder::build_autonomous_chat_prompt(
            adventure,
            &self.director,
            data,
            uncovered_messages,
        );

        log::info!("autonomous will reply");
        let reply = self.chat_handler.autonomous_respond(&context).await?;
        Ok(reply)
    }

    pub(super) async fn process_respond(
        &self,
        chat_reply: ChatReply,
        message_context: &mut MessageContext,
    ) -> Result<(Message, bool), MessageManagerError> {
        let reply_message = message_context
            .adventure
            .message_from_reply(&chat_reply.replier, &chat_reply.reply)
            .ok_or(MessageManagerError::EmptyReply)?;
        let stored_reply = self.create_message(reply_message).await?;

        message_context
            .uncovered_messages
            .push(stored_reply.clone());

        let act_closed = self
            .check_and_update_current_act(chat_reply, message_context)
            .await?;

        Ok((stored_reply, act_closed))
    }
}
