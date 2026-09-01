//! Worktree status tests (RFC 122, `replay-baseline-handoff-v1.md`).
//!
//! Every fixture here is a real, sealed `commit`/`seal` repository -- the same shape the CLI
//! actually produces -- never the old snapshot-only-block fixture this file used before RFC 122.
//! That fixture was not what any production path ever wrote (only a test helper here did), which
//! is exactly why the pre-fix `worktree_status` implementation could pass every test in this file
//! while failing on every repository the CLI can actually create (the bug RFC 122 fixes).

#![allow(clippy::expect_used, clippy::unwrap_used)]

use crate::test_support::unique_temp_dir;
use crate::worktree_patch::commit_worktree_changes_signed;
use crate::{
    Ed25519AuthorSigner, Ed25519MaintainerSigner, MaintainerSigner, RepositoryLayout,
    WorktreeChangeKind, WorktreePatchCommitOptions, worktree_status,
};

fn author_signer() -> Ed25519AuthorSigner {
    Ed25519AuthorSigner::from_seed("worktree-status-author", &[0x11; 32]).unwrap()
}

fn maintainer_signer() -> Ed25519MaintainerSigner {
    Ed25519MaintainerSigner::from_seed("worktree-status-maintainer", &[0x22; 32]).unwrap()
}

fn trust_maintainer(layout: &RepositoryLayout, maintainer: &Ed25519MaintainerSigner) {
    crate::trust::add_trusted_maintainer(
        layout,
        maintainer.key_id(),
        &prikk_hash::to_hex(&maintainer.public_key_bytes()),
    )
    .unwrap();
}

/// One real generation: write `path`, commit, seal -- the same commit/seal cycle
/// `prikk commit`/`prikk seal` perform, via `commit_worktree_changes_signed` (production, real
/// `NodeIdGenerator`) and `simulate_one_seal` (RFC 111 Stage 2's drift-guarded seal replica,
/// checked against the real `prikk seal` binary by its own gate).
fn generation(layout: &RepositoryLayout, path: &str, bytes: &[u8], message: &str) {
    std::fs::write(layout.root().join(path), bytes).unwrap();
    commit_worktree_changes_signed(
        layout,
        "heads/main",
        message,
        WorktreePatchCommitOptions::default(),
        &author_signer(),
    )
    .unwrap();
    crate::rfc111_seal_simulation::simulate_one_seal(layout, "heads/main", &maintainer_signer())
        .unwrap();
}

#[test]
fn worktree_status_is_clean_after_commit_and_seal() {
    let root = unique_temp_dir("worktree-status-clean");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    trust_maintainer(&layout, &maintainer_signer());
    generation(&layout, "README.md", b"hello\n", "genesis");

    let report = worktree_status(&layout, "heads/main").unwrap();
    assert!(report.is_clean(), "{report:?}");
    assert_eq!(report.tracked_files, 1);
    assert_eq!(report.unchanged_files, 1);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn worktree_status_reports_modified_file() {
    let root = unique_temp_dir("worktree-status-modified");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    trust_maintainer(&layout, &maintainer_signer());
    generation(&layout, "README.md", b"hello\n", "genesis");

    std::fs::write(root.join("README.md"), b"changed\n").unwrap();
    let report = worktree_status(&layout, "heads/main").unwrap();
    assert!(!report.is_clean());
    assert_eq!(report.count_kind(WorktreeChangeKind::Modified), 1);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn worktree_status_reports_missing_file() {
    let root = unique_temp_dir("worktree-status-missing");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    trust_maintainer(&layout, &maintainer_signer());
    generation(&layout, "README.md", b"hello\n", "genesis");

    std::fs::remove_file(root.join("README.md")).unwrap();
    let report = worktree_status(&layout, "heads/main").unwrap();
    assert!(!report.is_clean());
    assert_eq!(report.count_kind(WorktreeChangeKind::Missing), 1);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn worktree_status_reports_untracked_file() {
    let root = unique_temp_dir("worktree-status-untracked");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    trust_maintainer(&layout, &maintainer_signer());
    generation(&layout, "README.md", b"hello\n", "genesis");

    std::fs::write(root.join("extra.txt"), b"extra\n").unwrap();
    let report = worktree_status(&layout, "heads/main").unwrap();
    assert!(!report.is_clean());
    assert_eq!(report.count_kind(WorktreeChangeKind::Untracked), 1);

    let _ = std::fs::remove_dir_all(root);
}

/// RFC 122 §3's design choice, demonstrated: `worktree-status` folds an already-queued (unsealed)
/// patch onto the sealed baseline, the same way `commit` would author against it. Sealed
/// generation 1 (`a.txt`), then a second commit on `b.txt` left deliberately unsealed -- if folding
/// were not happening, `b.txt` would read as untracked (it is not in any sealed block yet) rather
/// than as part of the tracked, unchanged baseline.
#[test]
fn worktree_status_folds_an_already_queued_unsealed_patch() {
    let root = unique_temp_dir("worktree-status-queue");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    trust_maintainer(&layout, &maintainer_signer());
    generation(&layout, "a.txt", b"first\n", "first");

    // Second commit, deliberately not sealed -- stays queued in the active WAL.
    std::fs::write(root.join("b.txt"), b"second\n").unwrap();
    commit_worktree_changes_signed(
        &layout,
        "heads/main",
        "second, unsealed",
        WorktreePatchCommitOptions::default(),
        &author_signer(),
    )
    .unwrap();

    let report = worktree_status(&layout, "heads/main").unwrap();
    assert!(
        report.is_clean(),
        "the queued file's own worktree bytes still match what was just committed: {report:?}"
    );
    assert_eq!(
        report.tracked_files, 2,
        "the queued commit's own file must be folded into the baseline, not left untracked: \
         {report:?}"
    );
    assert_eq!(report.unchanged_files, 2);

    let _ = std::fs::remove_dir_all(root);
}
