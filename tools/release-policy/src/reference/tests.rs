#![allow(clippy::unwrap_used)]

use std::fs;

use super::{
    Classification, Executable, Inventory, PYTHON_PRIMARY, RUST_PRIMARY, Reference,
    required_markers, verify,
};

#[path = "tests/authority.rs"]
mod authority;

fn python_command() -> String {
    PYTHON_PRIMARY.command.to_owned()
}

fn rust_command() -> String {
    RUST_PRIMARY.command.to_owned()
}

fn fixture(primary: &str) -> (tempfile::TempDir, Inventory) {
    let temporary = tempfile::tempdir().unwrap();
    fs::create_dir_all(temporary.path().join("release")).unwrap();
    fs::create_dir_all(temporary.path().join("tools/release-policy")).unwrap();
    fs::create_dir_all(temporary.path().join("docs/src/contributing")).unwrap();
    fs::create_dir_all(temporary.path().join("docs/src/reference")).unwrap();
    fs::create_dir_all(temporary.path().join("rfcs")).unwrap();
    fs::write(temporary.path().join("release/check-policy.py"), "").unwrap();
    fs::write(temporary.path().join("tools/release-policy/Cargo.toml"), "").unwrap();
    let (path, command) = match primary {
        "python" => ("release/check-policy.py", python_command()),
        "rust" => ("tools/release-policy/Cargo.toml", rust_command()),
        _ => unreachable!(),
    };
    let mut references = Vec::new();
    for path in [
        "docs/src/contributing/development.md",
        "docs/src/reference/release-compatibility.md",
        "release/README.md",
    ] {
        fs::write(temporary.path().join(path), &command).unwrap();
        references.push(Reference {
            path: path.to_owned(),
            classification: Classification::LiveInvocation,
            command: command.clone(),
        });
    }
    let history = "rfcs/history.md";
    fs::write(
        temporary.path().join(history),
        ["cargo run --locked -p prikk-release-policy -- ", "check"].concat(),
    )
    .unwrap();
    references.push(Reference {
        path: history.to_owned(),
        classification: Classification::HistoricalOrExplanatory,
        command: ["cargo run --locked -p prikk-release-policy -- ", "check"].concat(),
    });
    (
        temporary,
        Inventory {
            schema_version: "release-policy-command-inventory-v1".to_owned(),
            primary_executable: Executable {
                path: path.to_owned(),
                command,
            },
            invocation_markers: required_markers(),
            references,
        },
    )
}

#[test]
fn marker_omission_and_classification_substitution_fail() {
    let (temporary, mut inventory) = fixture("python");
    inventory.invocation_markers.pop();
    assert!(
        verify(temporary.path(), &inventory)
            .unwrap()
            .iter()
            .any(|error| error == "invocation-markers")
    );

    let (temporary, mut inventory) = fixture("python");
    inventory.invocation_markers.swap(0, 1);
    assert!(
        verify(temporary.path(), &inventory)
            .unwrap()
            .iter()
            .any(|error| error == "invocation-markers")
    );

    let (temporary, mut inventory) = fixture("python");
    inventory.references.first_mut().unwrap().classification =
        Classification::HistoricalOrExplanatory;
    let errors = verify(temporary.path(), &inventory).unwrap();
    assert!(
        errors
            .iter()
            .any(|error| error.starts_with("classification:"))
    );
    assert!(
        errors
            .iter()
            .any(|error| error.starts_with("required-live-reference:"))
    );
}

#[test]
fn unregistered_python_and_rust_variants_fail() {
    for (index, marker) in [
        "python3  release/check-policy.py",
        "env python3 ./release/check-policy.py",
        "python3 \\\n  release/check-policy.py",
        "command python -B release/check-policy.py",
        "printf '#'; python3 release/check-policy.py",
        "printf \"#\"; python3 release/check-policy.py",
        ": ''#x; python3 release/check-policy.py",
        ": \"\"#x; python3 release/check-policy.py",
        "python3 -I -E -s -B ./release/check-policy.py",
        "python3 -I -E -s -B -- ./release/check-policy.py",
        "run: >-\n  python3\n  ./release/check-policy.py",
        "cargo  run --locked -p prikk-release-policy -- check",
        "env cargo run --locked -p prikk-release-policy -- check",
        "cargo \\\n  run -p prikk-release-policy -- check",
        "command cargo run -p prikk-release-policy -- check",
    ]
    .into_iter()
    .enumerate()
    {
        let (temporary, inventory) = fixture("python");
        let path = format!("docs/unregistered-{index}.md");
        fs::create_dir_all(temporary.path().join("docs")).unwrap();
        fs::write(temporary.path().join(&path), marker).unwrap();
        assert!(
            verify(temporary.path(), &inventory)
                .unwrap()
                .iter()
                .any(|error| error.starts_with(&format!("unregistered-reference:{path}:")))
        );
    }
}

#[test]
fn dynamic_rust_policy_commands_fail_closed_in_executable_files() {
    for (index, command) in [
        "$CARGO run --locked -p prikk-release-policy -- check",
        "${CARGO} run --locked -p prikk-release-policy -- check",
        "$(resolve-cargo) run --locked -p prikk-release-policy -- check",
    ]
    .into_iter()
    .enumerate()
    {
        let (temporary, inventory) = fixture("python");
        let path = format!("release/dynamic-{index}.sh");
        fs::write(temporary.path().join(&path), command).unwrap();
        assert!(
            verify(temporary.path(), &inventory)
                .unwrap()
                .iter()
                .any(|error| error.starts_with(&format!("unparseable-reference:{path}:")))
        );
    }
}

#[test]
fn genuine_comment_hides_non_executable_reference() {
    for (index, command) in [
        "printf ok # python3 release/check-policy.py\n",
        ": '' # python3 release/check-policy.py\n",
        ": \"\" # python3 release/check-policy.py\n",
    ]
    .into_iter()
    .enumerate()
    {
        let (temporary, inventory) = fixture("python");
        let path = format!("docs/commented-{index}.md");
        fs::create_dir_all(temporary.path().join("docs")).unwrap();
        fs::write(temporary.path().join(path), command).unwrap();
        assert!(verify(temporary.path(), &inventory).unwrap().is_empty());
    }
}
