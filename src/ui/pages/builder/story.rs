use iced::widget::text_editor;
use iced::widget::{column, row, text};
use iced::{Alignment, Element, Length};

use crate::backend::domain::adventure::Adventure;
use crate::ui::pages::builder::Effect;
use crate::ui::pages::builder::widgets::{caption, editor, field, pill, section_header, surface};
use crate::ui::theme;

#[derive(Debug, Clone)]
pub enum Message {
    TitleChanged(String),
    HeadlineChanged(String),
    NarratorChanged(String),
    ToneChanged(String),
    LoreEdited(text_editor::Action),
    SaveRequested,
    Saved(Result<Adventure, String>),
}

pub struct State {
    lore: text_editor::Content,
    changed: bool,
}

impl State {
    pub fn load(lore: &str) -> Self {
        Self {
            lore: text_editor::Content::with_text(lore),
            changed: false,
        }
    }

    pub fn update(&mut self, message: Message, adventure: &mut Adventure) -> Option<Effect> {
        match message {
            Message::TitleChanged(title) => {
                adventure.title = title;
                self.changed = true;
            }
            Message::HeadlineChanged(headline) => {
                adventure.headline = headline;
                self.changed = true;
            }
            Message::NarratorChanged(name) => {
                adventure.narrator.name = name;
                self.changed = true;
            }
            Message::ToneChanged(tone) => {
                adventure.tone = tone;
                self.changed = true;
            }
            Message::LoreEdited(action) => self.lore.perform(action),
            Message::SaveRequested => return Some(Effect::SaveStory),
            Message::Saved(Ok(saved)) => {
                *adventure = saved;
                self.changed = false;
            }
            Message::Saved(Err(err)) => log::error!("could not save the story: {err}"),
        }

        None
    }

    pub fn view<'a>(&'a self, adventure: &'a Adventure) -> Element<'a, Message> {
        let saved = if self.is_dirty(adventure) {
            ("Unsaved changes", theme::AMBER)
        } else {
            ("All changes saved", theme::TEXT_MUTED)
        };

        let head = row![
            section_header(
                "The Story",
                "Title, headline, lore, narrator voice and the adventure's tone."
            ),
            text(saved.0).size(12.0).color(saved.1),
            pill(
                "Save story",
                130.0,
                self.can_save(adventure),
                Message::SaveRequested
            ),
        ]
        .spacing(16.0)
        .align_y(Alignment::Center);

        let form = column![
            column![
                caption("TITLE"),
                field(
                    adventure.title.as_str(),
                    "Name your adventure…",
                    Message::TitleChanged
                ),
            ]
            .spacing(8.0),
            column![
                caption("HEADLINE"),
                field(
                    adventure.headline.as_str(),
                    "A one-line hook…",
                    Message::HeadlineChanged
                ),
            ]
            .spacing(8.0),
            row![
                column![
                    caption("NARRATOR"),
                    field(
                        adventure.narrator.name.as_str(),
                        "Who tells the story?",
                        Message::NarratorChanged
                    ),
                ]
                .spacing(8.0)
                .width(Length::FillPortion(1)),
                column![
                    caption("TONE"),
                    field(
                        adventure.tone.as_str(),
                        "grim, whimsical, tense…",
                        Message::ToneChanged
                    ),
                ]
                .spacing(8.0)
                .width(Length::FillPortion(1)),
            ]
            .spacing(24.0),
            column![
                caption("LORE"),
                editor(&self.lore, 220.0, Message::LoreEdited),
            ]
            .spacing(8.0)
            .height(Length::FillPortion(2)),
        ]
        .spacing(22.0)
        .height(Length::Fill)
        .width(Length::Fill);

        column![head, surface(form)]
            .spacing(20.0)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    pub fn lore_text(&self) -> String {
        self.lore.text()
    }

    pub fn is_ready(&self, adventure: &Adventure) -> bool {
        !adventure.title.trim().is_empty()
            && !adventure.headline.trim().is_empty()
            && !adventure.tone.trim().is_empty()
            && !adventure.narrator.name.trim().is_empty()
            && !self.lore.text().trim().is_empty()
    }

    pub fn can_save(&self, adventure: &Adventure) -> bool {
        self.is_dirty(adventure) && !adventure.title.trim().is_empty()
    }

    fn is_dirty(&self, adventure: &Adventure) -> bool {
        self.changed || self.lore.text() != adventure.lore.text
    }
}
