use uuid::Uuid;

use living_voices::backend::domain::memory::State;

#[test]
fn state_new_starts_empty() {
    let adv = Uuid::new_v4();
    let state = State::new(adv);
    assert_ne!(state.id, Uuid::nil());
    assert_eq!(state.adventure_id, adv);
    assert!(state.day.is_none());
    assert!(state.place.is_none());
    assert!(state.present.is_empty());
    assert!(state.open_threads.is_empty());
    assert!(state.wounds.is_empty());
    assert!(State::new(adv).render().is_none());
}

#[test]
fn state_render_lists_set_fields() {
    let mut state = State::new(Uuid::new_v4());
    state.day = Some("Day 2, dusk".to_string());
    state.place = Some("cave mouth".to_string());
    state.present = vec!["kaelen".to_string(), "elara".to_string()];
    state.open_threads = vec!["crown trail".to_string()];
    state.wounds = vec!["kaelen: left arm, minor".to_string()];
    let rendered = state.render().expect("some output");
    assert!(rendered.starts_with("Current state:\n- "));
    assert!(rendered.contains("Day: Day 2, dusk"));
    assert!(rendered.contains("Place: cave mouth"));
    assert!(rendered.contains("Present: kaelen, elara"));
    assert!(rendered.contains("Open threads: crown trail"));
    assert!(rendered.contains("Wounds: kaelen: left arm, minor"));
}
