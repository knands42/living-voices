use chrono::Utc;
use uuid::Uuid;

use living_voices::backend::domain::message::{Message, MessageKind, MessagePart, MessageParts};

#[test]
fn messagekind_all_contains_expected_variants() {
    // Given expected kinds in the defined order
    let expected = [
        MessageKind::Narration,
        MessageKind::Say,
        MessageKind::Do,
        MessageKind::Think,
        MessageKind::Ooc,
    ];
    // When checking ALL
    // Then ALL matches expected
    assert_eq!(MessageKind::ALL, expected);
}

#[test]
fn messagekind_as_str_returns_correct_labels() {
    // Given each kind
    // When converting to str
    // Then labels match domain vocabulary
    assert_eq!(MessageKind::Narration.as_str(), "narration");
    assert_eq!(MessageKind::Say.as_str(), "say");
    assert_eq!(MessageKind::Do.as_str(), "do");
    assert_eq!(MessageKind::Think.as_str(), "think");
    assert_eq!(MessageKind::Ooc.as_str(), "ooc");
}

#[test]
fn messagekind_from_str_parses_labels() {
    // Given valid labels
    // When parsing
    // Then corresponding kinds returned
    assert_eq!(
        MessageKind::from_str("narration"),
        Some(MessageKind::Narration)
    );
    assert_eq!(MessageKind::from_str("say"), Some(MessageKind::Say));
    assert_eq!(MessageKind::from_str("do"), Some(MessageKind::Do));
    assert_eq!(MessageKind::from_str("think"), Some(MessageKind::Think));
    assert_eq!(MessageKind::from_str("ooc"), Some(MessageKind::Ooc));
}

#[test]
fn messagekind_from_str_returns_none_for_unknown() {
    // Given unknown label
    // When parsing
    // Then None returned
    assert_eq!(MessageKind::from_str("unknown"), None);
    assert_eq!(MessageKind::from_str("SAY"), None);
    assert_eq!(MessageKind::from_str(""), None);
}

#[test]
fn messagekind_display_formats_as_str() {
    // Given kind
    // When formatting
    // Then uses as_str value
    assert_eq!(MessageKind::Ooc.to_string(), "ooc");
    assert_eq!(MessageKind::Narration.to_string(), "narration");
}

#[test]
fn messagekind_default_is_say() {
    // Given default
    // When created
    // Then default is Say
    assert_eq!(MessageKind::default(), MessageKind::Say);
}

#[test]
fn message_new_sets_defaults() {
    // Given identifiers and content
    let adv = Uuid::new_v4();
    let char_id = Some(Uuid::new_v4());
    let nar_id = Some(Uuid::new_v4());
    let before = Utc::now();
    // When creating new Message
    let m = Message::new(
        adv,
        char_id,
        nar_id,
        "Alice".to_string(),
        MessageParts::one(MessagePart::new(MessageKind::Say, "Hello")),
    );
    let after = Utc::now();
    // Then seq is None, parts hold the single Say part, fields match
    assert!(m.seq.is_none());
    assert_eq!(m.adventure_id, adv);
    assert_eq!(m.character_id, char_id);
    assert_eq!(m.narrator_id, nar_id);
    assert_eq!(m.speaker_name, "Alice");
    assert_eq!(m.parts.primary().kind, MessageKind::Say);
    assert_eq!(m.parts.primary().text, "Hello");
    assert!(m.parts.primary().target_name.is_none());
    assert!(m.parts.secondary().is_none());
    assert!(m.created_at >= before && m.created_at <= after);
}

#[test]
fn message_part_with_target_sets_target_name() {
    // Given a part without target
    // When adding a target
    let part = MessagePart::new(MessageKind::Do, "raise the drawing").with_target("Charlie");
    // Then target_name set — `do` allows a target: an action can be done to someone
    assert_eq!(part.target_name.as_deref(), Some("Charlie"));
}

#[test]
fn message_parts_one_yields_single_part() {
    // Given a single-part message
    let parts = MessageParts::one(MessagePart::new(MessageKind::Do, "open the door"));
    // When iterating
    // Then exactly one part is yielded
    let kinds: Vec<MessageKind> = parts.iter().map(|p| p.kind).collect();
    assert_eq!(kinds, [MessageKind::Do]);
    assert!(parts.secondary().is_none());
}

#[test]
fn message_parts_two_keeps_both_parts_in_order() {
    // Given two parts: a targeted whisper (say -> target) while drawing
    let parts = MessageParts::two(
        MessagePart::new(MessageKind::Say, "come here").with_target("Charlie"),
        MessagePart::new(MessageKind::Do, "raise the drawing"),
    );
    // When iterating
    let collected: Vec<&MessagePart> = parts.iter().collect();
    // Then both parts are yielded in order, each with its own text
    assert_eq!(collected.len(), 2);
    assert_eq!(collected[0].kind, MessageKind::Say);
    assert_eq!(collected[0].text, "come here");
    assert_eq!(collected[0].target_name.as_deref(), Some("Charlie"));
    assert_eq!(collected[1].kind, MessageKind::Do);
    assert_eq!(collected[1].text, "raise the drawing");
    assert!(collected[1].target_name.is_none());
}

#[test]
fn message_narrator_constant_defined() {
    // Given constant
    // When checking value
    // Then matches expected
    assert_eq!(Message::NARRATOR_SPEAKER_NAME, "NARRATOR");
}
