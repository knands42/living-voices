use iced::widget::rich_text;
use iced::widget::text::{Span, Wrapping};
use iced::{Color, Element, Font, Length};

use crate::ui::{common, theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Voice {
    Speech,
    Action,
    Thought,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece<'a> {
    pub text: &'a str,
    pub voice: Voice,
}

pub struct Look {
    size: f32,
    speech: Color,
    action: Color,
    thought: Color,
    base_font: Font,
}

impl Look {
    pub fn bubble(size: f32) -> Self {
        Self {
            size,
            speech: theme::TEXT_LIGHT,
            action: theme::TEXT_MUTED,
            thought: theme::TEXT_DIM,
            base_font: Font::DEFAULT,
        }
    }

    pub fn narrator(size: f32) -> Self {
        Self {
            size,
            speech: theme::TEXT_MUTED,
            action: theme::TEXT_DIM,
            thought: theme::TEXT_DIM,
            base_font: common::italic(),
        }
    }

    pub fn preview(size: f32) -> Self {
        Self {
            size,
            speech: theme::TEXT_MUTED,
            action: theme::TEXT_DIM,
            thought: theme::TEXT_DIM,
            base_font: Font::DEFAULT,
        }
    }
}

pub fn has_markup(body: &str) -> bool {
    pieces(body)
        .iter()
        .any(|piece| piece.voice != Voice::Speech)
}

pub fn pieces(body: &str) -> Vec<Piece<'_>> {
    let bytes = body.as_bytes();
    let mut pieces = Vec::new();
    let mut open: Option<(Voice, usize)> = None;
    let mut plain = 0;
    let mut index = 0;

    while index < bytes.len() {
        let Some((voice, width)) = delimiter(bytes, index, open.map(|(voice, _)| voice)) else {
            index += 1;
            continue;
        };

        let after = index + width;

        match open {
            None => {
                push(&mut pieces, &body[plain..index], Voice::Speech);
                open = Some((voice, after));
            }
            Some((_, start)) => {
                push(&mut pieces, &body[start..index], voice);
                open = None;
            }
        }

        plain = after;
        index = after;
    }

    let (voice, start) = open.unwrap_or((Voice::Speech, plain));
    push(&mut pieces, &body[start..], voice);

    pieces
}

// Every marker is ASCII, so a byte equal to one is that character: no char
// boundary can hide inside a multi-byte sequence and split a marker in half.
fn delimiter(bytes: &[u8], index: usize, open: Option<Voice>) -> Option<(Voice, usize)> {
    match (bytes[index], open) {
        // Runs collapse, so `**action**` reads as one delimiter too.
        (b'*', None | Some(Voice::Action)) => Some((Voice::Action, run(bytes, index, b'*'))),
        (b'(', None) if follows(bytes, index, b'(') => Some((Voice::Thought, 2)),
        (b')', Some(Voice::Thought)) if follows(bytes, index, b')') => Some((Voice::Thought, 2)),
        _ => None,
    }
}

fn run(bytes: &[u8], index: usize, marker: u8) -> usize {
    bytes[index..]
        .iter()
        .take_while(|byte| **byte == marker)
        .count()
}

fn follows(bytes: &[u8], index: usize, marker: u8) -> bool {
    bytes.get(index + 1) == Some(&marker)
}

fn push<'a>(pieces: &mut Vec<Piece<'a>>, text: &'a str, voice: Voice) {
    if !text.is_empty() {
        pieces.push(Piece { text, voice });
    }
}

pub fn rich<'a, Message: 'static>(body: &'a str, look: &Look) -> Element<'a, Message> {
    let spans = pieces(body)
        .into_iter()
        .map(|piece| match piece.voice {
            Voice::Speech => Span::new(piece.text).color(look.speech),
            Voice::Action => Span::new(piece.text)
                .color(look.action)
                .font(common::italic()),
            Voice::Thought => Span::new(piece.text)
                .color(look.thought)
                .font(common::italic()),
        })
        .collect::<Vec<_>>();

    rich_text::<(), Message, iced::Theme, iced::Renderer>(spans)
        .font(look.base_font)
        .size(look.size)
        .width(Length::Fill)
        .wrapping(Wrapping::Word)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_body_splits_into_its_voices_in_order() {
        // given
        let body = "*leans in* \"The gate stays shut.\" ((not tonight)) (to Kael) \"It stays.\"";

        // when
        let pieces = pieces(body);

        // then
        assert_eq!(
            pieces,
            vec![
                Piece {
                    text: "leans in",
                    voice: Voice::Action
                },
                Piece {
                    text: " \"The gate stays shut.\" ",
                    voice: Voice::Speech
                },
                Piece {
                    text: "not tonight",
                    voice: Voice::Thought
                },
                Piece {
                    text: " (to Kael) \"It stays.\"",
                    voice: Voice::Speech
                },
            ]
        );
    }

    #[test]
    fn an_unclosed_marker_runs_to_the_end_so_a_draft_can_be_previewed() {
        // given
        let body = "says nothing, then *raises the lantern";

        // when
        let pieces = pieces(body);

        // then
        assert_eq!(
            pieces,
            vec![
                Piece {
                    text: "says nothing, then ",
                    voice: Voice::Speech
                },
                Piece {
                    text: "raises the lantern",
                    voice: Voice::Action
                },
            ]
        );
    }

    #[test]
    fn a_marker_inside_another_voice_stays_literal() {
        // given
        let body = "((he *means* it))";

        // when
        let pieces = pieces(body);

        // then
        assert_eq!(
            pieces,
            vec![Piece {
                text: "he *means* it",
                voice: Voice::Thought
            }]
        );
    }
}
