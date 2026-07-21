#![allow(clippy::expect_used)]

use super::Oracle;

#[test]
fn loads_frozen_repository_oracle() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("repository root");
    let oracle = Oracle::load(root).expect("frozen oracle must verify");
    assert_eq!(oracle.manifest.cases.len(), 154);
}
