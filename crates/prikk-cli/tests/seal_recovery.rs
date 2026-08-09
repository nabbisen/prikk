//! DC-38 signer-backed seal recovery through the release-facing CLI.

// DC-84: pulls in `tests/support/mod.rs` for `unique_suffix()`, and that shared file's own
// (pre-existing, unrelated) helpers use `.unwrap()` throughout — matching every other prikk-cli
// integration test file that already carries this allow.
#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod support;

use prikk_store::{Ed25519MaintainerSigner, MaintainerSigner, RefStore, RepositoryLayout};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const AUTHOR_SEED: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";
const MAINTAINER_SEED: &str = "111122223333444455556666777788889999aaaabbbbccccddddeeeeffff0000";

struct SealedFixture {
    root: PathBuf,
    layout: RepositoryLayout,
    wal_bytes: Vec<u8>,
    metadata_bytes: Vec<u8>,
}

fn prikk(root: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_prikk"));
    command.current_dir(root);
    command
}

fn require_success(output: &Output, action: &str) -> TestResult {
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "{action} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    )
    .into())
}

fn run_seal(root: &Path) -> TestResult<Output> {
    Ok(prikk(root)
        .env("PRIKK_MAINTAINER_KEY_ID", "e2e-maintainer")
        .env("PRIKK_MAINTAINER_SEED", MAINTAINER_SEED)
        .args(["seal", "--allow-no-audit"])
        .output()?)
}

fn setup_sealed(tag: &str) -> TestResult<SealedFixture> {
    let root = unique_root(tag)?;
    require_success(&prikk(&root).arg("init").output()?, "init")?;
    std::fs::write(root.join("state.txt"), b"state\n")?;
    require_success(
        &prikk(&root)
            .env("PRIKK_AUTHOR_KEY_ID", "e2e-author")
            .env("PRIKK_AUTHOR_SEED", AUTHOR_SEED)
            .args(["commit", "-m", "state"])
            .output()?,
        "commit",
    )?;
    let signer = Ed25519MaintainerSigner::from_seed("e2e-maintainer", &hex_seed(MAINTAINER_SEED)?)?;
    let public_key: String = signer
        .public_key_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    require_success(
        &prikk(&root)
            .args([
                "trust",
                "maintainer",
                "add",
                "--key-id",
                "e2e-maintainer",
                "--public-key",
                &public_key,
            ])
            .output()?,
        "trust maintainer add",
    )?;
    let layout = RepositoryLayout::open(root.clone())?;
    let wal_bytes = std::fs::read(layout.default_queue_wal_path())?;
    let metadata_bytes = std::fs::read(layout.default_active_ref_name_path())?;
    require_success(&run_seal(&root)?, "initial seal")?;
    Ok(SealedFixture {
        root,
        layout,
        wal_bytes,
        metadata_bytes,
    })
}

fn restore_active(fixture: &SealedFixture) -> TestResult {
    std::fs::write(fixture.layout.default_queue_wal_path(), &fixture.wal_bytes)?;
    std::fs::write(
        fixture.layout.default_active_ref_name_path(),
        &fixture.metadata_bytes,
    )?;
    Ok(())
}

fn assert_verify_fails(root: &Path, expected_code: &str) -> TestResult {
    let output = prikk(root).arg("verify").output()?;
    if output.status.success() {
        return Err("verify unexpectedly accepted interrupted publication".into());
    }
    if !String::from_utf8_lossy(&output.stdout).contains(expected_code) {
        return Err(format!("verify did not report {expected_code}").into());
    }
    Ok(())
}

#[test]
fn seal_finishes_pointer_leading_log_once() -> TestResult {
    let fixture = setup_sealed("pointer-leading")?;
    std::fs::write(fixture.layout.ref_log_path("heads/main"), b"")?;
    restore_active(&fixture)?;
    assert_verify_fails(&fixture.root, "PRIKK-VERIFY-REF-POINTER-LEADS-LOG")?;
    require_success(&run_seal(&fixture.root)?, "pointer-leading retry")?;
    require_success(&prikk(&fixture.root).arg("verify").output()?, "verify")?;
    assert_eq!(
        RefStore::new(fixture.layout.clone())
            .replay_log("heads/main")?
            .records
            .len(),
        1
    );
    let _ = std::fs::remove_dir_all(fixture.root);
    Ok(())
}

#[test]
fn seal_cleans_matching_retained_active_state_without_append() -> TestResult {
    let fixture = setup_sealed("retained-active-cleanup")?;
    restore_active(&fixture)?;
    assert_verify_fails(&fixture.root, "PRIKK-VERIFY-REF-ACTIVE-CLEANUP-PENDING")?;
    require_success(&run_seal(&fixture.root)?, "active cleanup retry")?;
    require_success(&prikk(&fixture.root).arg("verify").output()?, "verify")?;
    assert_eq!(
        RefStore::new(fixture.layout.clone())
            .replay_log("heads/main")?
            .records
            .len(),
        1
    );
    let _ = std::fs::remove_dir_all(fixture.root);
    Ok(())
}

#[test]
fn seal_truncates_only_partial_tail_before_completion() -> TestResult {
    let fixture = setup_sealed("partial-tail")?;
    std::fs::write(fixture.layout.ref_log_path("heads/main"), b"PREF")?;
    restore_active(&fixture)?;
    assert_verify_fails(&fixture.root, "PRIKK-VERIFY-REF-POINTER-LEADS-LOG")?;
    require_success(&run_seal(&fixture.root)?, "partial-tail retry")?;
    let replay = RefStore::new(fixture.layout.clone()).replay_log("heads/main")?;
    assert_eq!(replay.records.len(), 1);
    assert_eq!(replay.trailing_partial_bytes, 0);
    let _ = std::fs::remove_dir_all(fixture.root);
    Ok(())
}

#[test]
fn seal_rejects_format2_missing_pointer_with_retained_state() -> TestResult {
    let fixture = setup_sealed("legacy-ahead")?;
    std::fs::remove_file(fixture.layout.ref_pointer_path("heads/main"))?;
    restore_active(&fixture)?;
    assert_verify_fails(&fixture.root, "PRIKK-VERIFY-REF-DIVERGENCE")?;
    let output = run_seal(&fixture.root)?;
    assert!(!output.status.success());
    assert!(!fixture.layout.ref_pointer_path("heads/main").exists());
    assert_eq!(
        RefStore::new(fixture.layout.clone())
            .replay_log("heads/main")?
            .records
            .len(),
        1
    );
    let _ = std::fs::remove_dir_all(fixture.root);
    Ok(())
}

#[test]
fn missing_pointer_without_retained_active_evidence_is_divergence() -> TestResult {
    let fixture = setup_sealed("missing-pointer-no-evidence")?;
    std::fs::remove_file(fixture.layout.ref_pointer_path("heads/main"))?;

    assert_verify_fails(&fixture.root, "PRIKK-VERIFY-REF-DIVERGENCE")?;
    let output = run_seal(&fixture.root)?;
    assert!(!output.status.success());
    assert!(!fixture.layout.ref_pointer_path("heads/main").exists());
    let _ = std::fs::remove_dir_all(fixture.root);
    Ok(())
}

#[test]
fn missing_pointer_with_mismatched_active_owner_is_divergence() -> TestResult {
    let fixture = setup_sealed("missing-pointer-wrong-owner")?;
    std::fs::remove_file(fixture.layout.ref_pointer_path("heads/main"))?;
    restore_active(&fixture)?;
    std::fs::write(
        fixture.layout.default_active_ref_name_path(),
        b"heads/topic",
    )?;

    assert_verify_fails(&fixture.root, "PRIKK-VERIFY-REF-DIVERGENCE")?;
    let output = run_seal(&fixture.root)?;
    assert!(!output.status.success());
    assert!(!fixture.layout.ref_pointer_path("heads/main").exists());
    let _ = std::fs::remove_dir_all(fixture.root);
    Ok(())
}

#[test]
fn seal_rejects_format2_log_lead() -> TestResult {
    let fixture = setup_sealed("legacy-existing-ahead")?;
    let pointer_path = fixture.layout.ref_pointer_path("heads/main");
    let old_pointer = std::fs::read(&pointer_path)?;
    std::fs::write(fixture.root.join("state.txt"), b"next state\n")?;
    require_success(
        &prikk(&fixture.root)
            .env("PRIKK_AUTHOR_KEY_ID", "e2e-author")
            .env("PRIKK_AUTHOR_SEED", AUTHOR_SEED)
            .args(["commit", "-m", "next state"])
            .output()?,
        "second commit",
    )?;
    let retained_wal = std::fs::read(fixture.layout.default_queue_wal_path())?;
    let retained_metadata = std::fs::read(fixture.layout.default_active_ref_name_path())?;
    require_success(&run_seal(&fixture.root)?, "second seal")?;
    std::fs::write(&pointer_path, old_pointer)?;
    std::fs::write(fixture.layout.default_queue_wal_path(), retained_wal)?;
    std::fs::write(
        fixture.layout.default_active_ref_name_path(),
        retained_metadata,
    )?;

    assert_verify_fails(&fixture.root, "PRIKK-VERIFY-REF-DIVERGENCE")?;
    let output = run_seal(&fixture.root)?;
    assert!(!output.status.success());
    assert_eq!(
        RefStore::new(fixture.layout.clone())
            .replay_log("heads/main")?
            .records
            .len(),
        2
    );
    let _ = std::fs::remove_dir_all(fixture.root);
    Ok(())
}

fn unique_root(tag: &str) -> TestResult<PathBuf> {
    let path = std::env::temp_dir().join(format!("prikk-dc38-{tag}-{}", support::unique_suffix()));
    std::fs::create_dir_all(&path)?;
    Ok(path)
}

fn hex_seed(value: &str) -> TestResult<[u8; 32]> {
    if value.len() != 64 {
        return Err("seed hex length is not 64".into());
    }
    let mut bytes = [0_u8; 32];
    for (slot, pair) in bytes.iter_mut().zip(value.as_bytes().chunks_exact(2)) {
        let text = std::str::from_utf8(pair)?;
        *slot = u8::from_str_radix(text, 16)?;
    }
    Ok(bytes)
}
