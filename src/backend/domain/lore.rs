use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Lore {
    pub id: Uuid,
    pub text: String,
}

impl Lore {
    pub fn new(id: Uuid, text: impl Into<String>) -> Self {
        Lore {
            id,
            text: text.into(),
        }
    }

    pub fn with_text(mut self, text: String) -> Self {
        self.text = text;
        self
    }
}
