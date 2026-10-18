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
