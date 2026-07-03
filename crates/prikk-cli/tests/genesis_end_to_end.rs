//! CLI end-to-end regression for the genesis first-commit flow (DC-09 4.4b P2-1).
//!
//! Drives the exact release-facing path through the compiled `prikk` binary:
//! `init → commit (genesis) → seal → log → verify`. This complements the store-level genesis tests
//! by guarding the CLI wiring that the release documents.

#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use prikk_store::{Ed25519MaintainerSigner, MaintainerSigner};

fn prikk(repo: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_prikk"));
    cmd.current_dir(repo);
    cmd
}

fn ok(output: &Output, what: &str) {
    assert!(
        output.status.success(),
        "{what} failed (status {:?})\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn fail(output: &Output, what: &str) {
    assert!(
        !output.status.success(),
        "{what} unexpectedly succeeded\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn unique_repo(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "prikk-cli-e2e-{tag}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn public_key_hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn maintainer_seed() -> &'static str {
    "111122223333444455556666777788889999aaaabbbbccccddddeeeeffff0000"
}

fn maintainer_signer() -> Ed25519MaintainerSigner {
    Ed25519MaintainerSigner::from_seed(
        "e2e-maintainer",
        &[
            0x11, 0x11, 0x22, 0x22, 0x33, 0x33, 0x44, 0x44, 0x55, 0x55, 0x66, 0x66, 0x77, 0x77,
            0x88, 0x88, 0x99, 0x99, 0xaa, 0xaa, 0xbb, 0xbb, 0xcc, 0xcc, 0xdd, 0xdd, 0xee, 0xee,
            0xff, 0xff, 0x00, 0x00,
        ],
    )
}

fn add_trusted_maintainer(repo: &Path) {
    let signer = maintainer_signer();
    let out = prikk(repo)
        .args([
            "trust",
            "maintainer",
            "add",
            "--key-id",
            "e2e-maintainer",
            "--public-key",
            &public_key_hex(&signer.public_key_bytes()),
        ])
        .output()
        .unwrap();
    ok(&out, "trust maintainer add");
}

#[test]
fn genesis_init_commit_seal_log_verify() {
    let repo = unique_repo("genesis");

    // init a fresh repository in the working directory.
    let out = prikk(&repo).arg("init").output().unwrap();
    ok(&out, "init");

    // two regular worktree files to author as the genesis commit.
    std::fs::write(repo.join("readme.txt"), b"hello prikk\n").unwrap();
    std::fs::write(repo.join("main.rs"), b"fn main() {}\n").unwrap();

    // genesis commit: node-addressed, role-bound Ed25519 AUTHOR-signed (key material via env).
    let out = prikk(&repo)
        .env("PRIKK_AUTHOR_KEY_ID", "e2e-author")
        .env(
            "PRIKK_AUTHOR_SEED",
            "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff",
        )
        .args(["commit", "-m", "genesis"])
        .output()
        .unwrap();
    ok(&out, "commit");
    let commit_stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        commit_stdout.contains("operations: 2"),
        "expected two CreateFile operations; stdout: {commit_stdout}"
    );

    add_trusted_maintainer(&repo);

    // seal publishes the first (Root) block and advances heads/main.
    let out = prikk(&repo)
        .env("PRIKK_MAINTAINER_KEY_ID", "e2e-maintainer")
        .env("PRIKK_MAINTAINER_SEED", maintainer_seed())
        .args(["seal", "--allow-no-audit"])
        .output()
        .unwrap();
    ok(&out, "seal");

    // log shows a Root block at update-seq 1.
    let out = prikk(&repo).arg("log").output().unwrap();
    ok(&out, "log");
    let log_stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        log_stdout.contains("Root"),
        "expected a Root block; log stdout: {log_stdout}"
    );
    assert!(
        log_stdout.contains("update-seq: 1"),
        "expected update-seq 1; log stdout: {log_stdout}"
    );

    // verify reports a clean repository.
    let out = prikk(&repo).arg("verify").output().unwrap();
    ok(&out, "verify");

    let _ = std::fs::remove_dir_all(&repo);
}

#[test]
fn non_default_ref_genesis_commit_seal_log_verify() {
    let repo = unique_repo("nondefault-genesis");
    let out = prikk(&repo).arg("init").output().unwrap();
    ok(&out, "init");

    std::fs::write(repo.join("topic.txt"), b"topic root\n").unwrap();
    let out = prikk(&repo)
        .env("PRIKK_AUTHOR_KEY_ID", "e2e-author")
        .env(
            "PRIKK_AUTHOR_SEED",
            "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff",
        )
        .args(["commit", "--ref", "heads/topic", "-m", "topic genesis"])
        .output()
        .unwrap();
    ok(&out, "commit --ref");
    let commit_stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        commit_stdout.contains("baseline ref: heads/topic"),
        "expected heads/topic baseline; stdout: {commit_stdout}"
    );

    add_trusted_maintainer(&repo);
    let out = prikk(&repo)
        .env("PRIKK_MAINTAINER_KEY_ID", "e2e-maintainer")
        .env("PRIKK_MAINTAINER_SEED", maintainer_seed())
        .args(["seal", "--allow-no-audit", "--ref", "heads/topic"])
        .output()
        .unwrap();
    ok(&out, "seal --ref");
    let seal_stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        seal_stdout.contains("heads/topic RefState:"),
        "expected heads/topic publication; stdout: {seal_stdout}"
    );
    assert!(
        !repo.join(".prikk/active/default/ref-name").exists(),
        "seal should remove active ref metadata"
    );

    let out = prikk(&repo)
        .args(["log", "--ref", "heads/topic"])
        .output()
        .unwrap();
    ok(&out, "log --ref");
    let log_stdout = String::from_utf8_lossy(&out.stdout);
    assert!(log_stdout.contains("Root"), "log stdout: {log_stdout}");
    assert!(
        log_stdout.contains("update-seq: 1"),
        "log stdout: {log_stdout}"
    );

    let out = prikk(&repo).arg("verify").output().unwrap();
    ok(&out, "verify");
    let _ = std::fs::remove_dir_all(&repo);
}

#[test]
fn seal_rejects_active_wal_owned_by_another_ref() {
    let repo = unique_repo("seal-ref-mismatch");
    let out = prikk(&repo).arg("init").output().unwrap();
    ok(&out, "init");

    std::fs::write(repo.join("topic.txt"), b"topic root\n").unwrap();
    let out = prikk(&repo)
        .env("PRIKK_AUTHOR_KEY_ID", "e2e-author")
        .env(
            "PRIKK_AUTHOR_SEED",
            "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff",
        )
        .args(["commit", "--ref", "heads/topic", "-m", "topic genesis"])
        .output()
        .unwrap();
    ok(&out, "commit --ref");

    add_trusted_maintainer(&repo);
    let out = prikk(&repo)
        .env("PRIKK_MAINTAINER_KEY_ID", "e2e-maintainer")
        .env("PRIKK_MAINTAINER_SEED", maintainer_seed())
        .args(["seal", "--allow-no-audit"])
        .output()
        .unwrap();
    fail(&out, "seal wrong ref");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("active WAL is owned by heads/topic"),
        "unexpected stderr: {stderr}"
    );

    let _ = std::fs::remove_dir_all(&repo);
}
