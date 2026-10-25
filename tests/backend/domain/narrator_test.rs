use living_voices::backend::domain::narrator::Narrator;
use uuid::Uuid;

fn new_narrator() -> Narrator {
    Narrator::new("The Chronicler".to_string(), None)
}

#[test]
fn new_sets_name_and_leaves_tone_unset() {
    let narrator = new_narrator();

    assert_eq!(narrator.name, "The Chronicler");
    assert!(narrator.tone.is_none());
    assert_ne!(narrator.id, Uuid::nil());
}

#[test]
fn new_keeps_the_given_tone() {
    let narrator = Narrator::new("The Chronicler".to_string(), Some("grim".to_string()));

    assert_eq!(narrator.tone.as_deref(), Some("grim"));
}

#[test]
fn with_name_replaces_the_name_and_keeps_the_id() {
    let narrator = new_narrator();
    let id = narrator.id;

    let renamed = narrator.with_name("The Witness".to_string());

    assert_eq!(renamed.name, "The Witness");
    assert_eq!(renamed.id, id);
}

#[test]
fn new_generates_unique_ids() {
    assert_ne!(new_narrator().id, new_narrator().id);
}
