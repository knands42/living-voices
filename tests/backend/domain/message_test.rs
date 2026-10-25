use uuid::Uuid;

use living_voices::backend::domain::message::Message;

#[test]
fn message_new_stores_text_with_seq_unset() {
    let adv = Uuid::new_v4();
    let char_id = Some(Uuid::new_v4());
    let m = Message::new(
        adv,
        char_id,
        None,
        "Alice".to_string(),
        "[Alice (say)]: Hello",
        false,
        vec![],
    );
    assert!(m.seq.is_none());
    assert_eq!(m.adventure_id, adv);
    assert_eq!(m.character_id, char_id);
    assert!(m.narrator_id.is_none());
    assert_eq!(m.speaker_name, "Alice");
    assert_eq!(m.text, "[Alice (say)]: Hello");
    assert!(!m.ooc);
    assert!(m.targeted_characters.is_empty());
}

#[test]
fn message_new_keeps_ooc_flag() {
    let adv = Uuid::new_v4();
    let m = Message::new(
        adv,
        None,
        None,
        "Alice".to_string(),
        "meta aside",
        true,
        vec![],
    );
    assert!(m.ooc);
}

#[test]
fn message_narrator_constant_defined() {
    assert_eq!(Message::NARRATOR_SPEAKER_NAME, "NARRATOR");
}
