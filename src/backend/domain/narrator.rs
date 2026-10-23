use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Narrator {
    pub id: Uuid,
    pub name: String,
    pub tone: Option<String>,
}

impl Narrator {
    pub fn new(name: String, tone: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            tone,
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }
}
