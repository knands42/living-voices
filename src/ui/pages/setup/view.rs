use crate::provision::{Gpu, MediaRecommendation};
use crate::ui::pages::setup::Message;
use crate::ui::{common, theme};
use iced::widget::{Space, column, container, row, text};
use iced::{Alignment, Color, Element, Length};

// --- what the machine has -------------------------------------------------

pub fn specs(gpu: Gpu, recommendation: MediaRecommendation) -> Element<'static, Message> {
    let top = row![
        common::subheading("This machine"),
        Space::new().width(Length::Fill),
        text("read at launch").size(10.5).color(theme::TEXT_DIM),
    ]
    .align_y(Alignment::Center);

    let cells = row![
        reading(
            "Graphics card",
            gpu.card_label().to_string(),
            theme::TEXT_LIGHT
        ),
        reading("Video memory", gpu.vram_label(), tone(recommendation)),
    ]
    .spacing(10.0);

    container(
        column![top, common::divider(), cells]
            .spacing(12.0)
            .width(Length::Fill),
    )
    .padding(18.0)
    .width(Length::Fill)
    .style(theme::panel)
    .into()
}

/// One sunk cell: a tiny all-caps label over a value.
fn reading(label: &str, value: String, tint: Color) -> Element<'_, Message> {
    container(
        column![
            common::eyebrow(label),
            text(value).size(13.5).color(tint).font(common::semibold()),
        ]
        .spacing(5.0)
        .width(Length::Fill),
    )
    .padding([11.0, 13.0])
    .width(Length::Fill)
    .style(theme::stat)
    .into()
}

// --- what the machine should do -------------------------------------------

/// The verdict on images: a tinted card whose left edge carries the colour.
pub fn verdict(recommendation: MediaRecommendation) -> Element<'static, Message> {
    let edge = container(Space::new().width(Length::Fixed(3.0)).height(Length::Fill))
        .style(move |_theme| theme::accent(tone(recommendation), 0.9));

    let words = column![
        text(headline(recommendation))
            .size(15.5)
            .color(tone(recommendation))
            .font(common::semibold()),
        text(advice(recommendation))
            .size(13.0)
            .color(theme::TEXT_MUTED)
            .width(Length::Fill)
            .wrapping(text::Wrapping::Word),
    ]
    .spacing(6.0)
    .width(Length::Fill);

    container(row![edge, words].spacing(14.0).width(Length::Fill))
        .padding([14.0, 16.0])
        .width(Length::Fill)
        .style(move |_theme| theme::callout(tone(recommendation), 1.0))
        .into()
}

fn tone(recommendation: MediaRecommendation) -> Color {
    match recommendation {
        MediaRecommendation::Comfortable => theme::SEA,
        MediaRecommendation::Tight => theme::AMBER,
        MediaRecommendation::Limited => theme::ROSE,
    }
}

fn headline(recommendation: MediaRecommendation) -> &'static str {
    match recommendation {
        MediaRecommendation::Comfortable => "Plenty of room for images.",
        MediaRecommendation::Tight => "Images will fit, with little headroom.",
        MediaRecommendation::Limited => "Images will not run well on this machine.",
    }
}

fn advice(recommendation: MediaRecommendation) -> &'static str {
    match recommendation {
        MediaRecommendation::Comfortable => {
            "The card carries the full-quality model alongside the text engine, so scenes and \
             portraits render while you keep talking."
        }
        MediaRecommendation::Tight => {
            "Eight to twelve gigabytes fit the lighter model only. Shots take longer, the card stays \
             warm, and the first run downloads the weights before anything draws. Leave it off and \
             the story still works exactly the same."
        }
        MediaRecommendation::Limited => {
            "No card with enough memory was found, so rendering would fall back to the processor or \
             fail outright. Everything except generated art works without it."
        }
    }
}
