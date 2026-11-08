use crate::app::{DefaultContext, DefaultMessageManager};
use crate::backend::application::adventure_manager::{
    AdventureManager, AdventureManagerError, MemorySnapshot,
};
use crate::backend::application::character_gallery::CharacterGallery;
use crate::backend::application::message_manager::MessageManagerError;
use crate::backend::domain::character::CharacterStatus;
use crate::backend::domain::message::Message;
use crate::ui::callbacks::parse_id;
use crate::ui::{ChatGlobals, MainGlobals, MessageView, RootWindow, notice, viewmodels};
use slint::ComponentHandle;
use tokio::runtime::Handle;
use uuid::Uuid;

/// Everything a chat-page callback needs, cheap to clone into each closure.
///
/// Opening an adventure is the one expensive read on this page: it warms the snapshot,
/// and every render afterwards comes from that snapshot.
#[derive(Clone)]
struct ChatPageContext {
    handle: Handle,
    adventures: AdventureManager,
    messages: DefaultMessageManager,
    gallery: CharacterGallery,
    ui_weak: slint::Weak<RootWindow>,
}

impl ChatPageContext {
    fn open(&self, adventure_id: Uuid) -> Result<(), AdventureManagerError> {
        let (snapshot, messages) = tokio::task::block_in_place(|| {
            self.handle.block_on(async {
                let adventure = self.adventures.get_adventure(adventure_id).await?;
                self.adventures.start_adventure(adventure.id).await?;
                let messages = self.adventures.get_messages(adventure.id, 50).await?;
                log::info!("adventure started: {adventure_id}");

                Ok::<_, AdventureManagerError>((self.adventures.current_snapshot(), messages))
            })
        })?;

        self.show(snapshot, &messages);
        self.reveal();
        Ok(())
    }

    /// Callbacks run on the UI thread, so the window can be updated directly.
    fn show(&self, snapshot: MemorySnapshot, messages: &[Message]) {
        let Some(ui) = self.ui_weak.upgrade() else {
            return;
        };
        let Some(player) = snapshot.player else {
            return;
        };

        let chat = ui.global::<ChatGlobals>();
        chat.set_adventure(viewmodels::adventure_view(
            snapshot.current_adventure.as_ref(),
            &self.gallery,
        ));
        chat.set_act(viewmodels::act_view(snapshot.current_act.as_ref()));
        chat.set_state(viewmodels::state_view(snapshot.current_state.as_ref()));
        chat.set_facts(viewmodels::facts_model(&snapshot.current_facts));
        chat.set_messages(viewmodels::messages_model(messages, Some(player.id)));
    }

    fn reveal(&self) {
        if let Some(ui) = self.ui_weak.upgrade() {
            ui.set_current_page("chat".into());
        }
    }

    /// The player's line is echoed right away and the reply is posted when the model
    /// answers, so the input is never blocked on the turn.
    fn send(&self, text: &str) {
        let Some(adventure) = self.adventures.current_adventure() else {
            notice::error(&self.ui_weak, "a message was sent with no adventure open");
            return;
        };
        let Some(player) = self.adventures.current_snapshot().player else {
            return;
        };

        let player_id = player.id;
        let ooc = player.status != CharacterStatus::Present;
        let message = Message::new(adventure.id, Some(player_id), None, player.name, text, ooc);

        self.echo(&message, player_id);

        let messages = self.messages.clone();
        let ui_weak = self.ui_weak.clone();
        self.handle.spawn(async move {
            let reply = messages.send_message(message).await;
            post_reply(&ui_weak, reply, player_id);
        });
    }

    fn echo(&self, message: &Message, player_id: Uuid) {
        let Some(ui) = self.ui_weak.upgrade() else {
            return;
        };

        push_message(&ui, viewmodels::message_view(message, Some(player_id)));
        ui.global::<ChatGlobals>().set_awaiting_reply(true);
    }
}

pub fn register_chat_page(ui: &RootWindow, ctx: &DefaultContext) {
    let env = ChatPageContext {
        handle: Handle::current(),
        adventures: ctx.adventures.clone(),
        messages: ctx.messages.clone(),
        gallery: ctx.characters.gallery().clone(),
        ui_weak: ui.as_weak(),
    };

    register_open(ui, &env);
    register_message_sent(ui, &env);
}

/// Main page -> chat: open an existing adventure, render its snapshot and switch pages.
fn register_open(ui: &RootWindow, env: &ChatPageContext) {
    let env = env.clone();
    ui.global::<MainGlobals>().on_adventure_selected(move |id| {
        let Some(adventure_id) = parse_id(id.as_str()) else {
            return;
        };

        if let Err(err) = env.open(adventure_id) {
            notice::error(
                &env.ui_weak,
                format!("could not open the adventure {adventure_id}: {err}"),
            );
        }
    });
}

/// Chat page: the player's turn, followed by the reply the assistant produces.
fn register_message_sent(ui: &RootWindow, env: &ChatPageContext) {
    let env = env.clone();
    ui.global::<ChatGlobals>()
        .on_message_sent(move |text| env.send(text.as_str()));
}

fn post_reply(
    ui_weak: &slint::Weak<RootWindow>,
    reply: Result<Message, MessageManagerError>,
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
            Err(err) => notice::error(
                &ui.as_weak(),
                format!("the reply to {player_id} failed: {err}"),
            ),
        }
    });

    if let Err(err) = post {
        log::error!("could not post the reply of {player_id} to the UI: {err}");
    }
}

fn push_message(ui: &RootWindow, view: MessageView) {
    use slint::{Model, ModelRc, VecModel};

    let chat = ui.global::<ChatGlobals>();
    let model = chat.get_messages();
    if let Some(vec_model) = model.as_any().downcast_ref::<VecModel<MessageView>>() {
        vec_model.push(view);
    } else {
        let mut items: Vec<MessageView> = model.iter().collect();
        items.push(view);
        chat.set_messages(ModelRc::new(VecModel::from(items)));
    }
}
