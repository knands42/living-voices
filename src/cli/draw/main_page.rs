use crate::cli::state::CliState;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::prelude::{Line, Modifier, Span, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn render(frame: &mut ratatui::Frame, state: &CliState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(frame.area());

    let title = Paragraph::new(Line::from(vec![Span::styled(
        "Living Voices — CLI",
        Style::default().add_modifier(Modifier::BOLD),
    )]))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));

    let body = Paragraph::new(vec![
        Line::from(""),
        Line::from("Welcome to the Living Voices terminal interface."),
        Line::from(""),
        Line::from(Span::styled(
            "Press q or Esc to quit.",
            Style::default().add_modifier(Modifier::DIM),
        )),
    ])
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL).title(" Status "));

    frame.render_widget(title, chunks[0]);
    frame.render_widget(body, chunks[1]);
}
