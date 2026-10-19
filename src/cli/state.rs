use crate::backend::domain::adventure::Adventure;
use ratatui::DefaultTerminal;

pub enum Page {
    Main,
    AdventureList,
    Chat { input: String },
}
pub struct CliState {
    pub page: Page,
    pub adventures: Vec<Adventure>,
    pub status: Option<String>,
}

impl Default for CliState {
    fn default() -> Self {
        Self {
            page: Page::Main,
            adventures: vec![],
            status: None,
        }
    }
}
