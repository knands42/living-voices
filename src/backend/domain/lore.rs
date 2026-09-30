use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Lore {
    pub id: Uuid,
    pub text: String
}

impl Lore {
    pub fn new() -> Self {
        Lore {
            id: Uuid::new_v4(),
            text: String::new()
        }
    }

    pub fn with_text(mut self, text: String) -> Self {
        self.text = text;
        self
    }
}