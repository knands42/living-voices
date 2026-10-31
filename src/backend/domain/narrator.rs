use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Narrator {
    pub id: Uuid,
    pub name: String,
    pub tone: String,
}

impl Narrator {
    const DEFAULT_TONE: &str = "Keep the tone casual and straightforward—clear, relaxed narration that stays out of the way and lets the story breathe";

    pub fn new(id: Uuid, name: impl Into<String>, tone: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            tone: tone.into(),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn with_tone(mut self, tone: impl Into<String>) -> Self {
        self.tone = tone.into();
        self
    }
}
