#![allow(clippy::expect_used)]

use super::Oracle;

#[test]
fn loads_frozen_repository_oracle() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("repository root");
    let oracle = Oracle::load(root).expect("frozen oracle must verify");
    // RFC 119 track A: 154 -> 111 when the 43 post-1.0 signer cases were parked (not deleted; see
    // release/oracle/parked-cases-v1.json).
    assert_eq!(oracle.manifest.cases.len(), 111);
}
