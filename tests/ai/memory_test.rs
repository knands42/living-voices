use living_voices::ai::integrations::memory::{MemoryUpdate, Permanence};

#[test]
fn parses_the_schema_shaped_object() {
    let reply = r#"{
        "facts": [
            {"subject": "Mara", "fact": "Mara hates the sea.", "permanence": "permanent"}
        ],
        "state": {
            "day": "Day 1, noon",
            "place": "the drowned quarter",
            "characters_present": ["Mara"],
            "open_threads": ["why Mara left the sea"],
            "conditions": ["mood: wary"]
        },
        "running_digest": "Mara arrived in the drowned quarter and admitted she hates the sea."
    }"#;

    let update = MemoryUpdate::from_reply(reply).expect("parse");

    assert_eq!(update.facts.len(), 1);
    assert_eq!(update.facts[0].subject, "Mara");
    assert_eq!(update.facts[0].fact, "Mara hates the sea.");
    assert_eq!(update.facts[0].permanence, Permanence::Permanent);
    assert_eq!(update.state.day.as_deref(), Some("Day 1, noon"));
    assert_eq!(update.state.place.as_deref(), Some("the drowned quarter"));
    assert_eq!(update.state.characters_present, vec!["Mara".to_string()]);
    assert!(update.running_digest.contains("drowned quarter"));
}

#[test]
fn tolerates_leading_and_trailing_whitespace_and_empty_collections() {
    let reply = "\n  {\"facts\": [], \"state\": {\"day\": null, \"place\": null, \
        \"characters_present\": [], \"open_threads\": [], \"conditions\": []}}  \n";

    let update = MemoryUpdate::from_reply(reply).expect("parse");

    assert!(update.facts.is_empty());
    assert!(update.state.day.is_none());
    assert!(update.state.place.is_none());
    assert!(update.state.characters_present.is_empty());
    assert!(update.state.open_threads.is_empty());
    assert!(update.state.conditions.is_empty());
}

#[test]
fn rejects_a_reply_without_the_expected_fields() {
    let reply = r#"{"unrelated": true}"#;

    assert!(MemoryUpdate::from_reply(reply).is_err());
}

#[test]
fn rejects_an_unknown_permanence() {
    let reply = r#"{
        "facts": [{"subject": "Mara", "fact": "x", "permanence": "forever"}],
        "state": {"day": null, "place": null, "characters_present": [],
                  "open_threads": [], "conditions": []}
    }"#;

    assert!(MemoryUpdate::from_reply(reply).is_err());
}

#[test]
fn rejects_prose_instead_of_json() {
    let reply = "Sure! Here is the record you asked for.";

    assert!(MemoryUpdate::from_reply(reply).is_err());
}
