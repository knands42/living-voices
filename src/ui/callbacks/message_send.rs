use crate::app::DefaultContext;
use crate::backend::domain::character::CharacterStatus;
use crate::backend::domain::message::Message;
use crate::ui::{ChatGlobals, RootWindow, viewmodels};
use slint::ComponentHandle;
use tokio::runtime::Handle;
use uuid::Uuid;

pub fn register_message_sent(ui: &RootWindow, ctx: &DefaultContext) {
    let ctx = ctx.clone();
    let handle = Handle::current();
    let ui_weak = ui.as_weak();

    ui.global::<ChatGlobals>().on_message_sent(move |text| {
        let Some(adventure) = ctx.adventures.current_adventure() else {
            log::error!("message sent with no active adventure");
            return;
        };

        let snapshot = ctx.adventures.current_snapshot();
        if let Some(player) = snapshot.player {
            let player_message = Message::new(
                adventure.id,
                Some(player.id),
                None,
                player.name,
                text.as_str(),
                player.status != CharacterStatus::Present,
            );

            update_message_view(&ui_weak, &player_message, player.id);

            let messages = ctx.messages.clone();
            let handle = handle.clone();
            let ui_weak = ui_weak.clone();
            handle.spawn(async move {
                let reply = messages.send_message(player_message).await;
                post_reply(&ui_weak, reply, player.id);
            });
        }
    });
}

fn update_message_view(ui_weak: &slint::Weak<RootWindow>, player: &Message, player_id: Uuid) {
    let ui_weak = ui_weak.clone();
    let view = viewmodels::message_view(player, Some(player_id));
    let post = slint::invoke_from_event_loop(move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };
        push_message(&ui, view);
        ui.global::<ChatGlobals>().set_awaiting_reply(true);
    });

    if let Err(err) = post {
        log::error!("could not echo player message: {err}");
    }
}

fn post_reply(
    ui_weak: &slint::Weak<RootWindow>,
    reply: Result<Message, crate::backend::application::message_manager::MessageManagerError>,
    player_id: Uuid,
) {
    let ui_weak = ui_weak.clone();
    let post = slint::invoke_from_event_loop(move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };
        ui.global::<ChatGlobals>().set_awaiting_reply(false);
        match reply {
            Ok(message) => push_message(&ui, viewmodels::message_view(&message, Some(player_id))),
            Err(err) => log::error!("send_message failed: {err}"),
        }
    });

    if let Err(err) = post {
        log::error!("could not post reply to UI: {err}");
    }
}

fn push_message(ui: &RootWindow, view: crate::ui::MessageView) {
    use slint::{Model, VecModel};
    let chat = ui.global::<ChatGlobals>();
    let model = chat.get_messages();
    if let Some(vec_model) = model
        .as_any()
        .downcast_ref::<VecModel<crate::ui::MessageView>>()
    {
        vec_model.push(view);
    } else {
        let items: Vec<crate::ui::MessageView> = model.iter().collect();
        let fresh = VecModel::from(items);
        fresh.push(view);
        chat.set_messages(slint::ModelRc::new(fresh));
    }
}
