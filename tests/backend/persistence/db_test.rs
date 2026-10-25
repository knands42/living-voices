use chrono::{TimeZone, Utc};
use uuid::Uuid;

use living_voices::backend::persistence::db::{
    from_str_to_timestamp, from_str_to_uuid, from_timestamp_to_str, parse_json_array, to_json_array,
};

#[test]
fn timestamp_round_trips_through_rfc3339() {
    let at = Utc.with_ymd_and_hms(2026, 10, 25, 16, 7, 21).unwrap();
    let text = from_timestamp_to_str(&at);
    let back = from_str_to_timestamp(&text).expect("parse");
    assert_eq!(back, at);
}

#[test]
fn from_str_to_timestamp_rejects_garbage() {
    assert!(from_str_to_timestamp("not a date").is_err());
}

#[test]
fn from_str_to_uuid_parses_a_valid_uuid() {
    let id = Uuid::new_v4();
    assert_eq!(from_str_to_uuid(&id.to_string()).expect("parse"), id);
}

#[test]
fn from_str_to_uuid_rejects_garbage() {
    assert!(from_str_to_uuid("not-a-uuid").is_err());
}

#[test]
fn parse_json_array_treats_none_and_empty_as_empty() {
    assert_eq!(parse_json_array(None).expect("none"), Vec::<String>::new());
    assert_eq!(
        parse_json_array(Some("")).expect("empty"),
        Vec::<String>::new()
    );
    assert_eq!(
        parse_json_array(Some("   ")).expect("blank"),
        Vec::<String>::new()
    );
}

#[test]
fn parse_json_array_decodes_a_json_list() {
    assert_eq!(
        parse_json_array(Some(r#"["kaelen","elara"]"#)).expect("decode"),
        vec!["kaelen".to_string(), "elara".to_string()]
    );
}

#[test]
fn parse_json_array_rejects_invalid_json() {
    assert!(parse_json_array(Some("{not json}")).is_err());
}

#[test]
fn to_json_array_maps_empty_to_none() {
    assert_eq!(to_json_array(&[]).expect("empty"), None);
}

#[test]
fn to_json_array_encodes_a_list() {
    let encoded = to_json_array(&["a".to_string(), "b".to_string()]).expect("encode");
    assert_eq!(encoded.as_deref(), Some(r#"["a","b"]"#));
}

#[test]
fn json_array_round_trips() {
    let values = vec!["crown trail".to_string(), "blood on the stones".to_string()];
    let encoded = to_json_array(&values).expect("encode");
    let decoded = parse_json_array(encoded.as_deref()).expect("decode");
    assert_eq!(decoded, values);
}
