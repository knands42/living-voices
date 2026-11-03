use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Narrator {
    pub id: Uuid,
    pub adventure_id: Uuid,
    pub name: String,
    pub tone: String,
}

impl Narrator {
    pub fn new(
        id: Uuid,
        adventure_id: Uuid,
        name: impl Into<String>,
        tone: impl Into<String>,
    ) -> Self {
        Self {
            id,
            adventure_id,
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
