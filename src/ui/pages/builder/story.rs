use iced::widget::{column, row, text};
use iced::{Alignment, Element, Length};

use crate::ui::pages::builder::widgets::{caption, editor, field, pill, section_header, surface};
use crate::ui::pages::builder::{Draft, Message};
use crate::ui::theme;

pub fn story(draft: &Draft) -> Element<'_, Message> {
    let saved = if draft.story_is_dirty() {
        ("Unsaved changes", theme::AMBER)
    } else {
        ("All changes saved", theme::TEXT_MUTED)
    };

    let head = row![
        section_header(
            "The Story",
            "Title, headline, lore, narrator voice and tone."
        ),
        text(saved.0).size(12.0).color(saved.1),
        pill(
            "Save story",
            130.0,
            draft.story_can_save(),
            Message::SaveStory
        ),
    ]
    .spacing(16.0)
    .align_y(Alignment::Center);

    let form = column![
        column![
            caption("TITLE"),
            field(
                draft.adventure.title.as_str(),
                "Name your adventure…",
                Message::TitleChanged
            ),
        ]
        .spacing(8.0),
        column![
            caption("HEADLINE"),
            field(
                draft.adventure.headline.as_str(),
                "A one-line hook…",
                Message::HeadlineChanged
            ),
        ]
        .spacing(8.0),
        row![
            column![
                caption("NARRATOR"),
                field(
                    draft.adventure.narrator.name.as_str(),
                    "Who tells the story?",
                    Message::NarratorChanged
                ),
            ]
            .spacing(8.0)
            .width(Length::FillPortion(1)),
            column![
                caption("TONE"),
                field(
                    draft.adventure.tone.as_str(),
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
            editor(&draft.lore, 220.0, Message::LoreEdited),
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
