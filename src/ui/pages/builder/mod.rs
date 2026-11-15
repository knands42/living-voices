mod acts;
mod cast;
mod opening;
mod sidebar;
mod story;
mod widgets;

use chrono::Utc;
use iced::widget::{container, row, scrollable, text_editor};
use iced::{Element, Length, Task};
use uuid::Uuid;

use crate::backend::application::adventure_manager::{AdventureManager, AdventureManagerError};
use crate::backend::application::character_manager::{CharacterManager, CharacterManagerError};
use crate::backend::application::port::chat_port::PromptPort;
use crate::backend::application::port::image_port::ImagePort;
use crate::backend::domain::adventure::{Adventure, AdventureStatus};
use crate::backend::domain::character::Character;
use crate::backend::domain::memory::{Act, Fact, Permanence, State as SceneState};
use crate::backend::domain::message::Message as MessageLine;
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
    CharacterPicked(usize),
    CharacterChanged(Character),
    CharacterEditClosed,
    RemoveCharacter,
    CharacterRemoved(Uuid, Result<(), String>),
    SaveCast,
    CastSaved(Result<(), String>),
    AddAct,
    ActPicked(usize),
    ActGoalChanged(String),
    ActTriggerChanged(String),
    SaveAct,
    ActSaved(Result<Act, String>),
    OpeningPlaceChanged(String),
    OpeningDayChanged(String),
    FactSubjectChanged(usize, String),
    FactTextChanged(usize, String),
    AddFact,
    BeatEdited(usize, text_editor::Action),
    BeatSpeakerPicked(usize, Speaker),
    AddBeat,
    RemoveBeat(usize),
    SaveOpening,
    OpeningSaved(Result<(), String>),
    Publish,
    Published(Result<(), String>),
}

#[derive(Debug, Clone)]
pub struct Opened {
    adventure: Adventure,
    acts: Vec<Act>,
    scene: Option<SceneState>,
    facts: Vec<Fact>,
    messages: Vec<MessageLine>,
}

/// Who says one line of the opening.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Speaker {
    Narrator,
    Character(Uuid),
}

pub struct Beat {
    speaker: Speaker,
    content: text_editor::Content,
}

impl Beat {
    fn narrator() -> Self {
        Self {
            speaker: Speaker::Narrator,
            content: text_editor::Content::new(),
        }
    }

    fn from_message(message: &MessageLine) -> Self {
        let speaker = match message.character_id {
            Some(character_id) => Speaker::Character(character_id),
            None => Speaker::Narrator,
        };

        Self {
            speaker,
            content: text_editor::Content::with_text(&message.text),
        }
    }

    fn text(&self) -> String {
        self.content.text()
    }

    fn message(&self, adventure: &Adventure) -> MessageLine {
        let text = self.text();

        match self.speaker {
            Speaker::Narrator => MessageLine::new(
                adventure.id,
                None,
                Some(adventure.narrator.id),
                MessageLine::NARRATOR_SPEAKER_NAME.to_string(),
                text,
                false,
            ),
            Speaker::Character(character_id) => {
                let speaker_name = adventure
                    .characters
                    .iter()
                    .find(|character| character.id == character_id)
                    .map(|character| character.name.clone())
                    .unwrap_or_default();

                MessageLine::new(
                    adventure.id,
                    Some(character_id),
                    None,
                    speaker_name,
                    text,
                    false,
                )
            }
        }
    }
}

#[derive(Default)]
struct FactDraft {
    subject: String,
    fact: String,
}

/// The scene the adventure starts in: where and when, the facts around it, and
/// the first lines, the first of which the narrator speaks.
pub struct Opening {
    place: String,
    day: String,
    facts: Vec<FactDraft>,
    beats: Vec<Beat>,
}

impl Opening {
    const MAX_BEATS: usize = 3;

    fn load(scene: &SceneState, facts: Vec<Fact>, messages: Vec<MessageLine>) -> Self {
        let beats = if messages.is_empty() {
            vec![Beat::narrator()]
        } else {
            messages
                .iter()
                .take(Self::MAX_BEATS)
                .map(Beat::from_message)
                .collect()
        };

        let facts = facts
            .into_iter()
            .map(|fact| FactDraft {
                subject: fact.subject,
                fact: fact.fact,
            })
            .collect();

        Self {
            place: from_scene(&scene.place),
            day: from_scene(&scene.day),
            facts,
            beats,
        }
    }

    fn stored_place(&self) -> String {
        to_scene(&self.place)
    }

    fn stored_day(&self) -> String {
        to_scene(&self.day)
    }

    fn fact_snapshot(&self) -> Vec<Fact> {
        self.facts
            .iter()
            .filter(|draft| !draft.fact.trim().is_empty())
            .map(|draft| {
                Fact::new(
                    draft.subject.trim(),
                    draft.fact.trim(),
                    Permanence::Permanent,
                )
            })
            .collect()
    }

    fn messages(&self, adventure: &Adventure) -> Vec<MessageLine> {
        self.beats
            .iter()
            .filter(|beat| !beat.text().trim().is_empty())
            .map(|beat| beat.message(adventure))
            .collect()
    }

    fn is_ready(&self) -> bool {
        !self.place.trim().is_empty()
            && !self.day.trim().is_empty()
            && !self.first_line().trim().is_empty()
    }

    fn can_save(&self) -> bool {
        !self.first_line().trim().is_empty()
    }

    fn first_line(&self) -> String {
        self.beats.first().map(Beat::text).unwrap_or_default()
    }

    fn add_fact(&mut self) {
        self.facts.push(FactDraft::default());
    }

    fn edit_fact(&mut self, index: usize, change: impl FnOnce(&mut FactDraft)) {
        if let Some(fact) = self.facts.get_mut(index) {
            change(fact);
        }
    }

    fn edit_beat(&mut self, index: usize, action: text_editor::Action) {
        if let Some(beat) = self.beats.get_mut(index) {
            beat.content.perform(action);
        }
    }

    fn pick_speaker(&mut self, index: usize, speaker: Speaker) {
        if index == 0 {
            return;
        }

        if let Some(beat) = self.beats.get_mut(index) {
            beat.speaker = speaker;
        }
    }

    fn add_beat(&mut self) {
        if self.beats.len() < Self::MAX_BEATS {
            self.beats.push(Beat::narrator());
        }
    }

    fn remove_beat(&mut self, index: usize) {
        if index > 0 && index < self.beats.len() {
            self.beats.remove(index);
        }
    }
}

fn from_scene(value: &str) -> String {
    if value == SceneState::UNKNOWN {
        String::new()
    } else {
        value.to_string()
    }
}

fn to_scene(value: &str) -> String {
    if value.trim().is_empty() {
        SceneState::UNKNOWN.to_string()
    } else {
        value.trim().to_string()
    }
}

/// The adventure as the page edits it: the persisted copy plus the edits that
/// have not been written yet.
pub struct Draft {
    adventure: Adventure,
    acts: Vec<Act>,
    scene: SceneState,
    opening: Opening,
    lore: text_editor::Content,
    story_changed: bool,
    act_index: Option<usize>,
    character_index: Option<usize>,
}

impl Draft {
    fn new(opened: Opened) -> Self {
        let scene = opened
            .scene
            .unwrap_or_else(|| SceneState::new(Uuid::new_v4(), opened.adventure.id));
        let lore = text_editor::Content::with_text(&opened.adventure.lore.text);
        let opening = Opening::load(&scene, opened.facts, opened.messages);

        Self {
            adventure: opened.adventure,
            acts: opened.acts,
            scene,
            opening,
            lore,
            story_changed: false,
            act_index: None,
            character_index: None,
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
        !self.adventure.characters.is_empty()
    }

    fn edited_character(&self) -> Option<&Character> {
        self.character_index
            .and_then(|index| self.adventure.characters.get(index))
    }

    fn replace_character(&mut self, character: Character) {
        if let Some(existing) = self
            .adventure
            .characters
            .iter_mut()
            .find(|existing| existing.id == character.id)
        {
            *existing = character;
        }
    }

    fn acts_are_ready(&self) -> bool {
        !self.acts.is_empty() && self.acts.iter().all(Act::is_valid_act)
    }

    fn section_is_done(&self, section: Section) -> bool {
        match section {
            Section::Story => self.story_is_ready(),
            Section::Cast => self.cast_is_ready(),
            Section::Acts => self.acts_are_ready(),
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
                let scene = adventure_manager.get_state(adventure_id).await?;
                let facts = adventure_manager.get_facts(adventure_id).await?;
                let messages = adventure_manager
                    .get_messages(adventure_id, Opening::MAX_BEATS)
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
                    draft.character_index = Some(draft.adventure.characters.len() - 1);
                }
            }
            Message::CharacterAdded(Err(err)) => log::error!("could not add a character: {err}"),
            Message::CharacterPicked(index) => {
                if let Some(draft) = &mut self.draft {
                    draft.character_index = Some(index);
                }
            }
            Message::CharacterChanged(character) => {
                if let Some(draft) = &mut self.draft {
                    draft.replace_character(character);
                }
            }
            Message::CharacterEditClosed => {
                if let Some(draft) = &mut self.draft {
                    draft.character_index = None;
                }
            }
            Message::RemoveCharacter => return (self.remove_character(), Event::None),
            Message::CharacterRemoved(character_id, Ok(())) => {
                if let Some(draft) = &mut self.draft {
                    draft
                        .adventure
                        .characters
                        .retain(|character| character.id != character_id);
                    draft.character_index = None;
                }
            }
            Message::CharacterRemoved(_, Err(err)) => {
                log::error!("could not remove the character: {err}")
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
            Message::OpeningPlaceChanged(place) => {
                self.edit_opening(|opening| opening.place = place)
            }
            Message::OpeningDayChanged(day) => self.edit_opening(|opening| opening.day = day),
            Message::FactSubjectChanged(index, subject) => {
                self.edit_opening(|opening| opening.edit_fact(index, |fact| fact.subject = subject))
            }
            Message::FactTextChanged(index, text) => {
                self.edit_opening(|opening| opening.edit_fact(index, |fact| fact.fact = text))
            }
            Message::AddFact => self.edit_opening(Opening::add_fact),
            Message::BeatEdited(index, action) => {
                self.edit_opening(|opening| opening.edit_beat(index, action))
            }
            Message::BeatSpeakerPicked(index, speaker) => {
                self.edit_opening(|opening| opening.pick_speaker(index, speaker))
            }
            Message::AddBeat => self.edit_opening(Opening::add_beat),
            Message::RemoveBeat(index) => self.edit_opening(|opening| opening.remove_beat(index)),
            Message::SaveOpening => return (self.save_opening(), Event::None),
            Message::OpeningSaved(result) => {
                if let Err(err) = result {
                    log::error!("could not save the opening: {err}");
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

    fn edit_opening(&mut self, change: impl FnOnce(&mut Opening)) {
        if let Some(draft) = &mut self.draft {
            change(&mut draft.opening);
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
                adventure_manager.update_adventure(&adventure).await
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
                character_manager.create_draft(adventure_id).await
            }),
            |result| Message::CharacterAdded(resolved(result, "adding the character failed")),
        )
    }

    fn remove_character(&self) -> Task<Message> {
        let Some(character) = self
            .draft
            .as_ref()
            .and_then(Draft::edited_character)
            .cloned()
        else {
            return Task::none();
        };

        let character_id = character.id;
        let character_manager = self.character_manager.clone();

        Task::perform(
            on_tokio(self.handle.clone(), async move {
                character_manager.delete_character(character).await
            }),
            move |result| {
                Message::CharacterRemoved(
                    character_id,
                    resolved(result, "removing the character failed"),
                )
            },
        )
    }

    fn save_cast(&self) -> Task<Message> {
        let Some(draft) = &self.draft else {
            return Task::none();
        };

        let cast = draft.adventure.characters.clone();
        let character_manager = self.character_manager.clone();

        Task::perform(
            on_tokio(self.handle.clone(), async move {
                for character in cast {
                    character_manager.update_character(character).await?;
                }
                Ok::<_, CharacterManagerError>(())
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
                adventure_manager.upsert_act(&act).await?;
                Ok::<_, AdventureManagerError>(act)
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

        let mut scene = draft.scene.clone();
        scene.place = draft.opening.stored_place();
        scene.day = draft.opening.stored_day();
        scene.updated_at = Utc::now();

        let adventure_id = draft.adventure.id;
        let facts = draft.opening.fact_snapshot();
        let messages = draft.opening.messages(&draft.adventure);
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
