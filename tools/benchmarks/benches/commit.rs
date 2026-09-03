//! Benchmarks `prikk commit`'s worktree-authoring path in-process (RFC 126 §5 increment A).
//!
//! One benchmark, a genesis commit (against an empty baseline) of a small fixed-size worktree --
//! the smallest fixture that produces a stable number, per this increment's own scope. Fixture
//! setup (a fresh temp directory, `RepositoryLayout::init`, writing the fixture files) is excluded
//! from the timed measurement via `iter_batched`'s untimed setup closure, so a data point reflects
//! only `commit_worktree_changes_signed` itself -- the same function `prikk commit` calls
//! (`crates/prikk-cli/src/main.rs::run_commit`).
//!
//! Not a suite. Migrating `dc59_commit_benchmark.rs`'s own larger Axis A/B repository-size and
//! changed-file-count measurements onto this member is a separate increment (RFC 126 §5 increment
//! B); this file does not move or duplicate any of that harness's fixture-generation code.
//!
//! **Measures wall-clock time only.** It says nothing about peak RSS -- the axis DC-62's own
//! regression risk is stated against -- because criterion does not measure memory. That gap is not
//! closed by this member existing; a peak-RSS measurement is a different, unbuilt mechanism.

#![allow(missing_docs)] // criterion_group! expands to an undocumented function; not fixable here.
#![allow(clippy::expect_used)] // Benchmark fixture setup, matching every other harness's own allow.

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use prikk_store::{
    Ed25519AuthorSigner, RepositoryLayout, WorktreePatchCommitOptions,
    commit_worktree_changes_signed,
};

const AUTHOR_KEY_ID: &str = "prikk-benchmarks-commit-author";
const AUTHOR_SEED: [u8; 32] = [0x33; 32];
/// Small and fixed: this benchmark measures one point, not a curve across repository size -- see
/// `dc59_commit_benchmark.rs` for the axis that does.
const FILE_COUNT: usize = 10;

/// One fresh repository, ready for a genesis commit. Held for the duration of one timed iteration;
/// the temp directory is removed when this drops, after the routine returns.
struct Fixture {
    _root: tempfile::TempDir,
    layout: RepositoryLayout,
    signer: Ed25519AuthorSigner,
}

fn setup() -> Fixture {
    let root = tempfile::tempdir().expect("create temp dir");
    let layout =
        RepositoryLayout::init(root.path().to_path_buf()).expect("init benchmark repository");
    for index in 0..FILE_COUNT {
        std::fs::write(
            root.path().join(format!("f{index}.txt")),
            b"prikk-benchmarks commit fixture content\n",
        )
        .expect("write benchmark fixture file");
    }
    let signer = Ed25519AuthorSigner::from_seed(AUTHOR_KEY_ID, &AUTHOR_SEED)
        .expect("derive fixed benchmark author signer");
    Fixture {
        _root: root,
        layout,
        signer,
    }
}

fn bench_genesis_commit(c: &mut Criterion) {
    c.bench_function(&format!("commit_genesis_{FILE_COUNT}_files"), |b| {
        // `PerIteration`, not batched: each input owns a temp directory (an OS resource), and
        // genesis authoring cannot run twice against the same repository without a seal between
        // attempts (the active-WAL guard `dc59_commit_benchmark.rs`'s own module doc explains),
        // so every iteration needs its own fresh repository regardless of batch size.
        b.iter_batched(
            setup,
            |fixture| {
                commit_worktree_changes_signed(
                    &fixture.layout,
                    "heads/main",
                    "benchmark commit",
                    WorktreePatchCommitOptions::default(),
                    &fixture.signer,
                )
                .expect("benchmark commit must succeed")
            },
            BatchSize::PerIteration,
        );
    });
}

criterion_group!(benches, bench_genesis_commit);
criterion_main!(benches);
