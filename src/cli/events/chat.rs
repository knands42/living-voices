use tokio::runtime::Handle;

use crate::ai::MemoryUpdate;
use crate::ai::integrations::chat::Chat;
use crate::ai::integrations::narrator::Narrator;
use crate::app::AppContext;
use crate::backend::domain::message::Message;
use crate::cli::events::event_handler::Flow;
use crate::cli::state::{ChatEvent, CliState, Page};
use crossterm::event::{KeyCode, KeyEvent};

const DEFAULT_PLAYER_NAME: &str = "You";

pub fn handle_chat(
    key: KeyEvent,
    state: &mut CliState,
    ctx: &AppContext<Chat, Narrator, MemoryUpdate>,
    runtime: &Handle,
) -> Flow {
    let Page::Chat { input } = &mut state.page else {
        return Flow::Continue;
    };

    match key.code {
        KeyCode::Char('q') if input.is_empty() => Flow::Quit,
        KeyCode::Char(c) => {
            input.push(c);
            Flow::Continue
        }
        KeyCode::Backspace => {
            input.pop();
            Flow::Continue
        }
        KeyCode::Enter => {
            send_current_input(state, ctx, runtime);
            Flow::Continue
        }
        _ => Flow::Continue,
    }
}

fn send_current_input(
    state: &mut CliState,
    ctx: &AppContext<Chat, Narrator, MemoryUpdate>,
    runtime: &Handle,
) {
    let text = {
        let Page::Chat { input } = &mut state.page else {
            return;
        };
        let text = input.trim().to_string();
        input.clear();
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
