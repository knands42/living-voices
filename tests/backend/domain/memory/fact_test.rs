use living_voices::backend::domain::memory::Fact;

#[test]
fn fact_new_populates_every_field() {
    let fact = Fact::new("elara", "fears deep water");
    assert_eq!(fact.subject, "elara");
    assert_eq!(fact.fact, "fears deep water");
}

#[test]
fn fact_render_lists_every_fact() {
    let first = Fact::new("elara", "fears deep water");
    let second = Fact::new("the lighthouse", "is abandoned");
    let rendered = Fact::render(&[first, second]).expect("some output");
    assert!(rendered.contains("\n- elara: fears deep water"));
    assert!(rendered.contains("\n- the lighthouse: is abandoned"));
}

#[test]
fn fact_render_is_none_when_empty() {
    assert!(Fact::render(&[]).is_none());
}
