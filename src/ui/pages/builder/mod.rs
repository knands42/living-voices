pub(crate) mod acts;
pub(crate) mod cast;
pub(crate) mod opening;
mod sidebar;
pub(crate) mod story;
mod widgets;

use chrono::Utc;
use iced::widget::{container, row, scrollable};
use iced::{Element, Length, Task};
use uuid::Uuid;

use crate::backend::application::adventure_manager::{AdventureManager, AdventureManagerError};
use crate::backend::application::character_manager::{CharacterManager, CharacterManagerError};
use crate::backend::application::port::chat_port::PromptPort;
use crate::backend::application::port::image_port::ImagePort;
use crate::backend::domain::adventure::{Adventure, AdventureStatus};
use crate::backend::domain::character::Character;
use crate::backend::domain::lora::{Lora, LoraKind};
use crate::backend::domain::memory::{Act, Fact, State as SceneState};
use crate::backend::domain::message::Message as MessageLine;
use crate::ui::pages::builder::sidebar::sidebar;
use crate::ui::pages::{Event, Page};
use crate::ui::task::on_tokio;
use crate::ui::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Story,
    Cast,
    Acts,
    Opening,
}

impl Section {
    pub const ALL: [Section; 4] = [
        Section::Story,
        Section::Cast,
        Section::Acts,
        Section::Opening,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Section::Story => "The Story",
            Section::Cast => "The Cast",
            Section::Acts => "Acts",
            Section::Opening => "The Opening",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    BackRequested,
    SectionPicked(Section),
    Opened(Result<Opened, String>),
    Story(story::Message),
    Cast(cast::Message),
    Acts(acts::Message),
    Opening(opening::Message),
    Publish,
    Published(Result<(), String>),
}

#[derive(Debug, Clone, Default)]
pub(crate) enum AdapterChoice {
    #[default]
    Untouched,
    Cleared,
    Picked(String),
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Adapters {
    pub text: AdapterChoice,
    pub image: AdapterChoice,
}

/// What a section asks the page to do, because only the page holds the managers.
pub(crate) enum Effect {
    AddCharacter,
    RemoveCharacter(Uuid),
    RemoveAct(Uuid),
    SaveStory,
    SaveCharacter { id: Uuid, adapters: Adapters },
    SaveAct,
    SaveOpening,
    SaveFacts,
}

#[derive(Debug, Clone)]
pub struct Opened {
    adventure: Adventure,
    acts: Vec<Act>,
    scene: Option<SceneState>,
    facts: Vec<Fact>,
    messages: Vec<MessageLine>,
}

/// The adventure as the page edits it: the persisted document plus the state
/// each section keeps while it is being edited.
pub struct Draft {
    adventure: Adventure,
    saved_cast: Vec<Character>,
    scene: SceneState,
    story: story::State,
    cast: cast::State,
    acts: acts::State,
    opening: opening::State,
}

impl Draft {
    fn new(opened: Opened) -> Self {
        let scene = opened
            .scene
            .unwrap_or_else(|| SceneState::new(Uuid::new_v4(), opened.adventure.id));
        let story = story::State::load(&opened.adventure.lore.text);
        let acts = acts::State::load(opened.acts);
        let opening = opening::State::load(&scene, opened.facts, opened.messages);
        let saved_cast = opened.adventure.characters.clone();

        Self {
            adventure: opened.adventure,
            saved_cast,
            scene,
            story,
            cast: cast::State::default(),
            acts,
            opening,
        }
    }

    fn section_is_done(&self, section: Section) -> bool {
        match section {
            Section::Story => self.story.is_ready(&self.adventure),
            Section::Cast => self.cast.is_ready(&self.adventure.characters),
            Section::Acts => self.acts.is_ready(),
            Section::Opening => self.opening.is_ready(),
        }
    }

    fn ready_count(&self) -> usize {
        Section::ALL
            .iter()
            .filter(|section| self.section_is_done(**section))
            .count()
    }

    fn is_publishable(&self) -> bool {
        self.ready_count() == Section::ALL.len()
    }
}

pub struct State<P: PromptPort + Clone + 'static, I: ImagePort + Clone + 'static> {
    handle: tokio::runtime::Handle,
    adventure_manager: AdventureManager,
    character_manager: CharacterManager<P, I>,
    draft: Option<Draft>,
    section: Section,
}

impl<P: PromptPort + Clone + 'static, I: ImagePort + Clone + 'static> State<P, I> {
    pub fn boot(
        adventure_manager: AdventureManager,
        character_manager: CharacterManager<P, I>,
        handle: tokio::runtime::Handle,
    ) -> (Self, Task<Message>) {
        (
            Self {
                handle,
                adventure_manager,
                character_manager,
                draft: None,
                section: Section::Story,
            },
            Task::none(),
        )
    }

    pub fn open(&mut self, adventure_id: Uuid) -> Task<Message> {
        self.draft = None;
        self.section = Section::Story;

        let adventure_manager = self.adventure_manager.clone();

        Task::perform(
            on_tokio(self.handle.clone(), async move {
                let adventure = adventure_manager.get_adventure(adventure_id).await?;
                let acts = adventure_manager.get_acts(adventure_id).await?;
                let scene = adventure_manager.get_state(adventure_id).await?;
                let facts = adventure_manager.get_facts(adventure_id).await?;
                let messages = adventure_manager
                    .get_messages(adventure_id, opening::State::MAX_BEATS)
                    .await?;

                Ok::<_, AdventureManagerError>(Opened {
                    adventure,
                    acts,
                    scene,
                    facts,
                    messages,
                })
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

    pub fn update(&mut self, message: Message) -> (Task<Message>, Event) {
        match message {
            Message::BackRequested => return (Task::none(), Event::Show(Page::MainHub)),
            Message::SectionPicked(section) => self.section = section,
            Message::Opened(Ok(opened)) => self.draft = Some(Draft::new(opened)),
            Message::Opened(Err(err)) => log::error!("could not open the adventure: {err}"),
            Message::Story(message) => {
                if let Some(effect) =
                    self.edit(|draft| draft.story.update(message, &mut draft.adventure))
                {
                    return (self.perform(effect), Event::None);
                }
            }
            Message::Cast(message) => {
                if let cast::Message::Saved(Ok(character)) = &message
                    && let Some(draft) = &mut self.draft
                {
                    update_cast(&mut draft.saved_cast, character.clone());
                }

                if let Some(effect) =
                    self.edit(|draft| draft.cast.update(message, &mut draft.adventure.characters))
                {
                    return (self.perform(effect), Event::None);
                }
            }
            Message::Acts(message) => {
                if let Some(effect) =
                    self.edit(|draft| draft.acts.update(message, draft.adventure.id))
                {
                    return (self.perform(effect), Event::None);
                }
            }
            Message::Opening(message) => {
                if let Some(effect) = self.edit(|draft| draft.opening.update(message)) {
                    return (self.perform(effect), Event::None);
                }
            }
            Message::Publish => return (self.publish(), Event::None),
            Message::Published(Ok(())) => {
                if let Some(draft) = &mut self.draft {
                    draft.adventure.status = AdventureStatus::Public;
                }
                return (Task::none(), Event::Show(Page::MainHub));
            }
            Message::Published(Err(err)) => log::error!("could not publish the adventure: {err}"),
        }

        (Task::none(), Event::None)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let body: Element<'_, Message> = match &self.draft {
            Some(draft) => match self.section {
                Section::Story => draft.story.view(&draft.adventure).map(Message::Story),
                Section::Cast => draft
                    .cast
                    .view(&draft.adventure.characters)
                    .map(Message::Cast),
                Section::Acts => draft.acts.view().map(Message::Acts),
                Section::Opening => draft
                    .opening
                    .view(&draft.adventure.characters)
                    .map(Message::Opening),
            },
            None => widgets::empty_state("This adventure is not open yet."),
        };

        let main = scrollable(container(body).padding(28.0).width(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill);

        container(row![sidebar(self), main])
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::page)
            .into()
    }

    fn edit(&mut self, change: impl FnOnce(&mut Draft) -> Option<Effect>) -> Option<Effect> {
        self.draft.as_mut().and_then(change)
    }

    fn perform(&self, effect: Effect) -> Task<Message> {
        match effect {
            Effect::AddCharacter => self.add_character(),
            Effect::RemoveCharacter(character_id) => self.remove_character(character_id),
            Effect::RemoveAct(act_id) => self.remove_act(act_id),
            Effect::SaveStory => self.save_story(),
            Effect::SaveCharacter { id, adapters } => self.save_character(id, adapters),
            Effect::SaveAct => self.save_act(),
            Effect::SaveOpening => self.save_opening(),
            Effect::SaveFacts => self.save_facts(),
        }
    }

    fn save_story(&self) -> Task<Message> {
        let Some(draft) = &self.draft else {
            return Task::none();
        };

        let mut adventure = draft.adventure.clone();
        adventure.lore.text = draft.story.lore_text();
        let adventure_manager = self.adventure_manager.clone();

        Task::perform(
            on_tokio(self.handle.clone(), async move {
                adventure_manager.update_adventure(&adventure).await
            }),
            |result| {
                Message::Story(story::Message::Saved(resolved(
                    result,
                    "saving the story failed",
                )))
            },
        )
    }

    fn add_character(&self) -> Task<Message> {
        let Some(draft) = &self.draft else {
            return Task::none();
        };

        let adventure_id = draft.adventure.id;
        let character_manager = self.character_manager.clone();

        Task::perform(
            on_tokio(self.handle.clone(), async move {
                character_manager.create_draft(adventure_id).await
            }),
            |result| {
                Message::Cast(cast::Message::Added(resolved(
                    result,
                    "adding the character failed",
                )))
            },
        )
    }

    fn remove_character(&self, character_id: Uuid) -> Task<Message> {
        let character = self
            .draft
            .as_ref()
            .and_then(|draft| {
                draft
                    .adventure
                    .characters
                    .iter()
                    .find(|character| character.id == character_id)
            })
            .cloned();

        let Some(character) = character else {
            return Task::none();
        };

        let character_manager = self.character_manager.clone();

        Task::perform(
            on_tokio(self.handle.clone(), async move {
                character_manager.delete_character(character).await
            }),
            move |result| {
                Message::Cast(cast::Message::Removed(
                    character_id,
                    resolved(result, "removing the character failed"),
                ))
            },
        )
    }

    fn save_character(&self, character_id: Uuid, adapters: Adapters) -> Task<Message> {
        let Some(draft) = &self.draft else {
            return Task::none();
        };

        let Some(character) = draft
            .adventure
            .characters
            .iter()
            .find(|character| character.id == character_id)
            .cloned()
        else {
            return Task::none();
        };

        let previous = draft
            .saved_cast
            .iter()
            .find(|saved| saved.id == character_id)
            .cloned();
        let character_manager = self.character_manager.clone();

        Task::perform(
            on_tokio(self.handle.clone(), async move {
                let body = needs_body_prompt(previous.as_ref(), &character);
                let outfit = needs_outfit_prompt(previous.as_ref(), &character);
                let mut character = character;

                for (kind, choice) in [
                    (LoraKind::Text, adapters.text),
                    (LoraKind::Image, adapters.image),
                ] {
                    match choice {
                        AdapterChoice::Untouched => {}
                        AdapterChoice::Cleared => character.loras.retain(|lora| lora.kind != kind),
                        AdapterChoice::Picked(source) => {
                            let scale = character
                                .loras
                                .iter()
                                .find(|lora| lora.kind == kind)
                                .map_or(Lora::DEFAULT_SCALE, |lora| lora.scale);

                            if let Some(lora) = character_manager
                                .stage_lora_adapter(&source, scale, kind)
                                .await
                            {
                                character.loras.retain(|lora| lora.kind != kind);
                                character.loras.push(lora);
                            }
                        }
                    }
                }

                character_manager
                    .update_character(character.clone())
                    .await?;

                if body {
                    character = character_manager
                        .create_character_body_prompt(&character)
                        .await?;
                }

                if outfit {
                    character = character_manager
                        .create_character_outfit_prompt(&character)
                        .await?;
                }

                Ok::<_, CharacterManagerError>(character)
            }),
            |result| {
                Message::Cast(cast::Message::Saved(resolved(
                    result,
                    "saving the character failed",
                )))
            },
        )
    }

    fn save_act(&self) -> Task<Message> {
        let act = self
            .draft
            .as_ref()
            .and_then(|draft| draft.acts.open())
            .cloned();

        let Some(act) = act else {
            return Task::none();
        };

        let adventure_manager = self.adventure_manager.clone();

        Task::perform(
            on_tokio(self.handle.clone(), async move {
                adventure_manager.upsert_act(&act).await?;
                Ok::<_, AdventureManagerError>(act)
            }),
            |result| {
                Message::Acts(acts::Message::Saved(resolved(
                    result,
                    "saving the act failed",
                )))
            },
        )
    }

    fn remove_act(&self, act_id: Uuid) -> Task<Message> {
        let adventure_manager = self.adventure_manager.clone();
        Task::perform(
            on_tokio(self.handle.clone(), async move {
                adventure_manager.delete_act(act_id).await?;
                Ok::<_, AdventureManagerError>(())
            }),
            |result| {
                Message::Acts(acts::Message::Removed(resolved(
                    result,
                    "removing the act failed",
                )))
            },
        )
    }

    fn save_facts(&self) -> Task<Message> {
        let Some(draft) = &self.draft else {
            return Task::none();
        };

        let adventure_id = draft.adventure.id;
        let facts = draft.opening.facts_snapshot();
        let adventure_manager = self.adventure_manager.clone();

        Task::perform(
            on_tokio(self.handle.clone(), async move {
                adventure_manager.upsert_facts(adventure_id, &facts).await?;
                Ok::<_, AdventureManagerError>(())
            }),
            |result| {
                Message::Opening(opening::Message::FactsSaved(resolved(
                    result,
                    "saving the facts failed",
                )))
            },
        )
    }

    fn save_opening(&self) -> Task<Message> {
        let Some(draft) = &self.draft else {
            return Task::none();
        };

        let mut scene = draft.scene.clone();
        scene.place = draft.opening.stored_place();
        scene.day = draft.opening.stored_day();
        scene.updated_at = Utc::now();

        let adventure_id = draft.adventure.id;
        let facts = draft.opening.facts_snapshot();
        let messages = draft.opening.outgoing_messages(&draft.adventure);
        let adventure_manager = self.adventure_manager.clone();

        Task::perform(
            on_tokio(self.handle.clone(), async move {
                adventure_manager.upsert_state(&scene).await?;
                adventure_manager.upsert_facts(adventure_id, &facts).await?;
                adventure_manager
                    .save_opening(adventure_id, &messages)
                    .await?;
                Ok::<_, AdventureManagerError>(())
            }),
            |result| {
                Message::Opening(opening::Message::Saved(resolved(
                    result,
                    "saving the opening failed",
                )))
            },
        )
    }

    fn publish(&self) -> Task<Message> {
        let Some(draft) = &self.draft else {
            return Task::none();
        };

        let adventure_id = draft.adventure.id;
        let adventure_manager = self.adventure_manager.clone();

        Task::perform(
            on_tokio(self.handle.clone(), async move {
                adventure_manager
                    .update_status(adventure_id, AdventureStatus::Public)
                    .await
            }),
            |result| Message::Published(resolved(result, "publishing the adventure failed")),
        )
    }
}

fn resolved<T, E: ToString>(
    result: Option<Result<T, E>>,
    failure: &'static str,
) -> Result<T, String> {
    match result {
        Some(Ok(value)) => Ok(value),
        Some(Err(err)) => Err(err.to_string()),
        None => Err(failure.to_string()),
    }
}

fn update_cast(cast: &mut Vec<Character>, character: Character) {
    match cast.iter_mut().find(|saved| saved.id == character.id) {
        Some(saved) => *saved = character,
        None => cast.push(character),
    }
}

fn needs_body_prompt(previous: Option<&Character>, character: &Character) -> bool {
    let filled = character
        .appearance
        .as_deref()
        .is_some_and(|appearance| !appearance.trim().is_empty());

    filled
        && previous
            .is_none_or(|previous| body_prompt_inputs(previous) != body_prompt_inputs(character))
}

fn body_prompt_inputs(character: &Character) -> (&str, &str, Option<&str>) {
    (
        character.name.as_str(),
        character.genre.trim(),
        character.appearance.as_deref(),
    )
}

fn needs_outfit_prompt(previous: Option<&Character>, character: &Character) -> bool {
    let filled = !character.appearance_current_outfit.trim().is_empty();

    filled
        && previous.is_none_or(|previous| {
            outfit_prompt_inputs(previous) != outfit_prompt_inputs(character)
        })
}

fn outfit_prompt_inputs(character: &Character) -> (&str, &str, &str) {
    (
        character.name.as_str(),
        character.genre.trim(),
        character.appearance_current_outfit.trim(),
    )
}
