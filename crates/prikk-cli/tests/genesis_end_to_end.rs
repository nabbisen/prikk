//! CLI end-to-end regression for the genesis first-commit flow (DC-09 4.4b P2-1).
//!
//! Drives the exact release-facing path through the compiled `prikk` binary:
//! `init → commit (genesis) → seal → log → verify`. This complements the store-level genesis tests
//! by guarding the CLI wiring that the release documents.

#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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

    // seal publishes the first (Root) block and advances heads/main.
    let out = prikk(&repo)
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
