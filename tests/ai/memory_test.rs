use living_voices::ai::jobs::memory::MemoryUpdate;

#[test]
fn parses_a_plain_json_object() {
    let reply = r#"{"facts": ["Mara hates the sea."], "summary": "Mara speaks."}"#;

    let update = MemoryUpdate::from_reply(reply).expect("parse");

    assert_eq!(update.facts, vec!["Mara hates the sea.".to_string()]);
    assert_eq!(update.summary, "Mara speaks.");
}

#[test]
fn parses_json_wrapped_in_a_code_fence() {
    let reply = "```json\n{\"facts\": [], \"summary\": \"Quiet scene.\"}\n```";

    let update = MemoryUpdate::from_reply(reply).expect("parse");

    assert!(update.facts.is_empty());
    assert_eq!(update.summary, "Quiet scene.");
}

#[test]
fn parses_json_surrounded_by_prose() {
    let reply =
        "Sure! Here is the record:\n{\"facts\": [\"A\"], \"summary\": \"B\"}\nHope that helps.";

    let update = MemoryUpdate::from_reply(reply).expect("parse");

    assert_eq!(update.facts, vec!["A".to_string()]);
    assert_eq!(update.summary, "B");
}

#[test]
fn merges_facts_and_summary_split_across_two_objects() {
    let reply = "{\"facts\": [\"Mara is a tide-singer.\", \"Mara hates the sea.\"]}\n\
                 {\"summary\": \"Mara, a tide-singer, hates the sea.\"}";

    let update = MemoryUpdate::from_reply(reply).expect("parse");

    assert_eq!(
        update.facts,
        vec![
            "Mara is a tide-singer.".to_string(),
            "Mara hates the sea.".to_string()
        ]
    );
    assert_eq!(update.summary, "Mara, a tide-singer, hates the sea.");
}

#[test]
fn ignores_braces_inside_string_values() {
    let reply = r#"{"facts": ["uses {} as a placeholder"], "summary": "brace talk"}"#;

    let update = MemoryUpdate::from_reply(reply).expect("parse");

    assert_eq!(update.facts, vec!["uses {} as a placeholder".to_string()]);
    assert_eq!(update.summary, "brace talk");
}

#[test]
fn rejects_a_reply_without_the_expected_fields() {
    let reply = r#"{"unrelated": true}"#;

    let result = MemoryUpdate::from_reply(reply);

    assert!(result.is_err());
}
