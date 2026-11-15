use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Background, Border, Element, Length, alignment};

use crate::backend::application::port::chat_port::PromptPort;
use crate::backend::application::port::image_port::ImagePort;
use crate::ui::pages::builder::{Draft, Message, Section, State};
use crate::ui::{common, theme};

const SIDEBAR_WIDTH: f32 = 240.0;

pub fn sidebar<P: PromptPort + Clone + 'static, I: ImagePort + Clone + 'static>(
    state: &State<P, I>,
) -> Element<'_, Message> {
    let draft = state.draft.as_ref();

    let head = row![
        back(Message::BackRequested),
        column![common::eyebrow("draft"), title(draft)]
            .spacing(3.0)
            .width(Length::Fill),
    ]
    .spacing(8.0)
    .width(Length::Fill);

    let mut nav = column![].spacing(4.0).width(Length::Fill);
    for section in Section::ALL {
        nav = nav.push(nav_row(draft, state.section, section));
    }

    let footer: Element<'_, Message> = match draft {
        Some(draft) => readiness(draft),
        None => Space::new().height(Length::Shrink).into(),
    };

    container(
        column![
            head,
            common::divider(),
            Space::new().height(Length::Fixed(8.0)),
            nav,
            Space::new().height(Length::Fill),
            footer,
        ]
        .spacing(12.0)
        .width(Length::Fill),
    )
    .padding(iced::Padding {
        top: 28.0,
        right: 16.0,
        bottom: 16.0,
        left: 16.0,
    })
    .width(Length::Fixed(SIDEBAR_WIDTH))
    .height(Length::Fill)
    .style(|_theme| {
        container::Style::default()
            .background(theme::BG_PANEL)
            .border(Border::default().width(1.0).color(theme::BRONZE_DIM))
    })
    .into()
}

fn back(message: Message) -> Element<'static, Message> {
    button(
        container(
            text("‹")
                .size(22.0)
                .color(theme::GOLD)
                .font(common::brand()),
        )
        .width(Length::Fixed(32.0))
        .height(Length::Fixed(32.0))
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center),
    )
    .padding(0.0)
    .style(theme::back_button)
    .on_press(message)
    .into()
}

fn title(draft: Option<&Draft>) -> Element<'static, Message> {
    let title = draft
        .map(|draft| draft.adventure.title.trim())
        .filter(|title| !title.is_empty())
        .unwrap_or("Untitled adventure")
        .to_string();

    text(title)
        .size(15.0)
        .color(theme::GOLD)
        .font(common::brand())
        .wrapping(Wrapping::Word)
        .into()
}

fn nav_row(draft: Option<&Draft>, current: Section, section: Section) -> Element<'static, Message> {
    let active = current == section;
    let done = draft.is_some_and(|draft| draft.section_is_done(section));

    let mark = if done { "✓" } else { "○" };
    let mark_tint = if done { theme::SEA } else { theme::TEXT_MUTED };
    let label_tint = if active {
        theme::GOLD
    } else {
        theme::TEXT_LIGHT
    };

    let label = row![
        text(mark).size(14.0).color(mark_tint),
        text(section.label())
            .size(15.0)
            .color(label_tint)
            .width(Length::Fill),
        text(trailing(draft, section))
            .size(12.0)
            .color(theme::TEXT_MUTED),
    ]
    .spacing(12.0)
    .align_y(Alignment::Center);

    button(container(label).padding([12.0, 14.0]).width(Length::Fill))
        .width(Length::Fill)
        .style(move |_theme, status| {
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            nav_row_style(active, hovered)
        })
        .on_press(Message::SectionPicked(section))
        .into()
}

fn nav_row_style(active: bool, hovered: bool) -> button::Style {
    let lit = active || hovered;

    button::Style {
        background: lit.then(|| Background::Color(theme::BG_PANEL_ALT)),
        text_color: theme::TEXT_LIGHT,
        border: if active {
            Border::default()
                .rounded(14.0)
                .width(1.0)
                .color(theme::GOLD)
        } else {
            Border::default().rounded(14.0)
        },
        ..button::Style::default()
    }
}

fn trailing(draft: Option<&Draft>, section: Section) -> String {
    let Some(draft) = draft else {
        return String::new();
    };

    match section {
        Section::Cast => draft
            .adventure
            .characters
            .iter()
            .filter(|character| !character.is_player)
            .count()
            .to_string(),
        Section::Acts => draft.acts.len().to_string(),
        Section::Story | Section::Opening => String::new(),
    }
}

fn readiness(draft: &Draft) -> Element<'_, Message> {
    let publishable = draft.is_publishable();
    let tint = if publishable {
        theme::SEA
    } else {
        theme::AMBER
    };

    column![
        common::divider(),
        row![
            common::eyebrow("readiness"),
            Space::new().width(Length::Fill),
            text(format!("{} / {}", draft.ready_count(), Section::ALL.len()))
                .size(15.0)
                .color(tint)
                .font(common::brand()),
        ]
        .width(Length::Fill)
        .align_y(Alignment::Center),
        readiness_bar(draft),
        publish_button(draft),
    ]
    .spacing(12.0)
    .into()
}

fn readiness_bar(draft: &Draft) -> Element<'_, Message> {
    let segments = Section::ALL.map(|section| draft.section_is_done(section));

    let mut bar = row![].spacing(4.0).width(Length::Fill);
    for done in segments {
        bar = bar.push(
            container(Space::new().height(Length::Fixed(4.0)))
                .width(Length::Fill)
                .style(move |_theme| {
                    container::Style::default()
                        .background(if done { theme::SEA } else { theme::BRONZE_DIM })
                        .border(Border::default().rounded(2.0))
                }),
        );
    }

    bar.into()
}

fn publish_button(draft: &Draft) -> Element<'_, Message> {
    let publishable = draft.is_publishable();
    let label = if publishable {
        "Publish"
    } else {
        "Publish (locked)"
    };

    button(
        container(
            text(label)
                .size(13.0)
                .font(common::semibold())
                .width(Length::Fill)
                .align_x(alignment::Horizontal::Center),
        )
        .padding([9.0, 12.0])
        .width(Length::Fill),
    )
    .width(Length::Fill)
    .style(theme::send_button)
    .on_press_maybe(publishable.then_some(Message::Publish))
    .into()
}
