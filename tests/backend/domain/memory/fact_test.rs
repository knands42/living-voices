use living_voices::backend::domain::memory::{Fact, Permanence};

#[test]
fn permanence_round_trips_through_str() {
    for permanence in [Permanence::Permanent, Permanence::Plot] {
        assert_eq!(Permanence::from_str(permanence.as_str()), Some(permanence));
    }
    assert_eq!(Permanence::from_str("nonsense"), None);
}

#[test]
fn fact_new_populates_every_field() {
    let fact = Fact::new("elara", "fears deep water", Permanence::Permanent);
    assert_eq!(fact.subject, "elara");
    assert_eq!(fact.fact, "fears deep water");
    assert_eq!(fact.permanence, Permanence::Permanent);
}

#[test]
fn fact_render_lists_every_fact() {
    let first = Fact::new("elara", "fears deep water", Permanence::Permanent);
    let second = Fact::new("the lighthouse", "is abandoned", Permanence::Plot);
    let rendered = Fact::render(&[first, second]).expect("some output");
    assert!(rendered.contains("\n- elara: fears deep water -> permanent"));
    assert!(rendered.contains("\n- the lighthouse: is abandoned -> plot"));
}

#[test]
fn fact_render_is_none_when_empty() {
    assert!(Fact::render(&[]).is_none());
}
