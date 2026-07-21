#![allow(clippy::expect_used)]

use serde_json::json;

use super::SchemaProfile;

#[test]
fn rejects_unknown_vocabulary_and_nonlocal_references() {
    assert!(SchemaProfile::compile(&json!({"notImplemented": true})).is_err());
    assert!(SchemaProfile::compile(&json!({"$ref": "https://example.invalid/schema"})).is_err());
}

#[test]
fn keeps_boolean_and_integer_distinct() {
    let const_schema = SchemaProfile::compile(&json!({"const": 1})).expect("schema");
    assert!(const_schema.is_valid(&json!(1)));
    assert!(!const_schema.is_valid(&json!(true)));
    let unique =
        SchemaProfile::compile(&json!({"type": "array", "uniqueItems": true})).expect("schema");
    assert!(unique.is_valid(&json!([true, 1])));
    assert!(!unique.is_valid(&json!([1, 1.0])));
}

#[test]
fn enforces_project_date_time_profile() {
    let profile =
        SchemaProfile::compile(&json!({"type": "string", "format": "date-time"})).expect("schema");
    assert!(profile.is_valid(&json!("2026-07-17T12:34:56Z")));
    assert!(!profile.is_valid(&json!("2026-07-17T12:34:56+00:00")));
    assert!(!profile.is_valid(&json!("2026-07-17T12:34:56.0Z")));
}
