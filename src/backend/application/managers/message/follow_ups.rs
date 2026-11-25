use crate::backend::PersistenceError;
use crate::backend::application::context::{ContextBuilder, ContextData};
use crate::backend::application::port::chat_port::{ChatPort, NarratorPort};
use crate::backend::application::port::memory_port::{ActCloserPort, MemoryDistillerPort};
use crate::backend::domain::message::Message;

use super::message_manager::{MessageManager, MessageManagerError};

impl<
    CHAT: ChatPort,
    NARRATOR: NarratorPort,
    MEMORY: MemoryDistillerPort + Clone + 'static,
    ACT: ActCloserPort + Clone + 'static,
> MessageManager<CHAT, NARRATOR, MEMORY, ACT>
{
    pub(super) async fn run_follow_ups(
        &self,
        mut last_message: Message,
    ) -> Result<(), MessageManagerError> {
        let mut steps_taken = 0;

        loop {
            let input = self.gather_data().await?;
            let candidates = input.adventure.active_characters(&input.present_names);

            if !self.director.should_a_conversation_follow_up(
                &candidates,
                steps_taken,
                &last_message,
            ) {
                return Ok(());
            }

            let data = {
                let mut conn = self.pool.acquire().await.map_err(PersistenceError::from)?;
                ContextData::load(&mut conn, input.adventure.id).await?
            };
            let prompt = ContextBuilder::build_follow_up_prompt(
                &input.adventure,
                &data,
                &input.uncovered_messages,
            );
            let reply = self.chat_handler.follow_up_respond(&prompt).await?;

            if reply.replier == Message::NARRATOR_SPEAKER_NAME {
                log::debug!("message follow-up skipped for speaker {}", reply.replier);
                return Ok(());
            }

            let Some(message) = input
                .adventure
                .message_from_reply(&reply.replier, &reply.reply)
            else {
                log::debug!(
                    "follow-up from {} had no body; stopping the follow-ups",
                    reply.replier
                );
                return Ok(());
            };
            let stored = self.create_message(message).await?;
            log::info!("follow-up from {}", input.adventure.speaker_name(&stored));

            last_message = stored;
            steps_taken += 1;
        }
    }
}

#[cfg(test)]
mod follow_up_tests {
    use super::*;
    use crate::backend::application::director::Director;
    use crate::backend::domain::character::Character;
    use crate::config::DirectorSettings;
    use uuid::Uuid;

    fn director(follow_up_chance: f32, follow_up_max_chain: u8) -> Director {
        Director::new(DirectorSettings {
            follow_up_chance,
            follow_up_max_chain,
            ..DirectorSettings::default()
        })
    }

    fn beat(adventure_id: Uuid, narrator: bool) -> Message {
        Message::new(
            adventure_id,
            None,
            narrator.then(Uuid::new_v4),
            "a beat".to_string(),
            false,
        )
    }

    #[test]
    fn a_lone_candidate_never_sparks_a_follow_up() {
        // given a director that would otherwise always chain
        let director = director(1.0, 6);
        let adventure_id = Uuid::new_v4();
        let solo = Character::draft(adventure_id);
        let candidates = [&solo];

        // when the only present character just spoke
        let follow_up =
            director.should_a_conversation_follow_up(&candidates, 0, &beat(adventure_id, false));

        // then a single voice cannot answer itself
        assert!(!follow_up);
    }

    #[test]
    fn the_chain_cap_ends_follow_ups_even_after_narration() {
        // given a director capped at two steps
        let director = director(1.0, 2);
        let adventure_id = Uuid::new_v4();
        let a = Character::draft(adventure_id);
        let b = Character::draft(adventure_id);
        let candidates = [&a, &b];

        // when the chain already reached its cap on a narrator beat
        let follow_up =
            director.should_a_conversation_follow_up(&candidates, 2, &beat(adventure_id, true));

        // then narration does not carry the chain past the cap
        assert!(!follow_up);
    }

    #[test]
    fn a_certain_chance_chains_right_after_a_character_beat() {
        // given a director whose base chance is the maximum
        let director = director(1.0, 6);
        let adventure_id = Uuid::new_v4();
        let a = Character::draft(adventure_id);
        let b = Character::draft(adventure_id);
        let candidates = [&a, &b];

        // when two characters are present at the start of a chain
        // chance = 1.0 / (0 + 1) = 1.0, and rand is always in [0, 1)
        let follow_up =
            director.should_a_conversation_follow_up(&candidates, 0, &beat(adventure_id, false));

        // then the follow-up fires, exercising the chance scaling
        assert!(follow_up);
    }
}
