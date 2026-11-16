use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Narrator {
    pub id: Uuid,
    pub adventure_id: Uuid,
    pub name: String,
    pub instructions: String,
}

impl Narrator {
    pub fn new(
        id: Uuid,
        adventure_id: Uuid,
        name: impl Into<String>,
        instructions: impl Into<String>,
    ) -> Self {
        Self {
            id,
            adventure_id,
            name: name.into(),
            instructions: instructions.into(),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn with_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = instructions.into();
        self
    }
}
