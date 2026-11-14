use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length, Task};

use crate::backend::application::adventure_manager::AdventureManager;
use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::character::Character;
use crate::backend::domain::memory::{
    Act, ActStatus, CharacterCondition, Fact, State as SceneState,
};
use crate::backend::domain::message::Message as MessageLine;
use crate::ui::pages::{Event, Page};
use crate::ui::{common, theme};

const CAST_LENGTH: f32 = 250.0;
const META_LENGTH: f32 = 280.0;
const ACT_LENGTH: f32 = 220.0;
const BUBBLE_LENGTH: f32 = 460.0;
const NARRATOR_LENGTH: f32 = 560.0;
const AVATAR: f32 = 38.0;

#[derive(Debug, Clone)]
pub enum Message {
    BackRequested,
    InputMessageChanged(String),
    Send,
}

pub struct State {
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
    pub fn boot(adventure_manager: AdventureManager) -> (Self, Task<Message>) {
        let snapshot = adventure_manager.current_snapshot();

        (
            Self {
                adventure_manager,
                adventure: snapshot.current_adventure,
                act: snapshot.current_act,
                scene_state: snapshot.current_state,
                facts: snapshot.current_facts,
                messages: Vec::new(),
                input_message: String::new(),
                awaiting_reply: false,
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, message: Message) -> (Task<Message>, Event) {
        match message {
            Message::BackRequested => return (Task::none(), Event::Show(Page::MainHub)),
            Message::InputMessageChanged(value) => self.input_message = value,
            Message::Send => {}
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

// --- header ---------------------------------------------------------------

fn header(chat_state: &State) -> Element<'_, Message> {
    let back = button(text("← Back").size(11.5))
        .padding([7.0, 13.0])
        .style(theme::back_button)
        .on_press(Message::BackRequested);

    let identity = column![
        text(title(chat_state))
            .size(21.0)
            .color(theme::GOLD)
            .font(common::brand()),
        text(headline(chat_state))
            .size(11.0)
            .color(theme::TEXT_MUTED)
            .width(Length::Fill)
            .wrapping(Wrapping::Word),
    ]
    .spacing(2.0)
    .align_x(Alignment::Center);

    row![
        container(back)
            .width(Length::Fixed(CAST_LENGTH))
            .align_x(Alignment::Start),
        container(identity)
            .width(Length::Fill)
            .align_x(Alignment::Center),
        container(act_chip(chat_state.act.as_ref()))
            .width(Length::Fixed(CAST_LENGTH))
            .align_x(Alignment::End),
    ]
    .spacing(14.0)
    .align_y(Alignment::Center)
    .into()
}

fn title(chat_state: &State) -> &str {
    chat_state
        .adventure
        .as_ref()
        .map_or("No adventure open", |adventure| adventure.title.as_str())
}

fn headline(chat_state: &State) -> &str {
    chat_state
        .adventure
        .as_ref()
        .map_or("Pick an adventure from the shelf", |adventure| {
            adventure.headline.as_str()
        })
}

fn act_chip(act: Option<&Act>) -> Element<'_, Message> {
    let Some(act) = act else {
        return Space::new().into();
    };

    let status = status_color(act.status);
    let label = format!("Act {}", act.ordinal);

    container(
        column![
            row![common::dot(5.0, status), common::eyebrow(&label),]
                .spacing(6.0)
                .align_y(Alignment::Center),
            text(act.goal.as_str())
                .size(11.0)
                .color(theme::TEXT_LIGHT)
                .width(Length::Fill)
                .wrapping(Wrapping::Word),
            text(act.status.as_str().to_uppercase())
                .size(9.0)
                .color(status)
                .font(common::semibold()),
        ]
        .spacing(4.0),
    )
    .padding([9.0, 12.0])
    .max_width(ACT_LENGTH)
    .style(theme::stat)
    .into()
}

fn status_color(status: ActStatus) -> iced::Color {
    match status {
        ActStatus::Active => theme::GOLD,
        ActStatus::Resolved => theme::SEA,
        ActStatus::Failed => theme::ROSE,
        ActStatus::Unstarted | ActStatus::Bypassed => theme::TEXT_DIM,
    }
}

// --- the party ------------------------------------------------------------

fn cast_rail(chat_state: &State) -> Element<'_, Message> {
    let mut rows = column![].spacing(4.0).width(Length::Fill);

    if let Some(adventure) = &chat_state.adventure {
        for character in &adventure.characters {
            rows = rows.push(cast_row(character));
        }
    }

    common::panel(
        column![
            common::heading("Cast"),
            scrollable(rows).height(Length::Fill),
        ]
        .spacing(10.0)
        .height(Length::Fill),
        14.0,
    )
    .width(Length::Fixed(CAST_LENGTH))
    .height(Length::Fill)
    .into()
}

fn cast_row(character: &Character) -> Element<'_, Message> {
    let hue = name_hue(character.name.as_str());
    let initials = initials_of(character.name.as_str());
    let avatar = common::initials(&initials, hue, AVATAR, 0.55);

    let name = row![
        text(character.name.as_str())
            .size(12.0)
            .color(theme::hsv(hue, 0.45, 0.95))
            .font(common::semibold()),
        text(character.status.as_str())
            .size(9.0)
            .color(theme::TEXT_DIM),
    ]
    .spacing(6.0)
    .align_y(Alignment::Center);

    let identity = column![
        name,
        text(character.headline.as_str())
            .size(10.0)
            .color(theme::TEXT_MUTED)
            .width(Length::Fill)
            .wrapping(Wrapping::Word),
    ]
    .spacing(2.0)
    .width(Length::Fill);

    let body = row![avatar, identity]
        .spacing(9.0)
        .align_y(Alignment::Start);

    if character.is_player {
        container(body)
            .padding(9.0)
            .style(theme::player_card)
            .into()
    } else {
        container(body).padding(9.0).into()
    }
}

// --- the transcript -------------------------------------------------------

fn transcript(chat_state: &State) -> Element<'_, Message> {
    let mut lines = column![].spacing(16.0).width(Length::Fill);

    if chat_state.messages.is_empty() && !chat_state.awaiting_reply {
        lines = lines.push(blank_page());
    }

    for message in &chat_state.messages {
        lines = lines.push(line(chat_state, message));
    }

    if chat_state.awaiting_reply {
        lines = lines.push(typing());
    }

    let history = scrollable(container(lines).padding([4.0, 8.0]))
        .height(Length::Fill)
        .anchor_bottom();

    common::panel(
        column![history, composer(chat_state)]
            .spacing(12.0)
            .height(Length::Fill),
        16.0,
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn blank_page() -> Element<'static, Message> {
    container(
        column![
            text("◆").size(11.0).color(theme::alpha(theme::GOLD, 0.55)),
            text("The page is blank. Say what you do.")
                .size(12.5)
                .color(theme::TEXT_DIM)
                .font(common::italic()),
        ]
        .spacing(6.0)
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([48.0, 0.0])
    .align_x(Alignment::Center)
    .into()
}

fn line<'a>(chat_state: &'a State, message: &'a MessageLine) -> Element<'a, Message> {
    if message.narrator_id.is_some() {
        return narrator(message);
    }

    speech(chat_state, message)
}

fn narrator(message: &MessageLine) -> Element<'_, Message> {
    let card = container(
        column![
            text("◆").size(9.0).color(theme::alpha(theme::AMBER, 0.9)),
            text(message.text.as_str())
                .size(12.5)
                .color(theme::TEXT_MUTED)
                .font(common::italic())
                .width(Length::Fill)
                .wrapping(Wrapping::Word),
        ]
        .spacing(5.0)
        .align_x(Alignment::Center),
    )
    .padding([10.0, 16.0])
    .max_width(NARRATOR_LENGTH)
    .style(move |_theme| theme::narrator(1.0));

    container(card)
        .width(Length::Fill)
        .align_x(Alignment::Center)
        .into()
}

fn speech<'a>(chat_state: &'a State, message: &'a MessageLine) -> Element<'a, Message> {
    let hue = name_hue(message.speaker_name.as_str());
    let is_player = chat_state
        .adventure
        .as_ref()
        .and_then(|adventure| adventure.characters.iter().find(|cast| cast.is_player))
        .is_some_and(|player| message.character_id == Some(player.id));

    let mut heading = row![
        text(message.speaker_name.as_str())
            .size(11.5)
            .color(theme::hsv(hue, 0.45, 0.95))
            .font(common::semibold()),
        text(message.created_at.format("%H:%M").to_string())
            .size(9.5)
            .color(theme::TEXT_DIM),
    ]
    .spacing(7.0)
    .align_y(Alignment::Center);

    if message.ooc {
        heading = heading.push(
            text("ooc")
                .size(9.0)
                .color(theme::AMBER)
                .font(common::semibold()),
        );
    }

    let words = text(message.text.as_str())
        .size(13.0)
        .color(theme::TEXT_LIGHT)
        .width(Length::Fill)
        .wrapping(Wrapping::Word);

    let bubble = container(words)
        .padding([9.0, 13.0])
        .max_width(BUBBLE_LENGTH)
        .style(move |_theme| theme::bubble(hue, is_player, 1.0));

    let initials = initials_of(message.speaker_name.as_str());
    let avatar = common::initials(&initials, hue, AVATAR, 0.55);

    if is_player {
        row![
            Space::new().width(Length::Fill),
            column![
                container(heading)
                    .width(Length::Fill)
                    .align_x(Alignment::End),
                bubble,
            ]
            .spacing(4.0)
            .align_x(Alignment::End),
            avatar,
        ]
        .spacing(9.0)
        .align_y(Alignment::Start)
        .into()
    } else {
        row![
            avatar,
            column![heading, bubble].spacing(4.0).width(Length::Fill),
        ]
        .spacing(9.0)
        .align_y(Alignment::Start)
        .into()
    }
}

fn typing() -> Element<'static, Message> {
    let mut dots = row![].spacing(4.0).align_y(Alignment::Center);

    for fade in [0.85, 0.55, 0.30] {
        dots = dots.push(common::dot(5.0, theme::alpha(theme::GOLD, fade)));
    }

    row![
        dots,
        text("writing…")
            .size(10.5)
            .color(theme::TEXT_DIM)
            .font(common::italic()),
    ]
    .spacing(8.0)
    .align_y(Alignment::Center)
    .into()
}

fn composer(chat_state: &State) -> Element<'_, Message> {
    let can_send = !chat_state.input_message.trim().is_empty() && !chat_state.awaiting_reply;

    let input = text_input("Speak, or do something…", chat_state.input_message.as_str())
        .on_input(Message::InputMessageChanged)
        .on_submit(Message::Send)
        .padding([13.0, 15.0])
        .size(13.5)
        .style(theme::composer);

    let send = button(text("Send  ↵").size(13.0).font(common::semibold()))
        .padding([12.0, 20.0])
        .style(theme::send_button)
        .on_press_maybe(can_send.then_some(Message::Send));

    row![input.width(Length::Fill), send]
        .spacing(9.0)
        .align_y(Alignment::Center)
        .into()
}

// --- what's going on ------------------------------------------------------

fn meta_rail(chat_state: &State) -> Element<'_, Message> {
    let scene = chat_state.scene_state.as_ref();

    let conditions = scene
        .map(|scene| {
            scene
                .character_conditions
                .iter()
                .map(CharacterCondition::to_string)
                .collect()
        })
        .unwrap_or_default();

    let threads = scene
        .map(|scene| scene.open_threads.clone())
        .unwrap_or_default();

    let mut notes = column![common::heading("What's going on")]
        .spacing(12.0)
        .width(Length::Fill);

    notes = notes.push(note_list("Conditions", conditions));
    notes = notes.push(note_list("Open threads", threads));

    if !chat_state.facts.is_empty() {
        let mut facts = column![common::subheading("Facts")].spacing(6.0);

        for fact in &chat_state.facts {
            facts = facts.push(fact_row(fact));
        }

        notes = notes.push(facts);
    }

    if let Some(scene) = scene {
        notes = notes.push(
            column![
                common::subheading("Time and place"),
                text(scene.day.as_str()).size(10.5).color(theme::TEXT_MUTED),
                text(scene.place.as_str())
                    .size(10.5)
                    .color(theme::TEXT_MUTED)
                    .width(Length::Fill)
                    .wrapping(Wrapping::Word),
            ]
            .spacing(3.0),
        );
    }

    common::panel(
        column![scrollable(notes).height(Length::Fill)].width(Length::Fill),
        14.0,
    )
    .width(Length::Fixed(META_LENGTH))
    .height(Length::Fill)
    .into()
}

fn note_list<'a>(title: &'a str, items: Vec<String>) -> Element<'a, Message> {
    if items.is_empty() {
        return Space::new().into();
    }

    let mut rows = column![common::subheading(title)].spacing(3.0);

    for item in items {
        rows = rows.push(
            text(format!("· {item}"))
                .size(10.5)
                .color(theme::TEXT_MUTED)
                .width(Length::Fill)
                .wrapping(Wrapping::Word),
        );
    }

    rows.into()
}

fn fact_row(fact: &Fact) -> Element<'_, Message> {
    container(
        column![
            row![
                text(fact.subject.as_str())
                    .size(10.5)
                    .color(theme::BRONZE)
                    .font(common::semibold()),
                text(fact.permanence.as_str())
                    .size(8.5)
                    .color(theme::TEXT_DIM),
            ]
            .spacing(6.0)
            .align_y(Alignment::Center),
            text(fact.fact.as_str())
                .size(10.5)
                .color(theme::TEXT_MUTED)
                .width(Length::Fill)
                .wrapping(Wrapping::Word),
        ]
        .spacing(2.0),
    )
    .padding([6.0, 8.0])
    .width(Length::Fill)
    .style(theme::inset)
    .into()
}

// --- helpers --------------------------------------------------------------

fn initials_of(name: &str) -> String {
    name.chars().take(2).collect::<String>().to_uppercase()
}

fn name_hue(name: &str) -> f32 {
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    (hasher.finish() % 360) as f32
}
