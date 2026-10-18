use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::prelude::{Line, Modifier, Span, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn main_page(frame: &mut ratatui::Frame) {
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
        Line::from("Select your adventure"),
        Line::from(Span::styled(
            "Press q or Esc to quit.",
            Style::default().add_modifier(Modifier::DIM),
        )),
    ])
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL).title(" Status "));

    let adventures = Paragraph::new(vec![
        Line::from(""),
        Line::from(""),
        Line::from(""),
        Line::from(""),
        Line::from(""),
    ])
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL).title(" Adventures "));

    frame.render_widget(title, chunks[0]);
    frame.render_widget(body, chunks[1]);
    frame.render_widget(adventures, chunks[2]);
}
