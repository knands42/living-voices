use living_voices::ai::jobs::memory::MemoryUpdate;

#[test]
fn parses_the_schema_shaped_object() {
    let reply = r#"{"facts": ["Mara hates the sea."], "summary": "Mara speaks."}"#;

    let update = MemoryUpdate::from_reply(reply).expect("parse");

    assert_eq!(update.facts, vec!["Mara hates the sea.".to_string()]);
    assert_eq!(update.summary, "Mara speaks.");
}

#[test]
fn tolerates_leading_and_trailing_whitespace() {
    let reply = "\n  {\"facts\": [], \"summary\": \"Quiet scene.\"}  \n";

    let update = MemoryUpdate::from_reply(reply).expect("parse");

    assert!(update.facts.is_empty());
    assert_eq!(update.summary, "Quiet scene.");
}

#[test]
fn rejects_a_reply_without_the_expected_fields() {
    let reply = r#"{"unrelated": true}"#;

    assert!(MemoryUpdate::from_reply(reply).is_err());
}

#[test]
fn rejects_prose_instead_of_json() {
    let reply = "Sure! Here is the record you asked for.";

    assert!(MemoryUpdate::from_reply(reply).is_err());
}
