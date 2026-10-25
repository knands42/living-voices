use crate::ai::MemoryUpdate;
use crate::ai::integrations::chat::Chat;
use crate::ai::integrations::narrator::Narrator;
use crate::app::AppContext;
use crate::cli::state::CliState;

pub fn render(
    frame: &mut ratatui::Frame,
    state: &CliState,
    ctx: &AppContext<Chat, Narrator, MemoryUpdate>,
) {
}
