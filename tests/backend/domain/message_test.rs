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
    );
    assert!(m.seq.is_none());
    assert_eq!(m.adventure_id, adv);
    assert_eq!(m.character_id, char_id);
    assert_eq!(m.speaker_name, "Alice");
    assert_eq!(m.text, "[Alice (say)]: Hello");
}

#[test]
fn message_narrator_constant_defined() {
    assert_eq!(Message::NARRATOR_SPEAKER_NAME, "NARRATOR");
}

#[test]
fn message_render_none_when_empty() {
    assert!(Message::render(&[]).is_none());
}

#[test]
fn message_render_joins_each_text_on_its_own_line() {
    let adv = Uuid::new_v4();
    let messages = vec![
        Message::new(adv, None, None, "Alice".to_string(), "[Alice (say)]: Hello"),
        Message::new(adv, None, None, "Bob".to_string(), "[Bob (say)]: Hi"),
    ];
    let rendered = Message::render(&messages).expect("some output");
    assert_eq!(rendered, "[Alice (say)]: Hello\n[Bob (say)]: Hi\n");
}
