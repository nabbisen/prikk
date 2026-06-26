# PR-014 History Inspection Handoff

## Summary

PR-014 adds read-only sealed-history inspection on top of the existing RefState and Block
persistence model. It does not introduce patch algebra, worktree diff capture, audit policy, or
sync behavior.

## Implemented

- `prikk_store::load_ref_history()`.
- `RefHistory` and `HistoryEntry` read models.
- Current RefState chain traversal from newest to oldest.
- Target Block decoding and validation for each history entry.
- Cycle detection in the RefState chain.
- CLI command: `prikk log [path] [--limit N] [--ref REF]`.
- Unit tests for newest-first ordering and limit handling.

## Scope Boundaries

- History follows `RefState.previous_ref_state_id`; it does not yet run full block-DAG graph
  traversal.
- Block summaries, merge-base traversal, and path-aware history queries remain future work.
- Output is intentionally plain text for early CLI diagnostics.

## QA Checklist

Run:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Manual smoke test:

```sh
cargo run -p prikk -- init ./sample-repo
(cd ./sample-repo && ../target/debug/prikk commit --allow-empty -m "initial scaffold")
(cd ./sample-repo && ../target/debug/prikk seal --allow-no-audit)
cargo run -p prikk -- log ./sample-repo
cargo run -p prikk -- verify ./sample-repo
```

## Deferred

- Worktree diff capture.
- Patch apply, inverse, commutation, and conflict witnesses.
- Full block-DAG history traversal.
- Audit/attestation publication.
- Plugin host and remote sync.
