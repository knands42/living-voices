mod cast;
mod composer;
mod header;
mod look;
mod meta;
mod top;
mod transcript;

use iced::widget::{column, container, row};
use iced::{Element, Length, Task};
use uuid::Uuid;

use crate::backend::application::adventure_manager::{
    AdventureManager, AdventureManagerError, MemorySnapshot,
};
use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::memory::{Act, Fact, State as SceneState};
use crate::backend::domain::message::Message as MessageLine;
use crate::ui::pages::chat::cast::cast_rail;
use crate::ui::pages::chat::header::header;
use crate::ui::pages::chat::meta::meta_rail;
use crate::ui::pages::chat::transcript::transcript;
use crate::ui::pages::{Event, Page};
use crate::ui::task::on_tokio;
use crate::ui::theme;

const HISTORY_LIMIT: usize = 100;

#[derive(Debug, Clone)]
pub enum Message {
    BackRequested,
    InputMessageChanged(String),
    Send,
    Opened(Result<Opened, String>),
}

#[derive(Debug, Clone)]
pub struct Opened {
    snapshot: MemorySnapshot,
    messages: Vec<MessageLine>,
}

pub struct State {
    handle: tokio::runtime::Handle,
    pub adventure_manager: AdventureManager,
    pub adventure: Option<Adventure>,
    pub act: Option<Act>,
    pub scene_state: Option<SceneState>,
    pub facts: Vec<Fact>,
    pub messages: Vec<MessageLine>,

    pub input_message: String,
    pub awaiting_reply: bool,
}

impl State {
    pub fn boot(
        adventure_manager: AdventureManager,
        handle: tokio::runtime::Handle,
    ) -> (Self, Task<Message>) {
        (
            Self {
                handle,
                adventure_manager,
                adventure: None,
                act: None,
                scene_state: None,
                facts: Vec::new(),
                messages: Vec::new(),
                input_message: String::new(),
                awaiting_reply: false,
            },
            Task::none(),
        )
    }

    pub fn rebind(&mut self, adventure_manager: AdventureManager) {
        self.adventure_manager = adventure_manager;
    }

    pub fn open(&mut self, adventure_id: Uuid) -> Task<Message> {
        self.clear();

        let adventure_manager = self.adventure_manager.clone();

        Task::perform(
            on_tokio(self.handle.clone(), async move {
                adventure_manager.start_adventure(adventure_id).await?;
                let snapshot = adventure_manager.current_snapshot();
                let messages = adventure_manager
                    .get_messages(adventure_id, HISTORY_LIMIT)
                    .await?;

                Ok::<_, AdventureManagerError>(Opened { snapshot, messages })
            }),
            |result| {
                Message::Opened(match result {
                    Some(Ok(opened)) => Ok(opened),
                    Some(Err(err)) => Err(err.to_string()),
                    None => Err("opening the adventure failed".to_string()),
                })
            },
        )
    }

    fn clear(&mut self) {
        self.adventure = None;
        self.act = None;
        self.scene_state = None;
        self.facts.clear();
        self.messages.clear();
        self.input_message.clear();
        self.awaiting_reply = false;
    }

    pub fn update(&mut self, message: Message) -> (Task<Message>, Event) {
        match message {
            Message::BackRequested => return (Task::none(), Event::Show(Page::MainHub)),
            Message::InputMessageChanged(value) => self.input_message = value,
            Message::Send => {}
            Message::Opened(Ok(opened)) => {
                self.adventure = opened.snapshot.current_adventure;
                self.act = opened.snapshot.current_act;
                self.scene_state = opened.snapshot.current_state;
                self.facts = opened.snapshot.current_facts;
                self.messages = opened.messages;
            }
            Message::Opened(Err(err)) => log::error!("could not open adventure: {err}"),
        }

        (Task::none(), Event::None)
    }

    pub fn view(&self) -> Element<'_, Message> {
        container(
            column![
                header(self),
                row![cast_rail(self), transcript(self), meta_rail(self)]
                    .spacing(14.0)
                    .width(Length::Fill)
                    .height(Length::Fill),
            ]
            .spacing(14.0)
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .padding([18.0, 22.0])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::page)
        .into()
    }
}
