use iced::widget::{column, container, row, text};
use iced::{Alignment, Element, Length};

use crate::ui::pages::builder::widgets::{caption, editor, pill, section_header, surface};
use crate::ui::pages::builder::{Draft, Message};
use crate::ui::theme;

pub fn opening(draft: &Draft) -> Element<'_, Message> {
    let head = row![
        section_header(
            "The Opening",
            "Who is present at the start, and the narrator's first words."
        ),
        pill(
            "Save opening",
            140.0,
            draft.opening_can_save(),
            Message::SaveOpening
        ),
    ]
    .spacing(16.0)
    .align_y(Alignment::Center);

    let state = if draft.opening_saved {
        "Opening is set up."
    } else {
        "Not set up yet."
    };

    let body = column![
        text(state).size(12.5).color(theme::TEXT_MUTED),
        caption("THE NARRATOR'S FIRST WORDS"),
        container(editor(&draft.opening, Message::OpeningEdited))
            .width(Length::Fill)
            .height(Length::FillPortion(1)),
    ]
    .spacing(10.0)
    .width(Length::Fill)
    .height(Length::Fill);

    column![head, surface(body)]
        .spacing(20.0)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
