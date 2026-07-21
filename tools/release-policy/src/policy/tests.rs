#![allow(clippy::expect_used)]

use super::{compare_expected, evaluate};
use crate::oracle::Oracle;

#[test]
fn rust_policy_matches_frozen_expectations() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("repository root");
    let oracle = Oracle::load(root).expect("oracle");
    let output = evaluate(&oracle).expect("Rust policy evaluation");
    compare_expected(&oracle, &output).expect("frozen expectations");
}
