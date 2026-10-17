use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Narrator {
    pub id: Uuid,
    pub name: String,
}

impl Narrator {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }
}
