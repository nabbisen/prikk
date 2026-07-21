use std::fs;

use super::{Classification, Reference, fixture, python_command, rust_command, verify};

#[test]
fn accepted_live_and_historical_contract_passes() {
    for primary in ["python", "rust"] {
        let (temporary, inventory) = fixture(primary);
        assert!(verify(temporary.path(), &inventory).unwrap().is_empty());
    }
}

#[test]
fn primary_descriptor_pair_and_live_references_cannot_mix() {
    for primary in ["python", "rust"] {
        let (temporary, mut inventory) = fixture(primary);
        inventory.primary_executable.command = if primary == "python" {
            rust_command()
        } else {
            python_command()
        };
        assert!(
            verify(temporary.path(), &inventory)
                .unwrap()
                .iter()
                .any(|error| error == "primary-executable")
        );

        let (temporary, mut inventory) = fixture(primary);
        inventory.references.first_mut().unwrap().command = if primary == "python" {
            rust_command()
        } else {
            python_command()
        };
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
}

#[test]
fn unknown_missing_and_nonregular_primary_anchors_fail() {
    let (temporary, mut inventory) = fixture("python");
    inventory.primary_executable.path = "release/unknown.py".to_owned();
    inventory.primary_executable.command = "unknown policy command".to_owned();
    assert!(
        verify(temporary.path(), &inventory)
            .unwrap()
            .iter()
            .any(|error| error == "primary-executable")
    );

    let (temporary, inventory) = fixture("rust");
    fs::remove_file(temporary.path().join("tools/release-policy/Cargo.toml")).unwrap();
    assert!(
        verify(temporary.path(), &inventory)
            .unwrap()
            .iter()
            .any(|error| error == "primary-executable")
    );

    let (temporary, inventory) = fixture("rust");
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
    for command in [python_command(), rust_command()] {
        let (temporary, inventory) = fixture("python");
        let path = ".github/workflows/rogue.yml";
        fs::create_dir_all(temporary.path().join(".github/workflows")).unwrap();
        fs::write(temporary.path().join(path), format!("run: {command}\n")).unwrap();
        assert!(
            verify(temporary.path(), &inventory)
                .unwrap()
                .iter()
                .any(|error| error.starts_with(&format!("unregistered-reference:{path}:")))
        );
    }
}

#[test]
fn missing_duplicate_extra_and_historical_live_references_fail() {
    let (temporary, mut inventory) = fixture("python");
    inventory.references.remove(0);
    assert!(
        verify(temporary.path(), &inventory)
            .unwrap()
            .iter()
            .any(|error| error.starts_with("required-live-reference:"))
    );

    let (temporary, mut inventory) = fixture("python");
    inventory.references.push(Reference {
        path: "docs/src/contributing/development.md".to_owned(),
        classification: Classification::LiveInvocation,
        command: python_command(),
    });
    let errors = verify(temporary.path(), &inventory).unwrap();
    assert!(errors.iter().any(|error| error == "duplicate-reference"));
    assert!(
        errors
            .iter()
            .any(|error| error.starts_with("required-live-reference:"))
    );

    let (temporary, mut inventory) = fixture("python");
    fs::create_dir_all(temporary.path().join("docs")).unwrap();
    fs::write(temporary.path().join("docs/extra.md"), python_command()).unwrap();
    inventory.references.push(Reference {
        path: "docs/extra.md".to_owned(),
        classification: Classification::LiveInvocation,
        command: python_command(),
    });
    assert!(
        verify(temporary.path(), &inventory)
            .unwrap()
            .iter()
            .any(|error| error == "classification:docs/extra.md")
    );

    let (temporary, mut inventory) = fixture("python");
    inventory.references.first_mut().unwrap().classification =
        Classification::HistoricalOrExplanatory;
    let errors = verify(temporary.path(), &inventory).unwrap();
    assert!(
        errors
            .iter()
            .any(|error| error.starts_with("required-live-reference:"))
    );
}
