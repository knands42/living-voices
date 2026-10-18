use crate::backend::application::port::chat_port::Candidate;
use crate::backend::domain::message::{Message, MessageKind};
use crate::config::DirectorSettings;

/// Decides who answers a player's turn, with no model call.
#[derive(Clone, Debug)]
pub struct Director {
    director_settings: DirectorSettings,
}

impl Director {
    pub fn new(director_settings: DirectorSettings) -> Self {
        Self { director_settings }
    }

    pub fn should_narrator_speak(
        &self,
        candidates: &[Candidate],
        incoming: &Message,
        messages_since_narration: u64,
    ) -> bool {
        // No cast to speak => narrator by necessity.
        if candidates.is_empty() {
            return true;
        }

        // Scene-setting turns are the narrator's.
        if matches!(incoming.parts.primary().kind, MessageKind::Narration) {
            return true;
        }

        let roll = rand::random::<f32>();
        // If the incoming message is OOC, the narrator has a chance to speak.
        if matches!(incoming.parts.primary().kind, MessageKind::Ooc) {
            return roll < self.director_settings.narrator_change_if_ooc;
        }

        roll < self.narrator_chance(messages_since_narration)
    }

    fn narrator_chance(&self, messages_since_narration: u64) -> f32 {
        let (min, max) = self
            .director_settings
            .narrator_pick_chance_after_message_threshold_range
            .normalized();
        if messages_since_narration < min {
            return 0.0;
        }
        if messages_since_narration >= max {
            return 1.0;
        }

        let span = (max - min) as f32;
        (messages_since_narration - min) as f32 / span
    }

    pub fn pick_character<'a>(
        &self,
        candidates: &'a [Candidate],
        incoming: &Message,
    ) -> Option<&'a Candidate> {
        let roll = rand::random::<f32>();
        if roll < self.director_settings.targeted_pick_chance {
            let targeted = self.targeted_character(candidates, incoming)?;
            Some(targeted)
        } else {
            None
        }
    }

    /// The cast member named as a target on any part of the incoming message.
    fn targeted_character<'a>(
        &self,
        candidates: &'a [Candidate],
        incoming: &Message,
    ) -> Option<&'a Candidate> {
        let target = incoming
            .parts
            .iter()
            .find_map(|part| part.target_name.as_deref())?;

        candidates.iter().find(|c| c.name == target)
    }
}
