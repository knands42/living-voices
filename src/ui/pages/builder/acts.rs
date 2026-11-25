use iced::widget::text::Wrapping;
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Background, Border, Element, Length, alignment};
use uuid::Uuid;

use crate::backend::domain::memory::Act;
use crate::ui::pages::builder::Effect;
use crate::ui::pages::builder::widgets::{card, empty_state, field, pill, section_header};
use crate::ui::{common, theme};

#[derive(Debug, Clone)]
pub enum Message {
    AddRequested,
    Picked(usize),
    GoalChanged(String),
    TriggerChanged(String),
    SaveRequested,
    Saved(Result<Act, String>),
}

pub struct State {
    acts: Vec<Act>,
    index: Option<usize>,
}

impl State {
    pub fn load(acts: Vec<Act>) -> Self {
        Self { acts, index: None }
    }

    pub fn update(&mut self, message: Message, adventure_id: Uuid) -> Option<Effect> {
        match message {
            Message::AddRequested => {
                let ordinal = self.acts.len() as u16;
                self.acts.push(Act::new(adventure_id, ordinal, ""));
                self.index = Some(self.acts.len() - 1);
            }
            Message::Picked(index) => self.index = Some(index),
            Message::GoalChanged(goal) => self.edit_open(|act| act.goal = goal),
            Message::TriggerChanged(trigger) => self.edit_open(|act| {
                act.trigger = (!trigger.trim().is_empty()).then_some(trigger);
            }),
            Message::SaveRequested => return Some(Effect::SaveAct),
            Message::Saved(Ok(saved)) => {
                if let Some(slot) = self.acts.iter_mut().find(|act| act.id == saved.id) {
                    *slot = saved;
                }
                self.index = None;
            }
            Message::Saved(Err(err)) => log::error!("could not save the act: {err}"),
        }

        None
    }

    pub fn view(&self) -> Element<'_, Message> {
        let head = row![
            section_header(
                "Acts",
                "The story's movements, in order. Each act has a goal."
            ),
            pill("+ Add act", 120.0, true, Message::AddRequested),
        ]
        .spacing(16.0)
        .align_y(Alignment::Center);

        let body: Element<'_, Message> = if self.acts.is_empty() {
            empty_state("No acts yet — add the first movement.")
        } else {
            let rows: Vec<Element<'_, Message>> = self
                .acts
                .iter()
                .enumerate()
                .map(|(index, act)| act_row(index, act, self.index == Some(index)))
                .collect();

            column(rows).spacing(10.0).width(Length::Fill).into()
        };

        column![
            head,
            body,
            text("Every act needs a real goal before the story can start.")
                .size(11.0)
                .color(theme::TEXT_DIM),
        ]
        .spacing(20.0)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    pub fn len(&self) -> usize {
        self.acts.len()
    }

    pub fn is_ready(&self) -> bool {
        !self.acts.is_empty() && self.acts.iter().all(Act::is_valid_act)
    }

    pub fn open(&self) -> Option<&Act> {
        self.index.and_then(|index| self.acts.get(index))
    }

    fn edit_open(&mut self, change: impl FnOnce(&mut Act)) {
        if let Some(act) = self.index.and_then(|index| self.acts.get_mut(index)) {
            change(act);
        }
    }
}

fn act_row(index: usize, act: &Act, editing: bool) -> Element<'_, Message> {
    let edge = if act.is_valid_act() {
        theme::BRONZE_DIM
    } else {
        theme::AMBER
    };

    let content: Element<'_, Message> = if editing {
        fields(act)
    } else {
        button(read(act))
            .padding(0.0)
            .width(Length::Fill)
            .style(act_row_style)
            .on_press(Message::Picked(index))
            .into()
    };

    card(
        row![ordinal(act.ordinal + 1), content]
            .spacing(14.0)
            .align_y(Alignment::Center)
            .width(Length::Fill),
        Length::Fill,
        edge,
    )
}

fn fields(act: &Act) -> Element<'_, Message> {
    let goal = if act.is_valid_act() {
        act.goal.as_str()
    } else {
        ""
    };

    column![
        field(goal, "What must be true by the end…", Message::GoalChanged),
        field(
            act.trigger.as_deref().unwrap_or_default(),
            "Trigger, or nothing to follow on",
            Message::TriggerChanged
        ),
        pill(
            "Save act",
            110.0,
            act.is_valid_act(),
            Message::SaveRequested
        ),
    ]
    .spacing(8.0)
    .width(Length::Fill)
    .into()
}

fn read(act: &Act) -> Element<'_, Message> {
    let goal = if act.is_valid_act() {
        (act.goal.as_str(), theme::TEXT_LIGHT)
    } else {
        ("— needs a goal —", theme::AMBER)
    };

    let trigger = match act.trigger.as_deref() {
        Some(trigger) if !trigger.trim().is_empty() => format!("trigger: {trigger}"),
        _ => "no trigger — follows on".to_string(),
    };

    column![
        text(goal.0)
            .size(14.5)
            .color(goal.1)
            .wrapping(Wrapping::Word),
        text(trigger).size(11.0).color(theme::TEXT_MUTED),
    ]
    .spacing(2.0)
    .width(Length::Fill)
    .into()
}

fn ordinal(ordinal: u16) -> Element<'static, Message> {
    container(
        text(ordinal.to_string())
            .size(12.0)
            .color(theme::GOLD)
            .font(common::brand()),
    )
    .width(Length::Fixed(30.0))
    .height(Length::Fixed(30.0))
    .align_x(alignment::Horizontal::Center)
    .align_y(alignment::Vertical::Center)
    .style(|_theme| {
        container::Style::default()
            .background(theme::BG_PANEL_ALT)
            .border(
                Border::default()
                    .rounded(999.0)
                    .width(1.0)
                    .color(theme::GOLD),
            )
    })
    .into()
}

fn act_row_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);

    button::Style {
        background: hovered.then_some(Background::Color(theme::BG_PANEL_ALT)),
        text_color: theme::TEXT_LIGHT,
        border: Border::default().rounded(12.0),
        ..button::Style::default()
    }
}
