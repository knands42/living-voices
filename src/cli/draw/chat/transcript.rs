//! Transcript body formatting: turning a stored message's raw text into styled
//! spans. This is the one piece of the chat screen that is parsing, not layout —
//! it scans for speech/action/thought markers and colours each segment.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::backend::domain::message::Message;
use crate::cli::draw::theme::{GOLD, dim, ink, muted, speaker_color};

pub(super) fn message_line<'a>(message: &Message) -> Line<'a> {
    // The narrator speaks in the voice of the world, not a cast member — give it
    // its own gold sigil and italic name so it reads as scene, not dialogue.
    let is_narrator = message.narrator_id.is_some();
    let hue = if is_narrator {
        GOLD
    } else {
        speaker_color(&message.speaker_name)
    };

    let gutter_glyph = if is_narrator { "❧ " } else { "▍ " };
    let gutter = Span::styled(gutter_glyph, Style::default().fg(hue));

    let name_modifier = if is_narrator {
        Modifier::BOLD | Modifier::ITALIC
    } else {
        Modifier::BOLD
    };

    let mut spans = vec![
        gutter,
        Span::styled(
            message.speaker_name.clone(),
            Style::default().fg(hue).add_modifier(name_modifier),
        ),
        Span::styled(" ❯ ", dim()),
    ];
    spans.extend(format_body(&message.text, hue));

    Line::from(spans)
}

enum Marker {
    Speech,
    Action,
    Thought,
}

impl Marker {
    fn from_symbol(c: char, next: Option<char>) -> Option<(Marker, &'static str)> {
        match c {
            '"' => Some((Marker::Speech, "\"")),
            '*' => Some((Marker::Action, "*")),
            '(' if next == Some('(') => Some((Marker::Thought, "))")),
            _ => None,
        }
    }

    fn style(&self, hue: Color) -> Style {
        match self {
            Marker::Speech => Style::default().fg(hue),
            Marker::Action => muted(),
            Marker::Thought => dim().add_modifier(Modifier::ITALIC),
        }
    }
}

fn format_body<'a>(text: &str, hue: Color) -> Vec<Span<'a>> {
    let mut spans: Vec<Span> = Vec::new();
    let mut plain = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();

        if let Some((marker, close)) = Marker::from_symbol(c, next) {
            let open_len = if matches!(marker, Marker::Thought) {
                2
            } else {
                1
            };
            let body_start = i + open_len;

            if let Some(end) = find_close_char(&chars, body_start, close) {
                // Flush the plain run that preceded this marker.
                if !plain.is_empty() {
                    spans.push(Span::styled(std::mem::take(&mut plain), ink()));
                }

                let segment: String = chars[i..end + close.chars().count()].iter().collect();
                spans.push(Span::styled(segment, marker.style(hue)));
                i = end + close.chars().count();
                continue;
            }
        }

        plain.push(c);
        i += 1;
    }

    if !plain.is_empty() {
        spans.push(Span::styled(plain, ink()));
    }
    if spans.is_empty() {
        spans.push(Span::styled(String::new(), ink()));
    }
    spans
}

fn find_close_char(chars: &[char], from: usize, close: &str) -> Option<usize> {
    let close: Vec<char> = close.chars().collect();
    if close.is_empty() || from >= chars.len() {
        return None;
    }
    let last = chars.len().checked_sub(close.len())?;
    (from..=last).find(|&start| chars[start..start + close.len()] == close[..])
}

#[cfg(test)]
mod tests {
    use super::*;

    const HUE: Color = Color::Rgb(0, 0, 0);

    /// Collect the rendered segments as (text, style) for easy assertions.
    fn segments(text: &str) -> Vec<String> {
        format_body(text, HUE)
            .into_iter()
            .map(|span| span.content.into_owned())
            .collect()
    }

    #[test]
    fn plain_text_is_one_ink_span() {
        let spans = format_body("just narration", HUE);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "just narration");
        assert_eq!(spans[0].style, ink());
    }

    #[test]
    fn speech_is_isolated_from_surrounding_plain_text() {
        // leading plain, a quoted segment, trailing plain => three spans.
        assert_eq!(
            segments("she said \"hello\" softly"),
            vec!["she said ", "\"hello\"", " softly"],
        );
    }

    #[test]
    fn action_and_thought_markers_each_become_their_own_span() {
        assert_eq!(
            segments("*nods* ((unsure))"),
            vec!["*nods*", " ", "((unsure))"]
        );
    }

    #[test]
    fn speech_segment_keeps_the_speaker_hue() {
        let spans = format_body("\"hi\"", HUE);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].style, Style::default().fg(HUE));
    }

    #[test]
    fn an_unclosed_marker_stays_plain() {
        // A '*' with no closing '*' must not swallow the rest of the line.
        let spans = format_body("*starts but never ends", HUE);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "*starts but never ends");
        assert_eq!(spans[0].style, ink());
    }

    #[test]
    fn empty_text_yields_a_single_empty_span() {
        let spans = format_body("", HUE);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "");
    }

    #[test]
    fn multibyte_text_around_a_marker_does_not_panic_and_splits_correctly() {
        // Ensures the char-index arithmetic holds with non-ASCII before the marker.
        assert_eq!(segments("café *shrugs*"), vec!["café ", "*shrugs*"]);
    }
}
