#![allow(clippy::unwrap_used)]

use super::run;
use crate::oracle::Oracle;

#[test]
fn accepted_oracle_passes_negative_assurance_matrix() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let oracle = Oracle::load(root).unwrap();
    assert!(run(root, &oracle).unwrap().is_empty());
}
