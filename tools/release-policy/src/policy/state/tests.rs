#![allow(clippy::expect_used)]

use serde_json::json;

use super::classify;

#[test]
fn classifies_only_exact_release_lifecycle_rows() {
    let value = json!({
        "workspace": "last-release",
        "latest": "last-release",
        "candidate": null,
        "changelog": "no-target-claim",
        "rfc": "proposed-or-accepted",
        "tag": "absent-at-head",
        "distribution": "pending"
    });
    assert_eq!(
        classify(value.as_object().expect("object")),
        Some("development")
    );
}
