use iced::widget::{Space, button, column, container, progress_bar, row, text, toggler};
use iced::{Alignment, Color, Element, Length};

use crate::provision::{Gpu, MediaRecommendation};
use crate::ui::gate::main::{BootProgress, MessageGate};

pub fn view(
    gpu: Gpu,
    recommendation: MediaRecommendation,
    media_engine: bool,
    booting: bool,
    progress: Option<&BootProgress>,
) -> Element<'_, MessageGate> {
    let body = if booting {
        booting_body(progress)
    } else {
        question(media_engine)
    };

    container(
        column![
            text("Living Voices").size(28),
            text("Before the story starts, a word about images").size(16),
            specs(gpu, recommendation),
            text(headline(recommendation)).color(tone(recommendation)),
            text(advice(recommendation)).size(14),
            body,
        ]
        .spacing(16),
    )
    .style(container::rounded_box)
    .padding(28)
    .width(Length::Fill)
    .center_x(560)
    .into()
}

fn question(media_engine: bool) -> Element<'static, MessageGate> {
    column![
        row![
            toggler(media_engine)
                .label("Media engine (ComfyUI)")
                .on_toggle(MessageGate::MediaEngineToggled),
            Space::new().width(Length::Fill),
            button(text(if media_engine {
                "Start with images"
            } else {
                "Start without images"
            }))
            .style(button::primary)
            .on_press(MessageGate::Confirmed(media_engine)),
        ]
        .align_y(Alignment::Center)
        .spacing(16),
        text("You can revisit this later with --setup.").size(12),
    ]
    .spacing(12)
    .into()
}

fn booting_body(progress: Option<&BootProgress>) -> Element<'_, MessageGate> {
    let label = progress
        .map(|progress| progress.label.clone())
        .unwrap_or_default();
    let ratio = progress.map(|progress| progress.ratio).unwrap_or_default();

    column![
        text("Starting the engines").size(16),
        progress_bar(0.0..=1.0, ratio),
        text(label).size(12),
    ]
    .spacing(8)
    .into()
}

fn specs(gpu: Gpu, recommendation: MediaRecommendation) -> Element<'static, MessageGate> {
    container(
        column![
            row![
                text("Graphics card").size(14),
                Space::new().width(Length::Fill),
                text(gpu.card_label()).size(14),
            ],
            row![
                text("Video memory").size(14),
                Space::new().width(Length::Fill),
                text(gpu.vram_label()).size(14).color(tone(recommendation)),
            ],
        ]
        .spacing(8),
    )
    .style(container::bordered_box)
    .padding(14)
    .width(Length::Fill)
    .into()
}

fn tone(recommendation: MediaRecommendation) -> Color {
    match recommendation {
        MediaRecommendation::Comfortable => Color::from_rgb(0.42, 0.71, 0.42),
        MediaRecommendation::Tight => Color::from_rgb(1.0, 0.62, 0.25),
        MediaRecommendation::Limited => Color::from_rgb(0.75, 0.37, 0.31),
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
