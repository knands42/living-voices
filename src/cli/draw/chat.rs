use crate::ai::MemoryUpdate;
use crate::ai::jobs::chat::Chat;
use crate::ai::jobs::narrator::Narrator;
use crate::app::AppContext;
use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::character::{Character, CharacterStatus};
use crate::backend::domain::message::Message;
use crate::cli::draw::avatar::{Avatar, GlyphAvatar};
use crate::cli::draw::theme::{
    CARD_EDGE, EMBER, GOLD, GOLD_FADED, INK, dim, ink, muted, speaker_color,
};
use crate::cli::draw::widgets::{StatusKind, card, hints, pill, status_dot};
use crate::cli::state::{CliState, Page};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};
use uuid::Uuid;

pub fn render(frame: &mut Frame, state: &CliState, ctx: &AppContext<Chat, Narrator, MemoryUpdate>) {
    let input = match &state.page {
        Page::Chat { input } => input.as_str(),
        _ => "",
    };
    let adventure = ctx.adventures.current_adventure();
    let adventure_ref = adventure.as_ref();

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(5),    // body (3 columns)
            Constraint::Length(3), // composer
            Constraint::Length(1), // hints
        ])
        .split(frame.area());

    render_header(frame, rows[0], adventure_ref);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(26), // cast
            Constraint::Min(30),    // chat
            Constraint::Length(28), // lore
        ])
        .split(rows[1]);

    render_cast(frame, cols[0], adventure_ref);
    render_transcript(frame, cols[1], adventure_ref);
    render_meta(frame, cols[2], adventure_ref);

    render_composer(frame, rows[2], input, adventure_ref);
    render_hints(frame, rows[3]);
}

// ---------- Header ----------

fn render_header(frame: &mut Frame, area: Rect, adventure: Option<&Adventure>) {
    let block = Block::default()
        .borders(ratatui::widgets::Borders::BOTTOM)
        .border_style(Style::default().fg(CARD_EDGE));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let (title, tone, narrator) = match adventure {
        Some(a) => (a.title.as_str(), a.tone.as_str(), a.narrator.name.as_str()),
        None => ("No tale open", "—", "—"),
    };

    let line = Line::from(vec![
        Span::styled("✦ ", Style::default().fg(GOLD)),
        Span::styled(
            title,
            Style::default().fg(EMBER).add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
        pill(tone, EMBER),
        Span::raw("  "),
        Span::styled("narrator ", dim()),
        Span::styled(narrator, ink().add_modifier(Modifier::BOLD)),
    ]);
    frame.render_widget(Paragraph::new(line), inner);
}

// ---------- Cast sidebar ----------

fn render_cast(frame: &mut Frame, area: Rect, adventure: Option<&Adventure>) {
    let block = card("Cast");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(adventure) = adventure else {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled("no cast", muted())))
                .alignment(Alignment::Center),
            inner,
        );
        return;
    };

    let mut row_top = inner.top();

    // Row is 3 lines tall: [avatar+name] [role+status] [spacer]
    for (i, character) in adventure.characters.iter().enumerate() {
        let row_h: u16 = 3;
        if row_top + row_h > inner.bottom() {
            break;
        }
        let row = Rect {
            x: inner.left(),
            y: row_top,
            width: inner.width,
            height: row_h,
        };
        render_cast_row(frame, row, character, i == 0);
        row_top += row_h;
    }
}

fn render_cast_row(frame: &mut Frame, area: Rect, character: &Character, active: bool) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(4), Constraint::Min(1)])
        .split(area);

    // Avatar square (2 cells tall inside the 3-line row).
    let avatar_area = Rect {
        x: cols[0].left() + 1,
        y: cols[0].top(),
        width: 3,
        height: 2,
    };
    // TODO: Make this a profile pic
    let mut avatar = GlyphAvatar::from_name(&character.name);
    let buf = frame.buffer_mut();
    avatar.render(avatar_area, buf);

    // Right side: name + status dot on line 1, role on line 2.
    let name_color = if active { EMBER } else { INK };
    let name_line = Line::from(vec![
        Span::raw(" "),
        status_dot(to_status_kind(character.status)),
        Span::raw(" "),
        Span::styled(
            character.name.as_str(),
            Style::default().fg(name_color).add_modifier(if active {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
        ),
    ]);

    let role_line = Line::from(vec![
        Span::raw(" "),
        Span::styled(character.status.as_str(), muted()),
    ]);
    let text_area = Rect {
        x: cols[1].left(),
        y: cols[1].top(),
        width: cols[1].width.saturating_sub(1),
        height: 2,
    };
    frame.render_widget(Paragraph::new(vec![name_line, role_line]), text_area);
}

fn to_status_kind(status: CharacterStatus) -> StatusKind {
    match status {
        CharacterStatus::Present => StatusKind::Present,
        CharacterStatus::Away => StatusKind::Away,
        CharacterStatus::Unavailable => StatusKind::Unavailable,
    }
}

// ---------- Transcript (center) ----------

fn render_transcript(frame: &mut Frame, area: Rect, adventure: Option<&Adventure>) {
    let block = card("Scene");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = match adventure {
        Some(a) => vec![message_line(Message::new(
            a.id,
            Some(a.characters.iter().next().unwrap().id),
            None,
            a.characters.iter().next().unwrap().name.clone(),
            "This is a test".to_string(),
            false,
            vec![],
        ))],
        None => vec![Line::from(Span::styled(
            "open a tale to begin the scene",
            muted(),
        ))],
    };

    let p = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .alignment(Alignment::Left);
    frame.render_widget(p, inner);
}

fn message_line<'a>(message: Message) -> Line<'a> {
    let hue = speaker_color(&message.speaker_name);
    let gutter = Span::styled("▌ ", Style::default().fg(hue));

    Line::from(vec![
        gutter,
        Span::styled(
            message.speaker_name,
            Style::default().fg(hue).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" · ", dim()),
        Span::styled(message.text, ink()),
    ])
}

// ---------- Meta rail (right) ----------

fn render_meta(frame: &mut Frame, area: Rect, adventure: Option<&Adventure>) {
    let block = card("Lore");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(adventure) = adventure else {
        return;
    };

    let tags_line: Line = if adventure.tags.is_empty() {
        Line::from(Span::styled("no tags", muted()))
    } else {
        let mut spans: Vec<Span> = Vec::with_capacity(adventure.tags.len() * 2);
        for (i, tag) in adventure.tags.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw(" "));
            }
            spans.push(pill(tag, GOLD_FADED));
        }
        Line::from(spans)
    };

    let genre = adventure.genre.as_deref().unwrap_or("—");

    let body = Paragraph::new(vec![
        Line::from(Span::styled("Genre", dim())),
        Line::from(Span::styled(genre, ink().add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled("Tone", dim())),
        Line::from(Span::styled(adventure.tone.as_str(), ink())),
        Line::from(""),
        Line::from(Span::styled("Tags", dim())),
        tags_line,
    ])
    .wrap(Wrap { trim: false });
    frame.render_widget(body, inner);
}

// ---------- Composer ----------

fn render_composer(frame: &mut Frame, area: Rect, input: &str, adventure: Option<&Adventure>) {
    let block = card("Say");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let target = adventure
        .and_then(|a| a.characters.first())
        .map(|c| c.name.as_str())
        .unwrap_or("—");

    let caret = Span::styled(
        "▎",
        Style::default()
            .fg(EMBER)
            .add_modifier(Modifier::SLOW_BLINK),
    );

    let line = Line::from(vec![
        pill("say", EMBER),
        Span::raw(" "),
        Span::styled("→ ", dim()),
        Span::styled(target, ink().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(input, ink()),
        caret,
    ]);
    frame.render_widget(Paragraph::new(line), inner);
}

// ---------- Hints ----------

fn render_hints(frame: &mut Frame, area: Rect) {
    let bar = hints(&[
        ("1", "say"),
        ("2", "do"),
        ("3", "think"),
        ("4", "be away"),
        ("enter", "send"),
        ("tab", "cycle kind"),
        ("@", "target"),
        ("esc", "back"),
    ]);
    frame.render_widget(bar, area);
}
