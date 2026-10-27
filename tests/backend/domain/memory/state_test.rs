use uuid::Uuid;

use living_voices::backend::domain::memory::State;

#[test]
fn state_new_starts_empty() {
    let adv = Uuid::new_v4();
    let state = State::new(adv);
    assert_ne!(state.id, Uuid::nil());
    assert_eq!(state.adventure_id, adv);
    assert_eq!(state.day, String::from("unknown"));
    assert_eq!(state.place, String::from("unknown"));
    assert!(state.characters_present.is_empty());
    assert!(state.open_threads.is_empty());
    assert!(state.conditions.is_empty());
    assert!(State::new(adv).render().is_some());
}

#[test]
fn state_builders_set_every_field() {
    let state = State::new(Uuid::new_v4())
        .with_day("Day 3".to_string())
        .with_place("the vault".to_string())
        .with_characters_present(vec!["Mira".to_string()])
        .with_open_threads(vec!["the sealed door".to_string()])
        .with_conditions(vec!["Mira: exhausted".to_string()]);

    assert_eq!(state.day, "Day 3");
    assert_eq!(state.place, "the vault");
    assert_eq!(state.characters_present, vec!["Mira".to_string()]);
    assert_eq!(state.open_threads, vec!["the sealed door".to_string()]);
    assert_eq!(state.conditions, vec!["Mira: exhausted".to_string()]);
}

#[test]
fn state_render_lists_set_fields() {
    let mut state = State::new(Uuid::new_v4());
    state.day = "Day 2, dusk".to_string();
    state.place = "cave mouth".to_string();
    state.characters_present = vec!["kaelen".to_string(), "elara".to_string()];
    state.open_threads = vec!["crown trail".to_string()];
    state.conditions = vec!["kaelen: left arm, minor".to_string()];
    let rendered = state.render().expect("some output");
    assert!(rendered.starts_with("Current state:\n- "));
    assert!(rendered.contains("Day: Day 2, dusk"));
    assert!(rendered.contains("Place: cave mouth"));
    assert!(rendered.contains("Present: kaelen, elara"));
    assert!(rendered.contains("Open threads: crown trail"));
    assert!(rendered.contains("Wounds: kaelen: left arm, minor"));
}
