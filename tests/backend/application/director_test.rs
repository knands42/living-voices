use living_voices::backend::application::director::Director;
use living_voices::backend::application::port::chat_port::Candidate;
use living_voices::backend::domain::message::{Message, MessageKind, MessagePart, MessageParts};
use living_voices::config::{DirectorSettings, MessageThresholdRange};
use uuid::Uuid;

fn cast(names: &[&str]) -> Vec<Candidate> {
    names
        .iter()
        .map(|n| Candidate::new((*n).to_string(), "a diver".into()))
        .collect()
}

fn line(kind: MessageKind, target: Option<&str>) -> Message {
    let mut part = MessagePart::new(kind, "hello");
    if let Some(t) = target {
        part = part.with_target(t);
    }
    Message::new(
        Uuid::new_v4(),
        None,
        None,
        "Player".to_string(),
        MessageParts::one(part),
    )
}

/// Build director settings with explicit, extreme chances so every assertion
/// is deterministic regardless of the internal random roll:
/// `roll < 0.0` is never true, `roll < 1.0` is always true for roll in [0,1).
fn director(targeted_pick_chance: f32, narrator_change_if_ooc: f32, ramp: (u64, u64)) -> Director {
    Director::new(DirectorSettings {
        targeted_pick_chance,
        narrator_change_if_ooc,
        narrator_pick_chance_after_message_threshold_range: MessageThresholdRange {
            min: ramp.0,
            max: ramp.1,
        },
    })
}

// ---------------------------------------------------------------- who speaks

#[test]
fn narrator_speaks_when_there_is_no_cast() {
    // Early return before any roll — holds for any settings.
    let d = director(1.0, 0.0, (1_000, 1_000));
    assert!(d.should_narrator_speak(&[], &line(MessageKind::Say, Some("Mira")), 0));
}

#[test]
fn narration_always_goes_to_the_narrator() {
    let d = director(1.0, 0.0, (1_000, 1_000));
    let candidates = cast(&["Mira"]);
    assert!(d.should_narrator_speak(&candidates, &line(MessageKind::Narration, None), 0));
}

#[test]
fn ooc_goes_to_the_narrator_only_when_its_chance_is_certain() {
    let candidates = cast(&["Mira"]);
    let ooc = line(MessageKind::Ooc, None);

    // chance 1.0 => roll < 1.0 always true => narrator.
    let certain = director(1.0, 1.0, (1_000, 1_000));
    assert!(certain.should_narrator_speak(&candidates, &ooc, 0));

    // chance 0.0 => roll < 0.0 never true => not the narrator.
    let never = director(1.0, 0.0, (1_000, 1_000));
    assert!(!never.should_narrator_speak(&candidates, &ooc, 0));
}

// ---------------------------------------------------------------- the ramp

#[test]
fn ramp_is_zero_below_min() {
    let d = director(1.0, 0.0, (20, 30));
    let candidates = cast(&["Mira"]);
    // 19 < 20 => chance 0 => never the narrator, whatever the roll.
    assert!(!d.should_narrator_speak(&candidates, &line(MessageKind::Say, None), 19));
}

#[test]
fn ramp_is_certain_at_or_after_max() {
    let d = director(1.0, 0.0, (20, 30));
    let candidates = cast(&["Mira"]);
    // >= max => chance 1 => always the narrator, whatever the roll.
    assert!(d.should_narrator_speak(&candidates, &line(MessageKind::Say, None), 30));
    assert!(d.should_narrator_speak(&candidates, &line(MessageKind::Say, None), 100));
}

// ---------------------------------------------------------------- targeting

#[test]
fn a_targeted_message_picks_that_character_when_chance_is_certain() {
    let d = director(1.0, 0.0, (20, 30));
    let candidates = cast(&["Mira", "Kael"]);
    let incoming = line(MessageKind::Say, Some("Kael"));
    assert_eq!(
        d.pick_character(&candidates, &incoming)
            .expect("picked")
            .name,
        "Kael"
    );
}

#[test]
fn targeting_defers_to_the_model_when_chance_is_zero() {
    let d = director(0.0, 0.0, (20, 30));
    let candidates = cast(&["Mira", "Kael"]);
    let incoming = line(MessageKind::Say, Some("Kael"));
    // chance 0 => roll < 0 never true => always defer.
    assert!(d.pick_character(&candidates, &incoming).is_none());
}

#[test]
fn an_untargeted_message_defers_even_when_chance_is_certain() {
    let d = director(1.0, 0.0, (20, 30));
    let candidates = cast(&["Mira", "Kael"]);
    let incoming = line(MessageKind::Say, None);
    // No cast member is targeted, so there is nobody to pick directly.
    assert!(d.pick_character(&candidates, &incoming).is_none());
}

#[test]
fn a_target_not_in_the_cast_defers() {
    let d = director(1.0, 0.0, (20, 30));
    let candidates = cast(&["Mira"]);
    let incoming = line(MessageKind::Say, Some("Nobody"));
    assert!(d.pick_character(&candidates, &incoming).is_none());
}
