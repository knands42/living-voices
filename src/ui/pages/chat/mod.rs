mod cast;
mod composer;
mod header;
mod look;
mod markup;
mod meta;
mod top;
mod transcript;

use iced::widget::{button, column, container, row, text};
use iced::{Element, Length, Task};
use uuid::Uuid;

use crate::backend::application::adventure_manager::{
    AdventureManager, AdventureManagerError, MemorySnapshot,
};
use crate::backend::application::managers::message::message_manager::MessageManager;
use crate::backend::application::port::chat_port::{ChatPort, NarratorPort};
use crate::backend::application::port::memory_port::{ActCloserPort, MemoryDistillerPort};
use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::memory::{Act, Fact, State as SceneState};
use crate::backend::domain::message::Message as MessageLine;
use crate::ui::common;
use crate::ui::pages::chat::header::header;
use crate::ui::pages::chat::meta::meta_rail;
use crate::ui::pages::chat::transcript::transcript;
use crate::ui::pages::{Event, Page};
use crate::ui::task::on_tokio;
use crate::ui::{layout, theme};

const HISTORY_LIMIT: usize = 100;

#[derive(Debug, Clone)]
pub enum Message {
    BackRequested,
    InputMessageChanged(String),
    Send,
    Cast(cast::Message),
    Opened(Result<Opened, String>),
    Replied {
        adventure_id: Uuid,
        sent: Result<(), String>,
        messages: Vec<MessageLine>,
        snapshot: MemorySnapshot,
    },
}

#[derive(Debug, Clone)]
pub struct Opened {
    snapshot: MemorySnapshot,
    messages: Vec<MessageLine>,
}

impl Opened {
    fn into_scene(self) -> Option<OngoingAdventure> {
        Some(OngoingAdventure::new(
            self.snapshot.current_adventure?,
            self.snapshot.current_act?,
            self.snapshot.current_state?,
            self.snapshot.current_facts,
            self.messages,
        ))
    }
}

pub struct OngoingAdventure {
    pub adventure: Adventure,
    pub act: Act,
    pub scene_state: SceneState,
    pub facts: Vec<Fact>,
    pub messages: Vec<MessageLine>,
    pub input_message: String,
    pub awaiting_reply: bool,
}

impl OngoingAdventure {
    fn new(
        adventure: Adventure,
        act: Act,
        scene_state: SceneState,
        facts: Vec<Fact>,
        messages: Vec<MessageLine>,
    ) -> Self {
        Self {
            adventure,
            act,
            scene_state,
            facts,
            messages,
            input_message: String::new(),
            awaiting_reply: false,
        }
    }

    fn adventure_id(&self) -> Uuid {
        self.adventure.id
    }

    fn take_outgoing(&mut self) -> Option<MessageLine> {
        let text = self.input_message.trim().to_string();
        if text.is_empty() || self.awaiting_reply {
            return None;
        }

        let player = self
            .adventure
            .characters
            .iter()
            .find(|cast| cast.is_player)?;
        let line = MessageLine::new(self.adventure.id, Some(player.id), None, text, false);

        self.messages.push(line.clone());
        self.input_message.clear();
        self.awaiting_reply = true;

        Some(line)
    }

    fn apply_snapshot(&mut self, snapshot: MemorySnapshot) {
        if let Some(adventure) = snapshot.current_adventure {
            self.adventure = adventure;
        }
        if let Some(act) = snapshot.current_act {
            self.act = act;
        }
        if let Some(scene_state) = snapshot.current_state {
            self.scene_state = scene_state;
        }
        self.facts = snapshot.current_facts;
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
    cast: cast::State,
    scene: Option<OngoingAdventure>,
    window_width: f32,
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
                cast: cast::State::default(),
                scene: None,
                window_width: layout::DEFAULT_WINDOW.width,
            },
            Task::none(),
        )
    }

    pub fn set_window_width(&mut self, window_width: f32) {
        self.window_width = window_width;
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
        self.scene = None;
        self.cast = cast::State::default();

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
        let Some(line) = self
            .scene
            .as_mut()
            .and_then(OngoingAdventure::take_outgoing)
        else {
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
                let snapshot = adventure_manager.current_snapshot();

                (sent, messages, snapshot)
            }),
            move |outcome| {
                let (sent, messages, snapshot) = outcome.unwrap_or_else(|| {
                    (
                        Err("the turn task failed".to_string()),
                        Vec::new(),
                        MemorySnapshot::default(),
                    )
                });

                Message::Replied {
                    adventure_id,
                    sent,
                    messages,
                    snapshot,
                }
            },
        )
    }

    fn perform(&self, effect: cast::Effect) -> Task<Message> {
        match effect {
            cast::Effect::LoadGallery(character_id) => {
                log::debug!("cast gallery requested for {character_id}");
                Task::none()
            }
        }
    }

    pub fn update(&mut self, message: Message) -> (Task<Message>, Event) {
        match message {
            Message::BackRequested => return (Task::none(), Event::Show(Page::MainHub)),
            Message::InputMessageChanged(value) => {
                if let Some(scene) = self.scene.as_mut() {
                    scene.input_message = value;
                }
            }
            Message::Send => return (self.send(), Event::None),
            Message::Cast(message) => {
                if let Some(effect) = self.cast.update(message) {
                    return (self.perform(effect), Event::None);
                }
            }
            Message::Replied {
                adventure_id,
                sent,
                messages,
                snapshot,
            } => {
                let Some(scene) = self.scene.as_mut() else {
                    return (Task::none(), Event::None);
                };
                if scene.adventure_id() != adventure_id {
                    return (Task::none(), Event::None);
                }
                if let Err(err) = sent {
                    log::error!("could not complete the turn: {err}");
                }
                scene.messages = messages;
                scene.awaiting_reply = false;
                scene.apply_snapshot(snapshot);
            }
            Message::Opened(Ok(opened)) => match opened.into_scene() {
                Some(scene) => {
                    let companion = {
                        let mut companions = scene
                            .adventure
                            .characters
                            .iter()
                            .filter(|character| !character.is_player);
                        match (companions.next(), companions.next()) {
                            (Some(companion), None) => Some(companion.id),
                            _ => None,
                        }
                    };

                    let task = match companion {
                        Some(character_id) => self
                            .cast
                            .update(cast::Message::Picked(character_id))
                            .map_or_else(Task::none, |effect| self.perform(effect)),
                        None => Task::none(),
                    };

                    self.scene = Some(scene);
                    return (task, Event::None);
                }
                None => log::error!("the adventure opened without an act or scene state"),
            },
            Message::Opened(Err(err)) => log::error!("could not open adventure: {err}"),
        }

        (Task::none(), Event::None)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let Some(scene) = self.scene.as_ref() else {
            return opening();
        };

        let mut columns = row![
            self.cast.view(&scene.adventure).map(Message::Cast),
            transcript(scene, layout::shows_act(self.window_width)),
        ]
        .spacing(14.0)
        .width(Length::Fill)
        .height(Length::Fill);

        if layout::shows_facts(self.window_width) {
            columns = columns.push(meta_rail(scene));
        }

        container(
            column![header(scene), columns]
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

fn opening() -> Element<'static, Message> {
    container(
        column![
            button(text("← Adventures Hub").size(11.5))
                .padding([7.0, 13.0])
                .style(theme::back_button)
                .on_press(Message::BackRequested),
            text("The story is opening.")
                .size(12.5)
                .color(theme::TEXT_MUTED)
                .font(common::italic()),
        ]
        .spacing(16.0),
    )
    .padding([18.0, 22.0])
    .width(Length::Fill)
    .height(Length::Fill)
    .style(theme::page)
    .into()
}
