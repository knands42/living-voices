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
    assert_eq!(act.from_seq, 0);
    assert_eq!(act.to_seq, 0);
    assert_eq!(act.created_at, act.updated_at);
}

#[test]
fn act_close_records_index_digest_and_span() {
    let mut act = Act::new(Uuid::new_v4(), 1, "recover the crown");
    act.close("ACT 1 Crown: recovered", "They found the crown.", 1, 420);
    assert_eq!(act.index_line.as_deref(), Some("ACT 1 Crown: recovered"));
    assert_eq!(act.digest.as_deref(), Some("They found the crown."));
    assert_eq!(act.from_seq, 1);
    assert_eq!(act.to_seq, 420);
}

#[test]
fn act_builders_set_trigger_and_status() {
    let act = Act::new(Uuid::new_v4(), 1, "recover the crown")
        .with_trigger("the king dies")
        .with_status(ActStatus::Active);
    assert_eq!(act.trigger.as_deref(), Some("the king dies"));
    assert_eq!(act.status, ActStatus::Active);
}

#[test]
fn act_status_round_trips_through_str() {
    for status in ActStatus::ALL {
        assert_eq!(ActStatus::from_str(&status.to_string()), Some(status));
    }
    assert_eq!(ActStatus::from_str("nonsense"), None);
}

#[test]
fn render_index_without_a_summary_shows_goal_and_status() {
    let act = Act::new(Uuid::new_v4(), 2, "cross the pass").with_status(ActStatus::Active);
    let rendered = act.render_index();
    assert!(rendered.contains("Act 2"));
    assert!(rendered.contains("goal: cross the pass"));
    assert!(rendered.contains("status: active"));
    assert!(!rendered.contains("summary:"));
}

#[test]
fn render_index_includes_the_summary_once_closed() {
    let mut act = Act::new(Uuid::new_v4(), 2, "cross the pass");
    act.close("ACT 2: crossed", "They crossed.", 1, 10);
    let rendered = act.render_index();
    assert!(rendered.contains("summary: ACT 2: crossed"));
}

#[test]
fn render_running_digest_is_empty_until_set() {
    let mut act = Act::new(Uuid::new_v4(), 1, "recover the crown");
    assert_eq!(act.render_running_digest(), "");
    act.running_digest = Some("So far they fled the castle.".to_string());
    assert_eq!(act.render_running_digest(), "So far they fled the castle.");
}

#[test]
fn render_digest_lists_only_closed_acts() {
    let adv = Uuid::new_v4();
    let mut first = Act::new(adv, 1, "escape");
    first.close("ACT 1: escaped", "They escaped the keep.", 1, 100);
    let open = Act::new(adv, 2, "regroup").with_status(ActStatus::Active);

    let rendered = Act::render_digest(&[&first, &open]);
    assert!(rendered.contains("Act 1"));
    assert!(rendered.contains("digest: They escaped the keep."));
    // The open act has no digest, so it is omitted.
    assert!(!rendered.contains("Act 2"));
}

#[test]
fn render_digest_is_empty_when_no_act_is_closed() {
    let act = Act::new(Uuid::new_v4(), 1, "begin");
    assert_eq!(Act::render_digest(&[&act]), "");
}
