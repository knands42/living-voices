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
use crate::backend::application::message_manager::MessageManager;
use crate::backend::application::port::chat_port::{ChatPort, NarratorPort};
use crate::backend::application::port::memory_port::{ActCloserPort, MemoryDistillerPort};
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
    Replied {
        adventure_id: Uuid,
        sent: Result<(), String>,
        messages: Vec<MessageLine>,
    },
}

#[derive(Debug, Clone)]
pub struct Opened {
    snapshot: MemorySnapshot,
    messages: Vec<MessageLine>,
}

#[derive(Default)]
pub struct Scene {
    pub adventure: Option<Adventure>,
    pub act: Option<Act>,
    pub scene_state: Option<SceneState>,
    pub facts: Vec<Fact>,
    pub messages: Vec<MessageLine>,
    pub input_message: String,
    pub awaiting_reply: bool,
}

impl Scene {
    fn adventure_id(&self) -> Option<Uuid> {
        self.adventure.as_ref().map(|adventure| adventure.id)
    }

    fn open(&mut self, opened: Opened) {
        self.adventure = opened.snapshot.current_adventure;
        self.act = opened.snapshot.current_act;
        self.scene_state = opened.snapshot.current_state;
        self.facts = opened.snapshot.current_facts;
        self.messages = opened.messages;
    }

    fn take_outgoing(&mut self) -> Option<MessageLine> {
        let text = self.input_message.trim().to_string();
        if text.is_empty() || self.awaiting_reply {
            return None;
        }

        let adventure = self.adventure.as_ref()?;
        let player = adventure.characters.iter().find(|cast| cast.is_player)?;
        let line = MessageLine::new(
            adventure.id,
            Some(player.id),
            None,
            player.name.clone(),
            text,
            false,
        );

        self.messages.push(line.clone());
        self.input_message.clear();
        self.awaiting_reply = true;

        Some(line)
    }
}

pub struct State<
    C: ChatPort + Clone + 'static,
    N: NarratorPort + Clone + 'static,
    M: MemoryDistillerPort + Clone + 'static,
    A: ActCloserPort + Clone + 'static,
> {
    handle: tokio::runtime::Handle,
    adventure_manager: AdventureManager,
    message_manager: MessageManager<C, N, M, A>,
    scene: Scene,
}

impl<
    C: ChatPort + Clone + 'static,
    N: NarratorPort + Clone + 'static,
    M: MemoryDistillerPort + Clone + 'static,
    A: ActCloserPort + Clone + 'static,
> State<C, N, M, A>
{
    pub fn boot(
        adventure_manager: AdventureManager,
        message_manager: MessageManager<C, N, M, A>,
        handle: tokio::runtime::Handle,
    ) -> (Self, Task<Message>) {
        (
            Self {
                handle,
                adventure_manager,
                message_manager,
                scene: Scene::default(),
            },
            Task::none(),
        )
    }

    pub fn rebind(
        &mut self,
        adventure_manager: AdventureManager,
        message_manager: MessageManager<C, N, M, A>,
    ) {
        self.adventure_manager = adventure_manager;
        self.message_manager = message_manager;
    }

    pub fn open(&mut self, adventure_id: Uuid) -> Task<Message> {
        self.scene = Scene::default();

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

    fn send(&mut self) -> Task<Message> {
        let Some(line) = self.scene.take_outgoing() else {
            return Task::none();
        };

        let adventure_id = line.adventure_id;
        let message_manager = self.message_manager.clone();
        let adventure_manager = self.adventure_manager.clone();

        Task::perform(
            on_tokio(self.handle.clone(), async move {
                let sent = message_manager
                    .send_message(line)
                    .await
                    .map(|_| ())
                    .map_err(|err| err.to_string());
                let messages = adventure_manager
                    .get_messages(adventure_id, HISTORY_LIMIT)
                    .await
                    .unwrap_or_default();

                (sent, messages)
            }),
            move |outcome| {
                let (sent, messages) = outcome
                    .unwrap_or_else(|| (Err("the turn task failed".to_string()), Vec::new()));

                Message::Replied {
                    adventure_id,
                    sent,
                    messages,
                }
            },
        )
    }

    pub fn update(&mut self, message: Message) -> (Task<Message>, Event) {
        match message {
            Message::BackRequested => return (Task::none(), Event::Show(Page::MainHub)),
            Message::InputMessageChanged(value) => self.scene.input_message = value,
            Message::Send => return (self.send(), Event::None),
            Message::Replied {
                adventure_id,
                sent,
                messages,
            } => {
                if self.scene.adventure_id() != Some(adventure_id) {
                    return (Task::none(), Event::None);
                }
                if let Err(err) = sent {
                    log::error!("could not complete the turn: {err}");
                }
                self.scene.messages = messages;
                self.scene.awaiting_reply = false;
            }
            Message::Opened(Ok(opened)) => self.scene.open(opened),
            Message::Opened(Err(err)) => log::error!("could not open adventure: {err}"),
        }

        (Task::none(), Event::None)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let scene = &self.scene;

        container(
            column![
                header(scene),
                row![cast_rail(scene), transcript(scene), meta_rail(scene)]
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
