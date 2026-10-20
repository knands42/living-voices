use crate::backend::application::port::chat_port::Candidate;
use crate::backend::domain::message::Message;
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
        if incoming.narrator_id.is_some() {
            return true;
        }

        let roll = rand::random::<f32>();
        // If the incoming message is OOC, the narrator has a chance to speak.
        if incoming.ooc {
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
        // TODO: Implement logic for picking a character when the roll is not for the targeted character.
    }

    /// A cast member chosen at random from the incoming message's targets.
    fn targeted_character<'a>(
        &self,
        candidates: &'a [Candidate],
        incoming: &Message,
    ) -> Option<&'a Candidate> {
        let targeted = incoming.targeted_characters.as_ref()?;

        let in_cast: Vec<&'a Candidate> = candidates
            .iter()
            .filter(|c| targeted.iter().any(|t| t.name == c.name))
            .collect();

        if in_cast.is_empty() {
            return None;
        }

        let pick = rand::random_range(0..in_cast.len());
        Some(in_cast[pick])
    }
}
