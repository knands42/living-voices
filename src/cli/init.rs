use crate::ai::MemoryUpdate;
use crate::ai::jobs::chat::Chat;
use crate::ai::jobs::narrator::Narrator;
use crate::app::{App, AppContext};
use crate::cli::event_handler::{Flow, handle_key};
use crate::cli::state::CliState;
use crate::cli::{Tui, draw};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture, Event};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::{event, execute};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;
use std::time::Duration;
use tokio::runtime::Handle;

pub fn run(app: &App<Chat, Narrator, MemoryUpdate>) -> io::Result<()> {
    let mut terminal = setup_terminal()?;
    let mut state = CliState::default();
    let runtime = tokio::runtime::Handle::current();
    let result = event_loop(&mut terminal, &mut state, &app.ctx(), &runtime);
    restore_terminal(&mut terminal)?;
    result
}

fn event_loop(
    terminal: &mut Tui,
    state: &mut CliState,
    ctx: &AppContext<Chat, Narrator, MemoryUpdate>,
    runtime: &Handle,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| draw::draw(f, state, ctx))?;

        if event::poll(Duration::from_millis(250))?
            && let Event::Key(key) = event::read()?
            && matches!(handle_key(key, state, ctx, runtime), Flow::Quit)
        {
            return Ok(());
        }
    }
}

fn setup_terminal() -> io::Result<Tui> {
    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    Terminal::new(CrosstermBackend::new(stdout))
}

fn restore_terminal(terminal: &mut Tui) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
