use uuid::Uuid;

use living_voices::backend::domain::memory::{Fact, Permanence};

#[test]
fn fact_new_is_live_with_no_provenance() {
    let adv = Uuid::new_v4();
    let fact = Fact::new(adv, "elara", "fears deep water", Permanence::Permanent);
    assert_ne!(fact.id, Uuid::nil());
    assert_eq!(fact.adventure_id, adv);
    assert_eq!(fact.subject, "elara");
    assert_eq!(fact.fact, "fears deep water");
    assert_eq!(fact.permanence, Permanence::Permanent);
    assert!(fact.is_live());
}

#[test]
fn fact_render_skips_superseded_facts() {
    let adv = Uuid::new_v4();
    let live = Fact::new(adv, "elara", "fears deep water", Permanence::Permanent);
    let mut stale = Fact::new(adv, "the lighthouse", "is abandoned", Permanence::Scene);
    stale.superseded_by = Some(Uuid::new_v4());
    let rendered = Fact::render(&[live, stale]).expect("some output");
    assert!(rendered.starts_with("\nKnown facts:"));
    assert!(rendered.contains("\n- elara: fears deep water"));
    assert!(!rendered.contains("lighthouse"));
}
