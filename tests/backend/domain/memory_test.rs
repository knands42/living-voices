use chrono::Utc;
use uuid::Uuid;

use living_voices::backend::domain::memory::{Fact, Summary};

#[test]
fn summary_new_sets_fields() {
    // Given adventure id and span
    let adv = Uuid::new_v4();
    let before = Utc::now();
    // When creating new Summary
    let s = Summary::new(adv, "compressed".to_string(), 1u64, 5u64);
    let after = Utc::now();
    // Then id generated, fields match
    assert_ne!(s.id, Uuid::nil());
    assert_eq!(s.adventure_id, adv);
    assert_eq!(s.text, "compressed");
    assert_eq!(s.from_seq, 1);
    assert_eq!(s.to_seq, 5);
    assert!(s.created_at >= before && s.created_at <= after);
}

#[test]
fn fact_new_sets_fields() {
    // Given adventure and source summary
    let adv = Uuid::new_v4();
    let src = Uuid::new_v4();
    let before = Utc::now();
    // When creating new Fact
    let f = Fact::new(adv, src, "fact".to_string());
    let after = Utc::now();
    // Then id generated, fields match
    assert_ne!(f.id, Uuid::nil());
    assert_eq!(f.adventure_id, adv);
    assert_eq!(f.source_summary_id, src);
    assert_eq!(f.text, "fact");
    assert!(f.created_at >= before && f.created_at <= after);
}

#[test]
fn summary_render_none_when_empty() {
    // Given no summaries
    // When rendering
    // Then nothing to render
    assert!(Summary::render(&[]).is_none());
}

#[test]
fn summary_render_lists_each_summary_under_a_heading() {
    // Given two summaries
    let adv = Uuid::new_v4();
    let summaries = vec![
        Summary::new(adv, "The ship sank.".to_string(), 1, 5),
        Summary::new(adv, "Survivors reached the shore.".to_string(), 6, 9),
    ];
    // When rendering
    let rendered = Summary::render(&summaries).expect("some output");
    // Then each summary text appears as a bullet under the "Story so far" heading
    assert!(rendered.starts_with("Story so far: "));
    assert!(rendered.contains("\n- The ship sank."));
    assert!(rendered.contains("\n- Survivors reached the shore."));
}

#[test]
fn fact_render_none_when_empty() {
    // Given no facts
    // When rendering
    // Then nothing to render
    assert!(Fact::render(&[]).is_none());
}

#[test]
fn fact_render_lists_each_fact_under_a_heading() {
    // Given two facts
    let adv = Uuid::new_v4();
    let src = Uuid::new_v4();
    let facts = vec![
        Fact::new(adv, src, "Mira fears deep water.".to_string()),
        Fact::new(adv, src, "The lighthouse is abandoned.".to_string()),
    ];
    // When rendering
    let rendered = Fact::render(&facts).expect("some output");
    // Then each fact text appears as a bullet under the "Known facts" heading
    assert!(rendered.starts_with("\nKnown facts:"));
    assert!(rendered.contains("\n- Mira fears deep water."));
    assert!(rendered.contains("\n- The lighthouse is abandoned."));
}
