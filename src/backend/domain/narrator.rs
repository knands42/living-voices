use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Narrator {
    pub id: Uuid,
    pub name: String,
    pub tone: String,
}

impl Narrator {
    const DEFAULT_TONE: &str = "Keep the tone casual and straightforward—clear, relaxed narration that stays out of the way and lets the story breathe";

    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            tone: String::from(Self::DEFAULT_TONE),
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
