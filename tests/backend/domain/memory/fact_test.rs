use uuid::Uuid;

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
    let adv = Uuid::new_v4();
    let act = Uuid::new_v4();
    let fact = Fact::new(adv, "elara", "fears deep water", act, Permanence::Permanent);
    assert_ne!(fact.id, Uuid::nil());
    assert_eq!(fact.adventure_id, adv);
    assert_eq!(fact.subject, "elara");
    assert_eq!(fact.fact, "fears deep water");
    assert_eq!(fact.act_id, act);
    assert_eq!(fact.permanence, Permanence::Permanent);
}

#[test]
fn fact_render_lists_every_fact() {
    let adv = Uuid::new_v4();
    let act = Uuid::new_v4();
    let first = Fact::new(adv, "elara", "fears deep water", act, Permanence::Permanent);
    let second = Fact::new(adv, "the lighthouse", "is abandoned", act, Permanence::Plot);
    let rendered = Fact::render(&[first, second]).expect("some output");
    assert!(rendered.starts_with("\nKnown facts:"));
    assert!(rendered.contains("\n- elara: fears deep water"));
    assert!(rendered.contains("\n- the lighthouse: is abandoned"));
}

#[test]
fn fact_render_is_none_when_empty() {
    assert!(Fact::render(&[]).is_none());
}
