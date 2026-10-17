use std::rc::Rc;

use chrono::Local;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use crate::{
    app::AppContext,
    ui::{ChatMessage, RootWindow},
};

// TODO: Make this enum
// Page codes
const PAGE_HOME: i32 = 0;
const PAGE_CHAT: i32 = 1;

// Tab codes
const TAB_CAMPAIGNS: i32 = 0;

// Chat action modes
const MODE_SAY: i32 = 0;
const MODE_DO: i32 = 1;
const MODE_THINK: i32 = 2;
const MODE_WHISPER: i32 = 3;

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn register_callbacks(ui: &RootWindow, ctx: &AppContext) {
    register_main_callbacks(ui, ctx);
    register_chat_callbacks(ui, ctx);
    register_shared_callbacks(ui);
}

// ---------------------------------------------------------------------------
// Home / Adventures page
// ---------------------------------------------------------------------------

fn register_main_callbacks(ui: &RootWindow, _ctx: &AppContext) {
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

fn register_chat_callbacks(ui: &RootWindow, _ctx: &AppContext) {
    let seed: Vec<ChatMessage> = ui.get_messages().iter().collect();
    let messages: Rc<VecModel<ChatMessage>> = Rc::new(VecModel::from(seed));
    ui.set_messages(ModelRc::from(messages.clone()));

    let ui_weak = ui.as_weak();
    let msgs = messages.clone();
    ui.on_send_message(move || {
        let Some(ui) = ui_weak.upgrade() else { return };

        let draft = ui.get_draft().to_string();
        let draft_trim = draft.trim();
        if draft_trim.is_empty() {
            return;
        }

        let kind = mode_kind(ui.get_active_mode());
        let needs_target = matches!(kind, "say" | "whisper");

        let target = ui.get_whisper_target();
        let target_name = target.name.to_string();
        if needs_target && target_name.trim().is_empty() {
            log::debug!("send ignored: {kind} requires a target");
            return;
        }

        let speaker = ui.get_player_name();
        let time: SharedString = Local::now().format("%H:%M").to_string().into();

        msgs.push(ChatMessage {
            speaker,
            text: draft_trim.into(),
            time,
            narrator: false,
            own: true,
            kind: kind.into(),
            target: if needs_target {
                target_name.into()
            } else {
                SharedString::new()
            },
        });

        // Clear the composer so the next message can be typed immediately.
        ui.set_draft(SharedString::new());
    });
}

fn mode_kind(mode: i32) -> &'static str {
    match mode {
        MODE_SAY => "say",
        MODE_DO => "do",
        MODE_THINK => "think",
        MODE_WHISPER => "whisper",
        _ => "say",
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
