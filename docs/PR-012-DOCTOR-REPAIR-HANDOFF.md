# PR-012 Doctor Repair Handoff

## Scope

PR-012 adds the first mutating doctor repair, limited to the one WAL case that FDD-02 allows as safe without semantic interpretation: truncating incomplete trailing bytes after the last checksum-valid active-WAL record.

This PR intentionally does not repair checksum mismatches, object corruption, ref pointers, ref logs, or missing objects.

## Implementation Handoff

- Added `WalRepair` and `Wal::truncate_trailing_partial()`.
- Added `DoctorRepairOptions` and `DoctorRepairReport`.
- Added `repair_repository()` in `prikk-store`.
- Added CLI support for `prikk doctor [path] --repair-wal-tail`.
- Split `prikk-store` tests into `src/tests.rs` plus `src/tests/` modules to satisfy project file-size guidance.

## Safety Contract

The repair path must only run when:

1. normal verification completes successfully,
2. active-WAL replay reports trailing partial bytes, and
3. the user explicitly passes `--repair-wal-tail`.

If verification fails with an integrity error, doctor repair must refuse to modify the repository.

## Acceptance / QA Checklist

Run:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Manual smoke test:

```sh
cargo run -p prikk -- init ./sample-repo
(cd ./sample-repo && ../target/debug/prikk commit --allow-empty -m "one")
printf 'partial' >> ./sample-repo/.prikk/active/default/queue.wal
cargo run -p prikk -- doctor ./sample-repo
cargo run -p prikk -- doctor ./sample-repo --repair-wal-tail
cargo run -p prikk -- verify ./sample-repo
```

Expected result:

- First doctor run reports a trailing partial WAL warning.
- Repair run reports 7 truncated bytes and preserves one WAL record.
- Verify reports zero trailing partial WAL bytes.

## Deferred

- Repairing checksum mismatch in non-final WAL records.
- Ref reconstruction from logs.
- Missing object recovery.
- Object quarantine or GC.
- Worktree materialization repair.
