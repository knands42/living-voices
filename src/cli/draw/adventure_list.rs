use crate::ai::integrations::chat::Chat;
use crate::ai::integrations::narrator::Narrator;
use crate::ai::{ActClose, MemoryUpdate};
use crate::app::AppContext;
use crate::cli::state::CliState;

pub fn render(
    _frame: &mut ratatui::Frame,
    _state: &CliState,
    _ctx: &AppContext<Chat, Narrator, MemoryUpdate, ActClose>,
) {
}
