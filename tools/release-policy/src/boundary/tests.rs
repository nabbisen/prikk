#![allow(clippy::expect_used)]

use super::run;

#[test]
fn workspace_and_product_boundaries_hold() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("repository root");
    let report = run(root).expect("boundary check");
    assert!(report.valid);
}
