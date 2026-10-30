use tokio::runtime::Handle;

use crate::ai::integrations::chat::Chat;
use crate::ai::integrations::narrator::Narrator;
use crate::ai::{ActClose, MemoryUpdate};
use crate::app::AppContext;
use crate::backend::domain::message::Message;
use crate::cli::events::event_handler::Flow;
use crate::cli::state::{ChatEvent, CliState, Page};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

const DEFAULT_PLAYER_NAME: &str = "You";

pub fn handle_chat(
    key: KeyEvent,
    state: &mut CliState,
    ctx: &AppContext<Chat, Narrator, MemoryUpdate, ActClose>,
    runtime: &Handle,
) -> Flow {
    let Page::Chat { input, cursor } = &mut state.page else {
        return Flow::Continue;
    };

    match key.code {
        KeyCode::Char('q') if input.is_empty() => Flow::Quit,
        KeyCode::Char(c) => {
            insert_char(input, cursor, c);
            Flow::Continue
        }
        KeyCode::Backspace => {
            backspace(input, cursor);
            Flow::Continue
        }
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
            insert_char(input, cursor, '\n');
            Flow::Continue
        }
        KeyCode::Enter => {
            send_current_input(state, ctx, runtime);
            Flow::Continue
        }
        KeyCode::Left if key.modifiers.contains(KeyModifiers::CONTROL) => {
            *cursor = get_previous_word(input, *cursor);
            Flow::Continue
        }
        KeyCode::Right if key.modifiers.contains(KeyModifiers::CONTROL) => {
            *cursor = get_next_word(input, *cursor);
            Flow::Continue
        }
        KeyCode::Left => {
            *cursor = cursor.saturating_sub(1);
            Flow::Continue
        }
        KeyCode::Right => {
            let len = input.chars().count();
            *cursor = (*cursor + 1).min(len);
            Flow::Continue
        }
        KeyCode::Home => {
            *cursor = 0;
            Flow::Continue
        }
        KeyCode::End => {
            *cursor = input.chars().count();
            Flow::Continue
        }
        _ => Flow::Continue,
    }
}

/// Insert a char at the cursor (a char index) and advance the cursor.
fn insert_char(input: &mut String, cursor: &mut usize, c: char) {
    let byte = char_index_to_byte(input, *cursor);
    input.insert(byte, c);
    *cursor += 1;
}

/// Delete the char before the cursor and step the cursor back.
fn backspace(input: &mut String, cursor: &mut usize) {
    if *cursor == 0 {
        return;
    }
    let byte = char_index_to_byte(input, *cursor - 1);
    input.remove(byte);
    *cursor -= 1;
}

/// Map a cursor expressed in chars to the matching byte offset in the String,
/// so inserts land on a char boundary even with multibyte text.
fn char_index_to_byte(input: &str, char_index: usize) -> usize {
    input
        .char_indices()
        .nth(char_index)
        .map(|(byte, _)| byte)
        .unwrap_or(input.len())
}

fn send_current_input(
    state: &mut CliState,
    ctx: &AppContext<Chat, Narrator, MemoryUpdate, ActClose>,
    runtime: &Handle,
) {
    let text = {
        let Page::Chat { input, cursor } = &mut state.page else {
            return;
        };
        let text = input.trim().to_string();
        input.clear();
        *cursor = 0;
        text
    };

    if text.is_empty() {
        return;
    }

    let Some(adventure) = ctx.adventures.current_adventure() else {
        state.status = Some("open a tale before sending".into());
        return;
    };

    // Identify the player: the character flagged is_player, else a generic voice.
    let player = adventure.characters.iter().find(|c| c.is_player);
    let (character_id, speaker_name) = match player {
        Some(c) => (Some(c.id), c.name.clone()),
        None => (None, DEFAULT_PLAYER_NAME.to_string()),
    };

    let outgoing = Message::new(
        adventure.id,
        character_id,
        None,
        speaker_name,
        text,
        false,
        vec![],
    );

    // Instant echo: show the player's line before the model is even called.
    state.messages.push(outgoing.clone());
    state.status = Some("the voices are thinking…".into());

    // Fire-and-forget the send; report the result back over the channel.
    let messages = ctx.messages.clone();
    let tx = state.reply_tx.clone();
    runtime.spawn(async move {
        let event = match messages.send_message(outgoing).await {
            Ok(reply) => ChatEvent::Reply(reply),
            Err(err) => ChatEvent::Failed(err.to_string()),
        };
        let _ = tx.send(event);
    });
}

/// Cursor position one word to the left
fn get_previous_word(input: &str, cursor: usize) -> usize {
    let chars: Vec<char> = input.chars().collect();
    let mut i = cursor.min(chars.len());

    while i > 0 && chars[i - 1] == ' ' {
        i -= 1;
    }
    while i > 0 && chars[i - 1] != ' ' {
        i -= 1;
    }
    i
}

/// Cursor position one word to the right
fn get_next_word(input: &str, cursor: usize) -> usize {
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = cursor.min(len);

    while i < len && chars[i] != ' ' {
        i += 1;
    }
    while i < len && chars[i] == ' ' {
        i += 1;
    }
    i
}

/// Mouse handling for the chat page: hovering a cast row opens its overlay,
/// moving off every row closes it. The row hitboxes are recorded by the
/// renderer each frame into `state.cast_row_hitboxes`.
pub fn handle_mouse(
    mouse: MouseEvent,
    state: &mut CliState,
    _ctx: &AppContext<Chat, Narrator, MemoryUpdate, ActClose>,
) {
    if !matches!(state.page, Page::Chat { .. }) {
        return;
    }
    if !matches!(mouse.kind, MouseEventKind::Moved | MouseEventKind::Down(_)) {
        return;
    }

    let hit = state
        .cast_row_hitboxes
        .iter()
        .position(|row| point_in_rect(mouse.column, mouse.row, row));

    state.hovered_character = hit;
}

fn point_in_rect(x: u16, y: u16, rect: &Rect) -> bool {
    x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previous_word_jumps_to_the_start_of_the_word_to_the_left() {
        // given: cursor at the end of "hello world"
        // when: jumping left by word
        // then: it lands at the start of "world" (index 6)
        assert_eq!(get_previous_word("hello world", 11), 6);
    }

    #[test]
    fn previous_word_skips_trailing_spaces_before_the_word() {
        // given: cursor sitting just after the spaces in "hi   "
        // then: it skips the spaces and the word, landing at 0
        assert_eq!(get_previous_word("hi   ", 5), 0);
    }

    #[test]
    fn previous_word_at_the_start_stays_at_zero() {
        assert_eq!(get_previous_word("hello", 0), 0);
    }

    #[test]
    fn next_word_jumps_to_the_start_of_the_following_word() {
        // given: cursor at the start of "hello world"
        // then: it skips "hello" and the space, landing at 6
        assert_eq!(get_next_word("hello world", 0), 6);
    }

    #[test]
    fn next_word_from_mid_word_lands_on_the_next_word() {
        // given: cursor inside "hello" (index 2)
        // then: it finishes the word and the space, landing at 6
        assert_eq!(get_next_word("hello world", 2), 6);
    }

    #[test]
    fn next_word_at_the_end_stays_at_the_end() {
        assert_eq!(get_next_word("hello", 5), 5);
    }

    #[test]
    fn insert_char_lands_at_the_cursor_not_the_end() {
        // given: "ac" with cursor between a and c
        let mut input = String::from("ac");
        let mut cursor = 1;
        // when: inserting 'b'
        insert_char(&mut input, &mut cursor, 'b');
        // then: "abc" and the cursor advanced past b
        assert_eq!(input, "abc");
        assert_eq!(cursor, 2);
    }

    #[test]
    fn insert_char_respects_multibyte_boundaries() {
        // given: "café" with cursor at the end (4 chars, 5 bytes)
        let mut input = String::from("café");
        let mut cursor = 4;
        insert_char(&mut input, &mut cursor, '!');
        assert_eq!(input, "café!");
        assert_eq!(cursor, 5);
    }

    #[test]
    fn backspace_removes_the_char_before_the_cursor() {
        let mut input = String::from("abc");
        let mut cursor = 2;
        backspace(&mut input, &mut cursor);
        assert_eq!(input, "ac");
        assert_eq!(cursor, 1);
    }

    #[test]
    fn backspace_at_the_start_is_a_noop() {
        let mut input = String::from("abc");
        let mut cursor = 0;
        backspace(&mut input, &mut cursor);
        assert_eq!(input, "abc");
        assert_eq!(cursor, 0);
    }
}
