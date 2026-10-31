use living_voices::backend::domain::lore::Lore;
use uuid::Uuid;

#[test]
fn new_starts_with_empty_text() {
    let lore = Lore::new(Uuid::new_v4(), "");

    assert_eq!(lore.text, "");
}

#[test]
fn new_generates_unique_ids() {
    assert_ne!(
        Lore::new(Uuid::new_v4(), "").id,
        Lore::new(Uuid::new_v4(), "").id
    );
}

#[test]
fn with_text_replaces_text() {
    let lore = Lore::new(Uuid::new_v4(), "").with_text("The sea remembers every name.".to_string());

    assert_eq!(lore.text, "The sea remembers every name.");
}
