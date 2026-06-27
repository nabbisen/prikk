# PR-023 Text Anchor Scaffold Handoff

## Summary

PR-023 introduces the first implementation-facing scaffold for content-anchored text edits. It does not enable text diff generation, text replay, commutation, inverse patches, or conflicted merge states. The goal is to make the `EditText` payload stricter before any later patch-engine code depends on it.

## Implemented

- Added `TEXT_SPAN_HASH_BYTES` as the fixed text-span hash length.
- Added `text_span_hash(bytes)` for stable span-precondition hashing.
- Added `validate_text_anchor_id(value)` for v1 anchor identifier validation.
- Changed `EditText.old_span_hash` to a fixed `[u8; 32]` value.
- Added `EditText::validate()` and canonical encoding validation.
- Added tests for anchor validation, stable span hashing, and invalid anchor rejection.
- Fixed an obvious replay-source transcription defect in the `ReplaceBinary` branch.

## Acceptance / QA Checklist

Run:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Expected behavior:

- Existing object, WAL, ref, checkout, and worktree tests still pass.
- Empty text anchor IDs are rejected.
- Text span hashes are stable for identical bytes and different for different bytes.
- Patch replay still refuses `EditText` as unsupported until the text replay RFC/FDD work is implemented.

## Deferred

- Worktree text diff generation.
- Content-anchor discovery in real files.
- Text edit replay.
- Offset presentation mapping.
- Inverse generation.
- Commutation and conflict witnesses.
