use std::cell::Cell;
use std::rc::Rc;

use slint::ComponentHandle;

use crate::provision::{Gpu, MediaRecommendation};
use crate::ui::SetupWindow;

pub fn ask(gpu: &Gpu) -> Result<bool, slint::PlatformError> {
    let recommendation = gpu.media_recommendation();
    let ui = SetupWindow::new()?;

    ui.set_card_label(gpu.card_label().into());
    ui.set_memory_label(gpu.vram_label().into());
    ui.set_headline(headline(recommendation).into());
    ui.set_advice(advice(recommendation).into());
    ui.set_tone(tone(recommendation));
    ui.set_media_engine(recommendation.suggests_engine());

    let answer = Rc::new(Cell::new(None));
    let decided = answer.clone();
    ui.on_decided(move |media_engine| {
        decided.set(Some(media_engine));
        let _ = slint::quit_event_loop();
    });

    ui.run()?;

    Ok(answer.get().unwrap_or(false))
}

fn tone(recommendation: MediaRecommendation) -> i32 {
    match recommendation {
        MediaRecommendation::Comfortable => 0,
        MediaRecommendation::Tight => 1,
        MediaRecommendation::Limited => 2,
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
