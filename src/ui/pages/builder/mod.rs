mod acts;
mod cast;
mod opening;
mod sidebar;
mod story;
mod widgets;

use iced::widget::{container, row, scrollable, text_editor};
use iced::{Element, Length, Task};
use uuid::Uuid;

use crate::backend::application::adventure_manager::{AdventureManager, AdventureManagerError};
use crate::backend::application::character_manager::CharacterManager;
use crate::backend::application::port::chat_port::PromptPort;
use crate::backend::application::port::image_port::ImagePort;
use crate::backend::domain::adventure::{Adventure, AdventureStatus};
use crate::backend::domain::character::Character;
use crate::backend::domain::memory::Act;
use crate::ui::pages::builder::acts::acts;
use crate::ui::pages::builder::cast::cast;
use crate::ui::pages::builder::opening::opening;
use crate::ui::pages::builder::sidebar::sidebar;
use crate::ui::pages::builder::story::story;
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
    TitleChanged(String),
    HeadlineChanged(String),
    NarratorChanged(String),
    ToneChanged(String),
    LoreEdited(text_editor::Action),
    SaveStory,
    StorySaved(Result<Adventure, String>),
    AddCharacter,
    CharacterAdded(Result<Character, String>),
    CharacterNameChanged(Uuid, String),
    SaveCast,
    CastSaved(Result<(), String>),
    AddAct,
    ActPicked(usize),
    ActGoalChanged(String),
    ActTriggerChanged(String),
    SaveAct,
    ActSaved(Result<Act, String>),
    OpeningEdited(text_editor::Action),
    SaveOpening,
    OpeningSaved(Result<(), String>),
    Publish,
    Published(Result<(), String>),
}

#[derive(Debug, Clone)]
pub struct Opened {
    adventure: Adventure,
    acts: Vec<Act>,
    opening: Option<String>,
}

/// The adventure as the page edits it: the persisted copy plus the edits that
/// have not been written yet.
pub struct Draft {
    adventure: Adventure,
    acts: Vec<Act>,
    lore: text_editor::Content,
    opening: text_editor::Content,
    opening_saved: bool,
    story_changed: bool,
    act_index: Option<usize>,
}

impl Draft {
    fn new(opened: Opened) -> Self {
        let lore = text_editor::Content::with_text(&opened.adventure.lore.text);
        let opening = match &opened.opening {
            Some(text) => text_editor::Content::with_text(text),
            None => text_editor::Content::new(),
        };

        Self {
            adventure: opened.adventure,
            acts: opened.acts,
            lore,
            opening,
            opening_saved: opened.opening.is_some(),
            story_changed: false,
            act_index: None,
        }
    }

    fn edit_story(&mut self, change: impl FnOnce(&mut Adventure)) {
        change(&mut self.adventure);
        self.story_changed = true;
    }

    fn story_is_dirty(&self) -> bool {
        self.story_changed || self.lore.text() != self.adventure.lore.text
    }

    fn story_can_save(&self) -> bool {
        self.story_is_dirty() && !self.adventure.title.trim().is_empty()
    }

    fn story_is_ready(&self) -> bool {
        let adventure = &self.adventure;

        !adventure.title.trim().is_empty()
            && !adventure.headline.trim().is_empty()
            && !adventure.tone.trim().is_empty()
            && !adventure.narrator.name.trim().is_empty()
            && !adventure.lore.text.trim().is_empty()
    }

    fn cast_is_ready(&self) -> bool {
        self.adventure
            .characters
            .iter()
            .any(|character| !character.is_player && !character.is_placeholder())
    }

    fn cast_can_save(&self) -> bool {
        self.adventure
            .characters
            .iter()
            .any(|character| !character.is_player)
    }

    fn name_character(&mut self, character_id: Uuid, name: String) {
        if let Some(character) = self
            .adventure
            .characters
            .iter_mut()
            .find(|character| character.id == character_id)
        {
            character.name = name;
        }
    }

    fn acts_are_ready(&self) -> bool {
        !self.acts.is_empty() && self.acts.iter().all(Act::is_valid_act)
    }

    fn opening_can_save(&self) -> bool {
        !self.opening.text().trim().is_empty()
    }

    fn section_is_done(&self, section: Section) -> bool {
        match section {
            Section::Story => self.story_is_ready(),
            Section::Cast => self.cast_is_ready(),
            Section::Acts => self.acts_are_ready(),
            Section::Opening => self.opening_saved,
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

    fn open_act(&self) -> Option<&Act> {
        self.act_index.and_then(|index| self.acts.get(index))
    }

    fn add_act(&mut self) {
        let ordinal = self.acts.len() as u16;
        self.acts.push(Act::new(self.adventure.id, ordinal, ""));
        self.act_index = Some(self.acts.len() - 1);
    }

    fn edit_open_act(&mut self, change: impl FnOnce(&mut Act)) {
        if let Some(index) = self.act_index
            && let Some(act) = self.acts.get_mut(index)
        {
            change(act);
        }
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

    pub fn rebind(
        &mut self,
        adventure_manager: AdventureManager,
        character_manager: CharacterManager<P, I>,
    ) {
        self.adventure_manager = adventure_manager;
        self.character_manager = character_manager;
    }

    pub fn open(&mut self, adventure_id: Uuid) -> Task<Message> {
        self.draft = None;
        self.section = Section::Story;

        let adventure_manager = self.adventure_manager.clone();

        Task::perform(
            on_tokio(self.handle.clone(), async move {
                let adventure = adventure_manager.get_adventure(adventure_id).await?;
                let acts = adventure_manager.get_acts(adventure_id).await?;
                let opening = adventure_manager.get_opening(adventure_id).await?;

                Ok::<_, AdventureManagerError>(Opened {
                    adventure,
                    acts,
                    opening: opening.map(|message| message.text),
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
            Message::TitleChanged(title) => self.edit_story(|adventure| adventure.title = title),
            Message::HeadlineChanged(headline) => {
                self.edit_story(|adventure| adventure.headline = headline)
            }
            Message::NarratorChanged(name) => {
                self.edit_story(|adventure| adventure.narrator.name = name)
            }
            Message::ToneChanged(tone) => self.edit_story(|adventure| adventure.tone = tone),
            Message::LoreEdited(action) => {
                if let Some(draft) = &mut self.draft {
                    draft.lore.perform(action);
                }
            }
            Message::SaveStory => return (self.save_story(), Event::None),
            Message::StorySaved(result) => self.story_saved(result),
            Message::AddCharacter => return (self.add_character(), Event::None),
            Message::CharacterAdded(Ok(character)) => {
                if let Some(draft) = &mut self.draft {
                    draft.adventure.characters.push(character);
                }
            }
            Message::CharacterAdded(Err(err)) => log::error!("could not add a character: {err}"),
            Message::CharacterNameChanged(character_id, name) => {
                if let Some(draft) = &mut self.draft {
                    draft.name_character(character_id, name);
                }
            }
            Message::SaveCast => return (self.save_cast(), Event::None),
            Message::CastSaved(result) => {
                if let Err(err) = result {
                    log::error!("could not save the cast: {err}");
                }
            }
            Message::AddAct => {
                if let Some(draft) = &mut self.draft {
                    draft.add_act();
                }
            }
            Message::ActPicked(index) => {
                if let Some(draft) = &mut self.draft {
                    draft.act_index = Some(index);
                }
            }
            Message::ActGoalChanged(goal) => {
                if let Some(draft) = &mut self.draft {
                    draft.edit_open_act(|act| act.goal = goal);
                }
            }
            Message::ActTriggerChanged(trigger) => {
                if let Some(draft) = &mut self.draft {
                    draft.edit_open_act(|act| {
                        act.trigger = (!trigger.trim().is_empty()).then_some(trigger);
                    });
                }
            }
            Message::SaveAct => return (self.save_act(), Event::None),
            Message::ActSaved(result) => self.act_saved(result),
            Message::OpeningEdited(action) => {
                if let Some(draft) = &mut self.draft {
                    draft.opening.perform(action);
                }
            }
            Message::SaveOpening => return (self.save_opening(), Event::None),
            Message::OpeningSaved(Ok(())) => {
                if let Some(draft) = &mut self.draft {
                    draft.opening_saved = true;
                }
            }
            Message::OpeningSaved(Err(err)) => log::error!("could not save the opening: {err}"),
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
                Section::Story => story(draft),
                Section::Cast => cast(draft),
                Section::Acts => acts(draft),
                Section::Opening => opening(draft),
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

    fn edit_story(&mut self, change: impl FnOnce(&mut Adventure)) {
        if let Some(draft) = &mut self.draft {
            draft.edit_story(change);
        }
    }

    fn save_story(&self) -> Task<Message> {
        let Some(draft) = &self.draft else {
            return Task::none();
        };

        let mut adventure = draft.adventure.clone();
        adventure.lore.text = draft.lore.text();
        let adventure_manager = self.adventure_manager.clone();

        Task::perform(
            on_tokio(self.handle.clone(), async move {
                adventure_manager
                    .update_adventure(&adventure)
                    .await
                    .map_err(|err| err.to_string())
            }),
            |result| Message::StorySaved(resolved(result, "saving the story failed")),
        )
    }

    fn story_saved(&mut self, result: Result<Adventure, String>) {
        let Some(draft) = &mut self.draft else {
            return;
        };

        match result {
            Ok(adventure) => {
                draft.adventure = adventure;
                draft.story_changed = false;
            }
            Err(err) => log::error!("could not save the story: {err}"),
        }
    }

    fn add_character(&self) -> Task<Message> {
        let Some(draft) = &self.draft else {
            return Task::none();
        };

        let adventure_id = draft.adventure.id;
        let character_manager = self.character_manager.clone();

        Task::perform(
            on_tokio(self.handle.clone(), async move {
                character_manager
                    .create_draft(adventure_id)
                    .await
                    .map_err(|err| err.to_string())
            }),
            |result| Message::CharacterAdded(resolved(result, "adding the character failed")),
        )
    }

    fn save_cast(&self) -> Task<Message> {
        let Some(draft) = &self.draft else {
            return Task::none();
        };

        let cast: Vec<Character> = draft
            .adventure
            .characters
            .iter()
            .filter(|character| !character.is_player)
            .cloned()
            .collect();
        let character_manager = self.character_manager.clone();

        Task::perform(
            on_tokio(self.handle.clone(), async move {
                for character in cast {
                    character_manager
                        .update_character(character)
                        .await
                        .map_err(|err| err.to_string())?;
                }
                Ok(())
            }),
            |result| Message::CastSaved(resolved(result, "saving the cast failed")),
        )
    }

    fn save_act(&self) -> Task<Message> {
        let Some(act) = self.draft.as_ref().and_then(Draft::open_act).cloned() else {
            return Task::none();
        };

        let adventure_manager = self.adventure_manager.clone();

        Task::perform(
            on_tokio(self.handle.clone(), async move {
                adventure_manager
                    .upsert_act(&act)
                    .await
                    .map_err(|err| err.to_string())?;
                Ok(act)
            }),
            |result| Message::ActSaved(resolved(result, "saving the act failed")),
        )
    }

    fn act_saved(&mut self, result: Result<Act, String>) {
        let Some(draft) = &mut self.draft else {
            return;
        };

        match result {
            Ok(act) => {
                if let Some(slot) = draft.acts.iter_mut().find(|saved| saved.id == act.id) {
                    *slot = act;
                }
                draft.act_index = None;
            }
            Err(err) => log::error!("could not save the act: {err}"),
        }
    }

    fn save_opening(&self) -> Task<Message> {
        let Some(draft) = &self.draft else {
            return Task::none();
        };

        let adventure_id = draft.adventure.id;
        let text = draft.opening.text();
        let adventure_manager = self.adventure_manager.clone();

        Task::perform(
            on_tokio(self.handle.clone(), async move {
                adventure_manager
                    .save_opening(adventure_id, &text)
                    .await
                    .map_err(|err| err.to_string())
            }),
            |result| Message::OpeningSaved(resolved(result, "saving the opening failed")),
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
                    .map_err(|err| err.to_string())
            }),
            |result| Message::Published(resolved(result, "publishing the adventure failed")),
        )
    }
}

fn resolved<T>(result: Option<Result<T, String>>, failure: &'static str) -> Result<T, String> {
    match result {
        Some(Ok(value)) => Ok(value),
        Some(Err(err)) => Err(err),
        None => Err(failure.to_string()),
    }
}
