#![allow(clippy::indexing_slicing)]

use serde_json::{Value, json};

use super::single_reason;

#[test]
fn active_governance_hold_excludes_completion_before_artifact_checks() {
    let snapshot = json!({
        "overall_status": "complete",
        "governance": {
            "record": "public-record",
            "action_or_classification": "authority transaction",
            "old_authority_blob_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "new_authority_blob_id": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "classification": null,
            "hold_started_at": "2026-07-10T00:00:00Z",
            "hold_ended_at": null,
            "hold_lift": null,
            "transaction_type": "addition",
            "old_authorized_fingerprints": [
                "1111111111111111111111111111111111111111"
            ],
            "new_authorized_fingerprints": [
                "1111111111111111111111111111111111111111",
                "2222222222222222222222222222222222222222"
            ],
            "approvals": [
                {"person":"a","role":"maintainer-administrator","record":"a"},
                {"person":"b","role":"architect-security","record":"b"}
            ],
            "authority_proof": {
                "state": "verified",
                "reason": null,
                "introduced_signers": [{
                    "primary_fingerprint":
                        "2222222222222222222222222222222222222222",
                    "verifier_result": "verified"
                }]
            }
        },
        "tag": Value::Null
    });
    assert_eq!(single_reason(&snapshot), Some("governance-review-or-hold"));
}
