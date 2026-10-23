use uuid::Uuid;

use living_voices::backend::domain::memory::{Act, ActStatus};

#[test]
fn act_new_starts_unstarted_with_no_span() {
    let adv = Uuid::new_v4();
    let act = Act::new(adv, 1, "recover the crown");
    assert_ne!(act.id, Uuid::nil());
    assert_eq!(act.adventure_id, adv);
    assert_eq!(act.ordinal, 1);
    assert_eq!(act.goal, "recover the crown");
    assert_eq!(act.status, ActStatus::Unstarted);
    assert!(act.from_seq.is_none());
    assert!(act.to_seq.is_none());
    assert_eq!(act.created_at, act.updated_at);
}

#[test]
fn act_close_records_index_digest_and_span() {
    let mut act = Act::new(Uuid::new_v4(), 1, "recover the crown");
    act.close("ACT 1 Crown: recovered", "They found the crown.", 1, 420);
    assert_eq!(act.index_line.as_deref(), Some("ACT 1 Crown: recovered"));
    assert_eq!(act.digest.as_deref(), Some("They found the crown."));
    assert_eq!(act.from_seq, Some(1));
    assert_eq!(act.to_seq, Some(420));
}
