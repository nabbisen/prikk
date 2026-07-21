#![allow(clippy::expect_used)]

use serde_json::Value;

use super::{python_observations, run};
use crate::oracle::{ObservationDocument, Oracle};
use crate::policy;

#[test]
fn deliberate_disagreement_is_detected() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("repository root");
    let report = run(root, true).expect("differential self-test");
    assert!(report.valid);
    assert!(report.deliberate_disagreement_detected);
}

#[test]
fn missing_input_digests_fail_the_live_contract() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("repository root");
    let python_document = python_observations(root).expect("Python observations");
    let oracle = Oracle::load(root).expect("oracle");
    let rust_document = policy::evaluate(&oracle)
        .expect("Rust observations")
        .observations;
    for removal in ["python", "rust", "both"] {
        let mut python = serde_json::to_value(&python_document).expect("serialize Python");
        let mut rust = serde_json::to_value(&rust_document).expect("serialize Rust");
        if removal != "rust" {
            remove_first_digest(&mut python);
        }
        if removal != "python" {
            remove_first_digest(&mut rust);
        }
        if removal != "rust" {
            assert!(serde_json::from_value::<ObservationDocument>(python).is_err());
        }
        if removal != "python" {
            assert!(serde_json::from_value::<ObservationDocument>(rust).is_err());
        }
    }
}

fn remove_first_digest(document: &mut Value) {
    document
        .get_mut("cases")
        .and_then(Value::as_array_mut)
        .and_then(|cases| cases.first_mut())
        .and_then(Value::as_object_mut)
        .expect("first observation")
        .remove("input_digest");
}
