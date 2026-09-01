#![allow(clippy::unwrap_used)]

use super::generate;

#[test]
fn writes_both_scripts_with_the_placeholder_substituted() {
    let temporary = tempfile::tempdir().unwrap();
    generate(temporary.path()).unwrap();

    let install = std::fs::read_to_string(temporary.path().join("install.sh")).unwrap();
    let uninstall = std::fs::read_to_string(temporary.path().join("uninstall.sh")).unwrap();

    assert!(install.starts_with("#!/bin/sh"));
    assert!(uninstall.starts_with("#!/bin/sh"));
    assert!(install.contains("prikk-vcs/prikk"));
    assert!(uninstall.contains("prikk-vcs/prikk"));
    assert!(
        !install.contains("REPO_SLUG"),
        "the placeholder must not survive substitution"
    );
    assert!(
        !uninstall.contains("REPO_SLUG"),
        "the placeholder must not survive substitution"
    );
}

#[cfg(unix)]
#[test]
fn generated_scripts_are_executable() {
    use std::os::unix::fs::PermissionsExt;

    let temporary = tempfile::tempdir().unwrap();
    generate(temporary.path()).unwrap();

    for name in ["install.sh", "uninstall.sh"] {
        let mode = std::fs::metadata(temporary.path().join(name))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(
            mode & 0o111,
            0o111,
            "{name} must be executable by owner, group, and other"
        );
    }
}

#[test]
fn creates_the_output_directory_if_missing() {
    let temporary = tempfile::tempdir().unwrap();
    let nested = temporary.path().join("does-not-exist-yet");
    generate(&nested).unwrap();
    assert!(nested.join("install.sh").is_file());
}
