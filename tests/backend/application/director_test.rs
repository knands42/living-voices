use living_voices::backend::application::director::Director;
use living_voices::backend::application::port::chat_port::Candidate;
use living_voices::backend::domain::message::{Message, MessageKind, MessagePart, MessageParts};
use living_voices::config::MessageThresholdRange;
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

fn ramp(min: u64, max: u64) -> MessageThresholdRange {
    MessageThresholdRange { min, max }
}

// A director that never ramps (min way out of reach), so we test the other
// rules in isolation.
fn no_ramp() -> Director {
    Director::new(1.0, ramp(1_000, 1_000))
}

// ---------------------------------------------------------------- who speaks

#[test]
fn narrator_speaks_when_there_is_no_cast() {
    let incoming = line(MessageKind::Say, Some("Mira"));
    assert!(no_ramp().should_narrator_speak(&[], &incoming, 0, 0.99));
}

#[test]
fn narrator_speaks_for_scene_setting_or_meta_turns() {
    let d = no_ramp();
    let candidates = cast(&["Mira"]);
    assert!(d.should_narrator_speak(&candidates, &line(MessageKind::Narration, None), 0, 0.99));
    assert!(d.should_narrator_speak(&candidates, &line(MessageKind::Ooc, None), 0, 0.99));
}

#[test]
fn characters_speak_for_an_ordinary_turn_before_the_ramp() {
    let d = no_ramp();
    let candidates = cast(&["Mira", "Kael"]);
    // Below the (unreachable) ramp min, narrator chance is 0.
    assert!(!d.should_narrator_speak(&candidates, &line(MessageKind::Say, None), 5, 0.0));
}

// ---------------------------------------------------------------- the ramp

#[test]
fn ramp_is_zero_below_min() {
    let d = Director::new(1.0, ramp(20, 30));
    let candidates = cast(&["Mira"]);
    // 19 < 20 => chance 0 => even roll 0.0 does not trigger the narrator.
    assert!(!d.should_narrator_speak(&candidates, &line(MessageKind::Say, None), 19, 0.0));
}

#[test]
fn ramp_is_certain_at_or_after_max() {
    let d = Director::new(1.0, ramp(20, 30));
    let candidates = cast(&["Mira"]);
    // 30 >= 30 => chance 1 => even roll 0.999 triggers the narrator.
    assert!(d.should_narrator_speak(&candidates, &line(MessageKind::Say, None), 30, 0.999));
    assert!(d.should_narrator_speak(&candidates, &line(MessageKind::Say, None), 42, 0.999));
}

#[test]
fn ramp_is_linear_in_the_middle() {
    let d = Director::new(1.0, ramp(20, 30));
    let candidates = cast(&["Mira"]);
    let msg = line(MessageKind::Say, None);
    // At 25 messages the span is 20..30, so chance = (25-20)/10 = 0.5.
    assert!(d.should_narrator_speak(&candidates, &msg, 25, 0.49)); // roll below 0.5 => hit
    assert!(!d.should_narrator_speak(&candidates, &msg, 25, 0.51)); // roll above 0.5 => miss
}

// ---------------------------------------------------------------- targeting

#[test]
fn a_targeted_message_picks_that_character_on_a_hit() {
    let d = Director::new(1.0, ramp(20, 30));
    let candidates = cast(&["Mira", "Kael"]);
    let incoming = line(MessageKind::Say, Some("Kael"));
    assert_eq!(
        d.pick_character(&candidates, &incoming, 0.99)
            .expect("hit")
            .name,
        "Kael"
    );
}

#[test]
fn a_targeted_message_defers_to_the_model_on_a_miss() {
    let d = Director::new(0.8, ramp(20, 30));
    let candidates = cast(&["Mira", "Kael"]);
    let incoming = line(MessageKind::Say, Some("Kael"));
    assert!(d.pick_character(&candidates, &incoming, 0.9).is_none());
    assert_eq!(
        d.pick_character(&candidates, &incoming, 0.5)
            .expect("hit")
            .name,
        "Kael"
    );
}

#[test]
fn an_untargeted_message_defers_regardless_of_chance() {
    let d = Director::new(1.0, ramp(20, 30));
    let candidates = cast(&["Mira", "Kael"]);
    let incoming = line(MessageKind::Say, None);
    assert!(d.pick_character(&candidates, &incoming, 0.0).is_none());
}
