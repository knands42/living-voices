use living_voices::backend::domain::lore::Lore;

#[test]
fn new_starts_with_empty_text() {
    let lore = Lore::new();

    assert_eq!(lore.text, "");
}

#[test]
fn new_generates_unique_ids() {
    assert_ne!(Lore::new().id, Lore::new().id);
}

#[test]
fn with_text_replaces_text() {
    let lore = Lore::new().with_text("The sea remembers every name.".to_string());

    assert_eq!(lore.text, "The sea remembers every name.");
}
