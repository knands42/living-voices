use crate::backend::PersistenceError;
use crate::backend::application::context::ContextBuilder;
use crate::backend::application::port::chat_port::{ChatPort, ChatReply, NarratorPort};
use crate::backend::application::port::memory_port::{ActCloserPort, MemoryDistillerPort};
use crate::backend::domain::memory::Act;
use crate::backend::domain::message::Message;
use crate::backend::persistence::act_persistence::update_active_act_last_closure_score;

use super::context::MessageContext;
use super::message_manager::{MessageManager, MessageManagerError};

impl<
    CHAT: ChatPort,
    NARRATOR: NarratorPort,
    MEMORY: MemoryDistillerPort + Clone + 'static,
    ACT: ActCloserPort + Clone + 'static,
> MessageManager<CHAT, NARRATOR, MEMORY, ACT>
{
    pub(super) async fn check_and_update_current_act(
        &self,
        chat_reply: ChatReply,
        message_context: &mut MessageContext,
    ) -> Result<bool, MessageManagerError> {
        let score = chat_reply.act_closure.unwrap_or(0);
        if chat_reply.act_closure.is_none() && chat_reply.replier != Message::NARRATOR_SPEAKER_NAME
        {
            log::debug!(
                "reply from {} carried no act closure score; treating it as 0",
                chat_reply.replier
            );
        }
        log::debug!(
            "act closure score {} for adventure {}",
            score,
            message_context.adventure.id
        );

        let updated_act = {
            let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
            update_active_act_last_closure_score(&mut conn, message_context.adventure.id, score)
                .await?
        };

        let Some(last_message) = message_context.uncovered_messages.last() else {
            return Ok(false);
        };
        if should_close_act(score, last_message, self.director.get_max_act_messages()) {
            self.close_current_act(message_context).await?;
            return Ok(true);
        }

        if let Some(act) = message_context.active_act.as_ref() {
            self.check_and_compress_memory(act, &message_context.uncovered_messages);
        }

        let mut guard = self.state.write()?;
        guard.current_act = updated_act;

        Ok(false)
    }

    fn check_and_compress_memory(&self, act: &Act, uncovered_messages: &[Message]) {
        if !self
            .memory_manager
            .needs_compression(uncovered_messages.len())
        {
            return;
        }

        log::debug!(
            "compressing memory: {} uncovered messages for act {}",
            uncovered_messages.len(),
            act.id
        );
        let memory = self.memory_manager.clone();
        let act = act.clone();
        let uncovered = uncovered_messages.to_vec();
        tokio::spawn(async move {
            if let Err(err) = memory.compress_and_update(&act, &uncovered).await {
                log::error!("memory compression failed: {err}");
            }
        });
    }

    async fn close_current_act(
        &self,
        input: &mut MessageContext,
    ) -> Result<(), MessageManagerError> {
        let Some(act) = input.active_act.as_ref() else {
            log::error!("No active act found for adventure {}", input.adventure.id);
            return Ok(());
        };
        log::info!(
            "closing act {} for adventure {}",
            act.id,
            input.adventure.id
        );

        if let Some(last_message) = input.uncovered_messages.last() {
            if last_message.narrator_id.is_none() {
                let data = self.load_context(&input.adventure).await?;
                let prompt = ContextBuilder::build_act_transition_prompt(
                    &input.adventure,
                    &data,
                    &input.uncovered_messages,
                );

                let narration = self.narrator_handler.narrator_respond(&prompt).await?;
                match input
                    .adventure
                    .message_from_reply(Message::NARRATOR_SPEAKER_NAME, &narration)
                {
                    Some(narration_message) => {
                        let stored = self.create_message(narration_message).await?;
                        input.uncovered_messages.push(stored);
                    }
                    None => log::warn!(
                        "the act transition narration had no body; closing act without one"
                    ),
                }
            }
        }

        self.memory_manager
            .close_current_act(act, &input.uncovered_messages)
            .await?;
        log::info!("act {} closed", act.id);

        Ok(())
    }
}

fn should_close_act(score: u8, last_message: &Message, act_max_messages: u16) -> bool {
    let reached_good_score = score >= 5;
    let reached_acceptable_score = score == 4;
    let eligible_to_close = last_message
        .seq
        .is_some_and(|seq| u32::from(seq) * 4 / 5 >= u32::from(act_max_messages));

    if !reached_good_score && reached_acceptable_score && last_message.seq.is_none() {
        log::debug!("act not closed at score 4: last message has no sequence to judge position");
    }

    reached_good_score || (eligible_to_close && reached_acceptable_score)
}

#[cfg(test)]
mod should_close_tests {
    use super::*;
    use uuid::Uuid;

    fn beat_at(seq: Option<u16>) -> Message {
        let mut message = Message::new(Uuid::new_v4(), None, None, "a beat", false);
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
