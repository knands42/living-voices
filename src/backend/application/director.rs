use crate::backend::domain::character::Character;
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
        candidates: &[&Character],
        incoming: &Message,
        messages_since_narration: u8,
    ) -> bool {
        // No cast to speak => narrator by necessity.
        if candidates.is_empty() {
            return true;
        }

        // TODO: if narrator spoke last message, wait for at least 10 messages

        let roll = rand::random::<f32>();
        // If the incoming message is OOC, the narrator has a chance to speak.
        if incoming.ooc {
            return roll < self.director_settings.narrator_change_if_ooc;
        }

        roll < self.narrator_chance(messages_since_narration)
    }

    pub fn pick_character<'a>(
        &self,
        candidates: &'a [&Character],
        incoming: &Message,
    ) -> Option<&'a Character> {
        if candidates.is_empty() {
            return None;
        }

        // Most turns go to whoever the incoming message aimed at.
        let roll = rand::random::<f32>();
        if roll < self.director_settings.targeted_pick_chance {
            let targeted = self.targeted_character(candidates, incoming)?;
            Some(targeted)
        } else {
            None
        }
        // TODO: Implement logic for picking a character when the roll is not for the targeted character.
    }

    pub fn should_request_closure(&self, messages_in_act: u16) -> bool {
        messages_in_act >= self.director_settings.act_max_messages / 2
    }

    pub fn get_max_act_messages(&self) -> u16 {
        self.director_settings.act_max_messages
    }

    fn narrator_chance(&self, messages_since_narration: u8) -> f32 {
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

    /// A cast member chosen at random from the incoming message's targets.
    fn targeted_character<'a>(
        &self,
        candidates: &'a [&Character],
        incoming: &Message,
    ) -> Option<&'a Character> {
        let targeted = &incoming.targeted_characters;
        if targeted.is_empty() {
            return None;
        }

        let in_cast: Vec<&'a Character> = candidates
            .iter()
            .filter(|c| targeted.iter().any(|t| t.name == c.name))
            .copied()
            .collect();

        if in_cast.is_empty() {
            return None;
        }

        let pick = rand::random_range(0..in_cast.len());
        Some(in_cast[pick])
    }
}
