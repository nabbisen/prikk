# PR-024 Implementation Handoff — Conservative Text Replay

## Summary

PR-024 adds conservative replay for a single content-anchored text edit shape: full-file exact-span replacement. This is an M2 bridge increment, not full patch algebra.

## Implemented

- Split supported patch-operation decoding into `crates/prikk-store/src/patch_replay/decode.rs`.
- Added canonical decode support for `EditText` operations.
- Added supported replay for `EditText` when `anchor_id == "full-file"`.
- Replay verifies that:
  - the target path exists in the replayed state;
  - the current file bytes are valid UTF-8;
  - `text_span_hash(current_file_bytes)` equals `old_span_hash`.
- Replay replaces the whole file with `replacement` bytes.
- Added a replay test for full-file text edit behavior.

## Explicit Non-Goals

- No worktree text diff generation.
- No arbitrary span lookup or anchor discovery.
- No offset-based text replay.
- No inverse generation.
- No commutation or conflict witnesses.
- No audit/plugin/sync work.

## QA Checklist

Run:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Also manually test:

```sh
cargo run -p prikk -- checkout --patch-plan ./sample-repo
cargo run -p prikk -- checkout --patch-materialize ./sample-repo
```

## Follow-up

The next patch-engine work should be a reviewed apply/inverse boundary for the currently supported operation subset, not arbitrary text diff generation yet.
