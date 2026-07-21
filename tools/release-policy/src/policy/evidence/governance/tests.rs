#![allow(clippy::indexing_slicing)]

use serde_json::{Value, json};

use super::{progression_reason, reason};

fn governance() -> Value {
    json!({
        "record": "public-record",
        "action_or_classification": "authority transaction",
        "old_authority_blob_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "new_authority_blob_id": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "classification": null,
        "hold_started_at": "2026-07-10T00:00:00Z",
        "hold_ended_at": "2026-07-13T00:00:00Z",
        "hold_lift": {"record":"public-lift","ruling":"lift-approved"},
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
    })
}

#[test]
fn hold_lift_requires_full_72_hour_boundary() {
    let exact = governance();
    assert_eq!(reason(Some(&exact)), None);
    let mut early = governance();
    early["hold_ended_at"] = json!("2026-07-12T23:59:59Z");
    assert_eq!(reason(Some(&early)), Some("governance-review-or-hold"));
}

#[test]
fn progression_only_fills_reviewed_nullable_fields() {
    let mut active = governance();
    active["hold_ended_at"] = Value::Null;
    active["hold_lift"] = Value::Null;
    let lifted = governance();
    assert_eq!(progression_reason(Some(&active), Some(&lifted)), None);

    let mut mutated = lifted;
    mutated["record"] = json!("different");
    assert_eq!(
        progression_reason(Some(&active), Some(&mutated)),
        Some("governance-transition-or-proof")
    );
    assert_eq!(
        progression_reason(Some(&active), None),
        Some("governance-transition-or-proof")
    );
}
