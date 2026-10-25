use std::rc::Rc;

use chrono::Local;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use crate::{
    app::AppContext,
    backend::application::port::chat_port::{ChatHandler, NarratorHandler},
    backend::application::port::memory_port::MemoryDistiller,
    backend::domain::message::Message,
    ui::{ChatMessage, RootWindow},
};

// TODO: Make this enum
// Page codes
const PAGE_HOME: i32 = 0;
const PAGE_CHAT: i32 = 1;

// Tab codes
const TAB_CAMPAIGNS: i32 = 0;

// Chat action modes (must match the order of `action-modes` in app.slint).
const MODE_SAY: i32 = 0;
const MODE_DO: i32 = 1;
const MODE_THINK: i32 = 2;
const MODE_WHISPER: i32 = 3;

// Sentinel for "no secondary action selected" (see ChatCenter in chat.slint).
const MODE_NONE: i32 = -1;

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn register_callbacks<
    C: ChatHandler + Clone + 'static,
    N: NarratorHandler + Clone + 'static,
    M: MemoryDistiller + Clone + 'static,
>(
    ui: &RootWindow,
    ctx: &AppContext<C, N, M>,
) {
    register_main_callbacks(ui, ctx);
    register_chat_callbacks(ui, ctx);
    register_shared_callbacks(ui);
}

// ---------------------------------------------------------------------------
// Home / Adventures page
// ---------------------------------------------------------------------------

fn register_main_callbacks<
    C: ChatHandler + Clone + 'static,
    N: NarratorHandler + Clone + 'static,
    M: MemoryDistiller + Clone + 'static,
>(
    ui: &RootWindow,
    _ctx: &AppContext<C, N, M>,
) {
    let ui_weak = ui.as_weak();

    ui.on_adventure_selected(move |title| {
        log::info!("adventure selected: {title}");
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_page(PAGE_CHAT);
            ui.set_active_tab(TAB_CAMPAIGNS);
        }
    });
}

// ---------------------------------------------------------------------------
// Chat page
// ---------------------------------------------------------------------------

fn register_chat_callbacks<
    C: ChatHandler + Clone + 'static,
    N: NarratorHandler + Clone + 'static,
    M: MemoryDistiller + Clone + 'static,
>(
    ui: &RootWindow,
    ctx: &AppContext<C, N, M>,
) {
    // Back the `messages` property with a VecModel we can push to over time.
    let seed: Vec<ChatMessage> = ui.get_messages().iter().collect();
    let messages: Rc<VecModel<ChatMessage>> = Rc::new(VecModel::from(seed));
    ui.set_messages(ModelRc::from(messages.clone()));

    // The async handle for the Tokio runtime `main` is already running on.
    // Model/DB calls run there; results are posted back to the UI thread via
    // `slint::invoke_from_event_loop`.
    let runtime = tokio::runtime::Handle::current();
    let message_manager = ctx.messages.clone();
    let adventures = ctx.adventures.clone();

    let ui_weak = ui.as_weak();
    let msgs = messages.clone();
    ui.on_send_message(move || {
        let Some(ui) = ui_weak.upgrade() else { return };

        let draft = ui.get_draft().to_string();
        let draft_trim = draft.trim();
        if draft_trim.is_empty() {
            return;
        }

        // Which action(s) the player picked: a primary plus an optional second.
        let primary = ui.get_mode_primary();
        let secondary = ui.get_mode_secondary();

        let target_name = ui.get_whisper_target().name.to_string();
        let target_trimmed = target_name.trim();
        // A Say / Whisper part needs someone to address.
        let needs_target = mode_needs_target(primary) || mode_needs_target(secondary);
        if needs_target && target_trimmed.is_empty() {
            log::debug!("send ignored: selected action needs a target");
            return;
        }

        let Some(adventure) = adventures.current_adventure() else {
            log::warn!("send ignored: no adventure loaded");
            return;
        };

        let speaker = ui.get_player_name().to_string();

        // 1) Show the player's line immediately, in order, at the bottom.
        push_player_bubble(
            &msgs,
            &speaker,
            draft_trim,
            primary,
            secondary,
            target_trimmed,
        );
        ui.set_draft(SharedString::new());

        // 2) Build the domain message (one or two parts) and send it.
        let domain_message = build_domain_message(
            &adventure.id,
            &speaker,
            draft_trim,
            primary,
            secondary,
            target_trimmed,
        );

        let manager = message_manager.clone();
        let ui_for_reply = ui.as_weak();
        runtime.spawn(async move {
            let result = manager.send_message(domain_message).await;
            // Hop back to the UI thread to mutate the model safely.
            let _ = slint::invoke_from_event_loop(move || {
                let Some(ui) = ui_for_reply.upgrade() else {
                    return;
                };
                let model = ui.get_messages();
                let Some(vec_model) = model.as_any().downcast_ref::<VecModel<ChatMessage>>() else {
                    return;
                };
                match result {
                    Ok(reply) => vec_model.push(reply_to_bubble(&reply)),
                    Err(err) => {
                        log::error!("send_message failed: {err}");
                        vec_model.push(error_bubble(&err.to_string()));
                    }
                }
            });
        });
    });
}

/// Does this UI mode address another character (Say / Whisper)?
fn mode_needs_target(mode: i32) -> bool {
    matches!(mode, MODE_SAY | MODE_WHISPER)
}

/// Map a UI mode to a short prose prefix so the flat message text carries the
/// action. The domain no longer has a message kind — the text is pre-formatted.
fn mode_to_prefix(mode: i32) -> &'static str {
    match mode {
        MODE_SAY => "says",
        MODE_DO => "does",
        MODE_THINK => "thinks",
        MODE_WHISPER => "whispers",
        _ => "says",
    }
}

/// The UI `kind` string used for styling a bubble (keeps "whisper" distinct).
fn mode_to_ui_kind(mode: i32) -> &'static str {
    match mode {
        MODE_SAY => "say",
        MODE_DO => "do",
        MODE_THINK => "think",
        MODE_WHISPER => "whisper",
        _ => "say",
    }
}

/// Render one action as a line of ready-formatted text.
fn format_part(mode: i32, text: &str, target: &str) -> String {
    if mode_needs_target(mode) && !target.is_empty() {
        format!("{} to {}: {}", mode_to_prefix(mode), target, text)
    } else {
        format!("{}: {}", mode_to_prefix(mode), text)
    }
}

/// Assemble the domain message from the draft and the one or two chosen modes.
fn build_domain_message(
    adventure_id: &uuid::Uuid,
    speaker: &str,
    text: &str,
    primary: i32,
    secondary: i32,
    target: &str,
) -> Message {
    let mut body = format_part(primary, text, target);
    if secondary != MODE_NONE {
        body.push('\n');
        body.push_str(&format_part(secondary, text, target));
    }

    // Player-authored line: not tied to a cast character or the narrator.
    Message::new(
        *adventure_id,
        None,
        None,
        speaker.to_string(),
        body,
        false,
        vec![],
    )
}

/// Append the player's own line to the visible log (right-aligned, "own").
fn push_player_bubble(
    msgs: &Rc<VecModel<ChatMessage>>,
    speaker: &str,
    text: &str,
    primary: i32,
    secondary: i32,
    target: &str,
) {
    let time: SharedString = Local::now().format("%H:%M").to_string().into();
    // The bubble's caption shows the primary mode; a combined turn still reads
    // naturally because the text carries both actions.
    let ui_kind = mode_to_ui_kind(if secondary != MODE_NONE && primary == MODE_THINK {
        secondary // prefer the spoken/acted caption over a silent "think"
    } else {
        primary
    });
    let show_target = mode_needs_target(primary) || mode_needs_target(secondary);

    msgs.push(ChatMessage {
        speaker: speaker.into(),
        text: text.into(),
        time,
        narrator: false,
        own: true,
        kind: ui_kind.into(),
        target: if show_target {
            target.into()
        } else {
            SharedString::new()
        },
    });
}

/// Convert a persisted reply (character or narrator) into a UI bubble.
fn reply_to_bubble(reply: &Message) -> ChatMessage {
    let is_narrator = reply.narrator_id.is_some();
    let time: SharedString = reply
        .created_at
        .with_timezone(&Local)
        .format("%H:%M")
        .to_string()
        .into();

    ChatMessage {
        speaker: reply.speaker_name.as_str().into(),
        text: reply.text.as_str().into(),
        time,
        narrator: is_narrator,
        own: false,
        kind: SharedString::new(),
        target: SharedString::new(),
    }
}

/// A non-blocking inline notice when a send fails (e.g. model unreachable).
fn error_bubble(message: &str) -> ChatMessage {
    ChatMessage {
        speaker: "System".into(),
        text: format!("Could not get a reply: {message}").into(),
        time: Local::now().format("%H:%M").to_string().into(),
        narrator: true,
        own: false,
        kind: SharedString::new(),
        target: SharedString::new(),
    }
}

// ---------------------------------------------------------------------------
// Shared chrome (top bar nav)
// ---------------------------------------------------------------------------

fn register_shared_callbacks(ui: &RootWindow) {
    let ui_weak = ui.as_weak();

    ui.on_nav_tab_selected(move |index| {
        let Some(ui) = ui_weak.upgrade() else { return };
        if index == TAB_CAMPAIGNS {
            ui.set_page(PAGE_HOME);
        }
    });
}
