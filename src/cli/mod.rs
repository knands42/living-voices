mod draw;
mod events;
mod init;
mod state;

pub use init::run;

use std::io::Stdout;

use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

type Tui = Terminal<CrosstermBackend<Stdout>>;
