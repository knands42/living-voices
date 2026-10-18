use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::character::Character;
use crate::backend::domain::message::Message;
use crate::backend::domain::narrator::Narrator;

#[derive(Clone, Debug)]
pub struct Director {}

enum Speaker {
    NPC(Character),
    Narrator(Narrator),
    None,
}

impl Director {
    pub fn new() -> Self {
        Self {}
    }

    pub fn next_speaker(self, adventure: &Adventure, incoming: Message) -> Speaker {
        todo!()
    }
}
