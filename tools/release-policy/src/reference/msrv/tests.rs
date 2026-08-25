#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::Path;

use super::check;

fn write_cargo_toml(root: &Path, rust_version: &str) {
    fs::write(
        root.join("Cargo.toml"),
        format!("[workspace.package]\nrust-version = \"{rust_version}\"\n"),
    )
    .unwrap();
}

fn write_ci_workflow(root: &Path, job_name_version: &str, toolchain_pin: &str) {
    fs::write(
        root.join(".github/workflows/ci.yml"),
        format!(
            "jobs:\n  msrv:\n    name: msrv-{job_name_version}\n    steps:\n      - uses: dtolnay/rust-toolchain@{toolchain_pin}\n"
        ),
    )
    .unwrap();
}

/// A CI workflow whose job carries no `name:` line at all -- proves the "marker absent" path,
/// distinct from "marker present but wrong".
fn write_ci_workflow_without_job_name(root: &Path, toolchain_pin: &str) {
    fs::write(
        root.join(".github/workflows/ci.yml"),
        format!(
            "jobs:\n  msrv:\n    steps:\n      - uses: dtolnay/rust-toolchain@{toolchain_pin}\n"
        ),
    )
    .unwrap();
}

fn write_development_guide(root: &Path, prose_version: &str, command_version: &str) {
    fs::write(
        root.join("docs/src/contributing/development.md"),
        format!(
            "The workspace declares Rust {prose_version} as its minimum supported version.\n\n\
             cargo +{command_version} check --workspace --all-targets --locked\n\
             cargo +{command_version} test --workspace --locked\n\
             cargo +{command_version} build --workspace --locked\n"
        ),
    )
    .unwrap();
}

fn write_release_compatibility(root: &Path, prose_version: &str, command_version: &str) {
    fs::write(
        root.join("docs/src/reference/release-compatibility.md"),
        format!(
            "The workspace's declared minimum Rust version is exactly {prose_version}.\n\n\
             cargo +{command_version} check --workspace --all-targets --locked\n\
             cargo +{command_version} test --workspace --locked\n\
             cargo +{command_version} build --workspace --locked\n"
        ),
    )
    .unwrap();
}

fn write_execution_order(root: &Path, command_version: &str) {
    fs::write(
        root.join("rfcs/EXECUTION-ORDER.md"),
        format!("9. Gate set: `cargo +{command_version} test --workspace --locked`; `git diff --check`;\n"),
    )
    .unwrap();
}

/// A repository-shaped fixture where every one of the six live sites (`msrv.rs::MARKERS` plus the
/// `Cargo.toml` authority) agrees: `1.85` in the manifest, `1.85.0` everywhere else.
fn healthy_fixture() -> tempfile::TempDir {
    let temporary = tempfile::tempdir().unwrap();
    fs::create_dir_all(temporary.path().join(".github/workflows")).unwrap();
    fs::create_dir_all(temporary.path().join("docs/src/contributing")).unwrap();
    fs::create_dir_all(temporary.path().join("docs/src/reference")).unwrap();
    fs::create_dir_all(temporary.path().join("rfcs")).unwrap();
    write_cargo_toml(temporary.path(), "1.85");
    write_ci_workflow(temporary.path(), "1.85.0", "1.85.0");
    write_development_guide(temporary.path(), "1.85", "1.85.0");
    write_release_compatibility(temporary.path(), "1.85.0", "1.85.0");
    write_execution_order(temporary.path(), "1.85.0");
    temporary
}

#[test]
fn healthy_repository_passes_clean() {
    let temporary = healthy_fixture();
    let mut errors = Vec::new();
    check(temporary.path(), &mut errors).unwrap();
    assert!(errors.is_empty(), "{errors:?}");
}

/// Control 1: raising the MSRV for real must name every site still holding the old value, not
/// just the first.
#[test]
fn a_raised_msrv_names_every_stale_site() {
    let temporary = healthy_fixture();
    write_cargo_toml(temporary.path(), "1.90");
    let mut errors = Vec::new();
    check(temporary.path(), &mut errors).unwrap();

    let expected_stale = [
        "msrv-transcription-mismatch:.github/workflows/ci.yml:toolchain-pin:1.85.0",
        "msrv-transcription-mismatch:.github/workflows/ci.yml:job-name:1.85.0",
        "msrv-transcription-mismatch:docs/src/contributing/development.md:prose:1.85",
        "msrv-transcription-mismatch:docs/src/reference/release-compatibility.md:prose:1.85.0",
        "msrv-transcription-mismatch:rfcs/EXECUTION-ORDER.md:gate-command:1.85.0",
    ];
    for expected in expected_stale {
        assert!(
            errors.iter().any(|error| error == expected),
            "expected {expected:?} in {errors:?}"
        );
    }
    // development.md and release-compatibility.md each carry three identical `cargo +1.85.0`
    // command lines -- every one is reported independently, not deduplicated to a single finding,
    // matching "report all mismatches, not the first" (handoff §3).
    let gate_command_mismatches = errors
        .iter()
        .filter(|error| error.ends_with(":gate-command:1.85.0"))
        .count();
    assert_eq!(
        gate_command_mismatches, 7,
        "3 (development.md) + 3 (release-compatibility.md) + 1 (EXECUTION-ORDER.md): {errors:?}"
    );
    // toolchain-pin (1) + job-name (1) + development.md prose (1) + release-compatibility.md
    // prose (1) + the 7 gate-command mismatches counted above = 11.
    assert_eq!(errors.len(), 11, "{errors:?}");
}

/// Control 2: a single drifted transcription -- the CI toolchain pin, the highest-value one -- is
/// caught while every other site still agrees with the (unchanged) authority.
#[test]
fn a_single_drifted_site_is_caught_alone() {
    let temporary = healthy_fixture();
    write_ci_workflow(temporary.path(), "1.85.0", "1.86.0");
    let mut errors = Vec::new();
    check(temporary.path(), &mut errors).unwrap();
    assert_eq!(
        errors,
        vec!["msrv-transcription-mismatch:.github/workflows/ci.yml:toolchain-pin:1.86.0"],
        "exactly one site drifted, so exactly one error: {errors:?}"
    );
}

/// Control 3: `"1.85"` (manifest) and `"1.85.0"` (every other site) is not a mismatch --
/// `healthy_repository_passes_clean` already proves that half. This proves the other half: a
/// genuine mismatch (`1.85` vs `1.86.0`) is still caught, so the normalization is a real semantic
/// comparison, not a substring check that would accept anything starting with `1.85`.
#[test]
fn spelling_normalization_accepts_the_documented_forms_and_rejects_a_real_mismatch() {
    let temporary = healthy_fixture();
    write_development_guide(temporary.path(), "1.86", "1.85.0");
    let mut errors = Vec::new();
    check(temporary.path(), &mut errors).unwrap();
    assert_eq!(
        errors,
        vec!["msrv-transcription-mismatch:docs/src/contributing/development.md:prose:1.86"],
        "{errors:?}"
    );
}

/// Control 4: `msrv::check` only ever opens the six named live sites, by construction -- it has no
/// directory walk and no path derived from file content. A historical document sitting right next
/// to the live ones, holding a stale version, cannot become a failure no matter what it says,
/// because nothing ever reads it.
#[test]
fn a_historical_document_with_a_stale_version_is_never_read() {
    let temporary = healthy_fixture();
    fs::write(
        temporary.path().join("MILESTONES.md"),
        "Rust 1.70 compatibility was the floor at the time.\n",
    )
    .unwrap();
    let mut errors = Vec::new();
    check(temporary.path(), &mut errors).unwrap();
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn a_removed_marker_is_reported_as_missing_not_as_a_pass() {
    let temporary = healthy_fixture();
    write_ci_workflow_without_job_name(temporary.path(), "1.85.0");
    let mut errors = Vec::new();
    check(temporary.path(), &mut errors).unwrap();
    assert_eq!(
        errors,
        vec!["msrv-transcription-missing:.github/workflows/ci.yml:job-name"],
        "{errors:?}"
    );
}
