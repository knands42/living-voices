use chrono::Utc;
use uuid::Uuid;

use living_voices::backend::domain::message::{Message, MessageKind};

#[test]
fn messagekind_all_contains_expected_variants() {
    // Given expected kinds in the defined order
    let expected = [
        MessageKind::Narration,
        MessageKind::Say,
        MessageKind::Do,
        MessageKind::Think,
        MessageKind::Whisper,
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
    assert_eq!(MessageKind::Whisper.as_str(), "whisper");
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
    assert_eq!(MessageKind::from_str("whisper"), Some(MessageKind::Whisper));
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
    assert_eq!(MessageKind::Whisper.to_string(), "whisper");
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
        MessageKind::Say,
        "Hello".to_string(),
    );
    let after = Utc::now();
    // Then seq is None, target_name None, fields match
    assert!(m.seq.is_none());
    assert_eq!(m.adventure_id, adv);
    assert_eq!(m.character_id, char_id);
    assert_eq!(m.narrator_id, nar_id);
    assert_eq!(m.speaker_name, "Alice");
    assert_eq!(m.kind, MessageKind::Say);
    assert_eq!(m.text, "Hello");
    assert!(m.target_name.is_none());
    assert!(m.created_at >= before && m.created_at <= after);
}

#[test]
fn message_with_target_sets_target_name() {
    // Given base message
    let m = Message::new(
        Uuid::new_v4(),
        None,
        None,
        "Bob".to_string(),
        MessageKind::Whisper,
        "Pssst".to_string(),
    );
    // When adding target
    let m2 = m.with_target("Charlie".to_string());
    // Then target_name set
    assert_eq!(m2.target_name.as_deref(), Some("Charlie"));
}

#[test]
fn message_narrator_constant_defined() {
    // Given constant
    // When checking value
    // Then matches expected
    assert_eq!(Message::NARRATOR_SPEAKER_NAME, "NARRATOR");
}
