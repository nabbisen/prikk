use serde_json::json;

use super::validate;

#[test]
fn accepts_canonical_challenge_bytes() {
    let context = json!({
        "id": "valid",
        "observed_at": "2026-07-15T00:05:00Z",
        "observed_primary_fingerprint": "1111111111111111111111111111111111111111",
        "verifier_result": "verified",
        "expected_authority_revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    });
    let challenge = b"prikk-release-signer-proof-v1\nrepository=https://github.com/nabbisen/prikk\nprimary_fingerprint=1111111111111111111111111111111111111111\nrole=official-release\nauthority_revision=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\nnonce=0000000000000000000000000000000000000000000000000000000000000000\nissued_at=2026-07-15T00:00:00Z\nexpires_at=2026-07-16T00:00:00Z\n";
    assert_eq!(validate(&context, challenge), None);
}
