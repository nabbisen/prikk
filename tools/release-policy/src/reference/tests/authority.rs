use std::fs;

use super::{Classification, Reference, fixture, rust_command, verify};

#[test]
fn accepted_live_and_historical_contract_passes() {
    let (temporary, inventory) = fixture();
    assert!(verify(temporary.path(), &inventory).unwrap().is_empty());
}

// RFC 119 track B: this used to hold two authority descriptors (Python, Rust) and prove neither
// could stand in for the other's command. `differential-check` and the Python it invoked are
// gone, along with `PYTHON_PRIMARY` -- there is exactly one authority left, so what remains
// worth proving is that an arbitrary wrong command is rejected in both places a real command is
// checked against it.
#[test]
fn primary_executable_and_live_references_reject_wrong_command() {
    let (temporary, mut inventory) = fixture();
    inventory.primary_executable.command =
        "cargo run --locked -p prikk-release-policy -- oracle-check".to_owned();
    assert!(
        verify(temporary.path(), &inventory)
            .unwrap()
            .iter()
            .any(|error| error == "primary-executable")
    );

    let (temporary, mut inventory) = fixture();
    inventory.references.first_mut().unwrap().command =
        "cargo run --locked -p prikk-release-policy -- oracle-check".to_owned();
    let errors = verify(temporary.path(), &inventory).unwrap();
    assert!(
        errors
            .iter()
            .any(|error| error.starts_with("required-live-reference:"))
    );
    assert!(
        errors
            .iter()
            .any(|error| error.starts_with("classification:"))
    );
}

#[test]
fn unknown_missing_and_nonregular_primary_anchors_fail() {
    let (temporary, mut inventory) = fixture();
    inventory.primary_executable.path = "release/unknown.py".to_owned();
    inventory.primary_executable.command = "unknown policy command".to_owned();
    assert!(
        verify(temporary.path(), &inventory)
            .unwrap()
            .iter()
            .any(|error| error == "primary-executable")
    );

    let (temporary, inventory) = fixture();
    fs::remove_file(temporary.path().join("tools/release-policy/Cargo.toml")).unwrap();
    assert!(
        verify(temporary.path(), &inventory)
            .unwrap()
            .iter()
            .any(|error| error == "primary-executable")
    );

    let (temporary, inventory) = fixture();
    let anchor = temporary.path().join("tools/release-policy/Cargo.toml");
    fs::remove_file(&anchor).unwrap();
    fs::create_dir(&anchor).unwrap();
    assert!(
        verify(temporary.path(), &inventory)
            .unwrap()
            .iter()
            .any(|error| error == "primary-executable")
    );
}

#[test]
fn unregistered_ci_primary_invocation_fails() {
    let (temporary, inventory) = fixture();
    let path = ".github/workflows/rogue.yml";
    fs::create_dir_all(temporary.path().join(".github/workflows")).unwrap();
    fs::write(
        temporary.path().join(path),
        format!("run: {}\n", rust_command()),
    )
    .unwrap();
    assert!(
        verify(temporary.path(), &inventory)
            .unwrap()
            .iter()
            .any(|error| error.starts_with(&format!("unregistered-reference:{path}:")))
    );
}

#[test]
fn missing_duplicate_extra_and_historical_live_references_fail() {
    let (temporary, mut inventory) = fixture();
    inventory.references.remove(0);
    assert!(
        verify(temporary.path(), &inventory)
            .unwrap()
            .iter()
            .any(|error| error.starts_with("required-live-reference:"))
    );

    let (temporary, mut inventory) = fixture();
    inventory.references.push(Reference {
        path: "docs/src/contributing/development.md".to_owned(),
        classification: Classification::LiveInvocation,
        command: rust_command(),
    });
    let errors = verify(temporary.path(), &inventory).unwrap();
    assert!(errors.iter().any(|error| error == "duplicate-reference"));
    assert!(
        errors
            .iter()
            .any(|error| error.starts_with("required-live-reference:"))
    );

    let (temporary, mut inventory) = fixture();
    fs::create_dir_all(temporary.path().join("docs")).unwrap();
    fs::write(temporary.path().join("docs/extra.md"), rust_command()).unwrap();
    inventory.references.push(Reference {
        path: "docs/extra.md".to_owned(),
        classification: Classification::LiveInvocation,
        command: rust_command(),
    });
    assert!(
        verify(temporary.path(), &inventory)
            .unwrap()
            .iter()
            .any(|error| error == "classification:docs/extra.md")
    );

    let (temporary, mut inventory) = fixture();
    inventory.references.first_mut().unwrap().classification =
        Classification::HistoricalOrExplanatory;
    let errors = verify(temporary.path(), &inventory).unwrap();
    assert!(
        errors
            .iter()
            .any(|error| error.starts_with("required-live-reference:"))
    );
}
