# DC-54 Operation Path Validation Symmetry - Implementation Evidence

**Scope.** DC-54 only. No CI change. `Cargo.lock` unchanged (no dependency added).
**Predecessor.** DC-41 stage 4 (uncommitted at the time of this work — see the note at the end on
commit sequencing).
**Design authority.** `rfcs/accepted/DC-54-OPERATION-PATH-VALIDATION-SYMMETRY.md`, accepted by the
project owner 2026-07-28 after the author's design-completion self-critique.

## Step 1 — the validator move (pure move, verified behavior-neutral)

`validate_repo_path`, `validate_component`, and `is_windows_reserved_name` moved verbatim from
`crates/prikk-replay/src/path.rs` to new `crates/prikk-object/src/path.rs`. (The handoff named the
first two explicitly; `is_windows_reserved_name` is `validate_component`'s own private callee and
had to move with it to compile — noted since the handoff didn't name it separately.)
`prikk-replay/src/path.rs` now does `pub use prikk_object::validate_repo_path;` instead of defining
it, so `prikk_replay::validate_repo_path` and (via its existing re-export)
`prikk_store::validate_repo_path` keep compiling unchanged for every current caller. Confirmed no
new dependency: `prikk-object` already depends on `prikk-error`, and `prikk-replay` already
depended on `prikk-object`.

Verified behavior-neutral **before** adding any new validation: full workspace test run after the
move alone showed identical counts to the pre-DC-54 baseline (`prikk-object` 72, `prikk-store` 540,
all else unchanged), confirming the move altered nothing.

`prikk-replay/src/path/tests.rs`'s existing three tests needed no changes — they exercise
`RepoPath::parse`/`validate_no_path_collisions` black-box and never touched the moved private
helpers directly.

## Step 2 — validation wired into the four operation kinds

`crates/prikk-object/src/payload/patch.rs`: `validate_repo_path(&self.path)?` (or, for `RenamePath`,
both `old_path` and `new_path` independently) added to the existing `validate()` of `CreateFile`,
`DeleteNode`, `RenamePath`, `CreateSymlink`, alongside the existing `node_id.is_zero()` check.
`DeleteNode.old_target` and `CreateSymlink.target` are untouched, per the "do not touch" boundary.

**RenamePath's two-call requirement honored.** `old_path` and `new_path` are validated
independently — the handoff's guard test
(`rename_path_rejects_a_bad_old_path_even_when_new_path_is_valid`) confirms a bad `old_path` with a
valid `new_path` still fails, which a single-field check would have missed.

Running the **full workspace test suite immediately after this step** (before adding any new tests)
showed zero regressions — every existing fixture and golden vector already used valid paths, exactly
as the design's compatibility analysis predicted.

## Error taxonomy — as decided

`InvalidName` propagates unchanged (`validate_repo_path(&self.path)?`, no wrapping), uniformly
across all four kinds. Verified directly (not just asserted) via
`dc54_encode_decode_symmetry::encode_and_decode_reject_the_same_invalid_path_with_identical_error_text`:
for four distinct invalid inputs (reserved name, traversal, absolute, `.prikk`-prefixed),
`RenamePath::validate()`'s error string and `RepoPath::parse()`'s error string (what
`decode_rename_path` actually calls) are asserted equal. They are equal **by construction**, since
both now bottom out in the same moved function — but the test proves it empirically rather than
relying on the construction argument alone.

## Existing-repository compatibility — evidenced, both citations checked directly

Read both cited call sites myself rather than trusting the design document's claim:

- `crates/prikk-store/src/worktree_patch/node_authoring/worktree_files.rs:72` —
  `RepoPath::parse(rel).map_err(AuthorError::Store)?` runs before the corresponding operation is
  constructed.
- `crates/prikk-store/src/worktree_patch/node_authoring.rs:340` —
  `RepoPath::parse(path).map_err(AuthorError::Store)?` likewise, in the create-candidate authoring
  loop.

Both confirmed present and doing what the design claimed. Combined with: an operation whose path
fails `RepoPath::parse` could never have decoded, so it could never have been replayed or sealed by
this tool; and the full test suite (including every FDD golden vector in `prikk-object::vectors`)
passing unchanged after Step 2, with no fixture touching an invalid path. **Conclusion:** no
repository produced by this tool can contain an operation this change newly rejects.

## Identity neutrality — evidenced, not merely asserted

Tightening encode only **removes** previously-encodable (but never-decodable) values; it cannot
change the bytes produced for any value that already encoded *and* decoded, because `validate()`'s
new checks run before `encode_canonical`'s field-writing logic, which is byte-for-byte unchanged.
The empirical proof: every currently-committed FDD-03 golden vector and canonical-encoding snapshot
test (`prikk-object::vectors::hard::*`, `vectors::snapshot::generated_snapshot_matches_committed`)
still passes with **identical** expected bytes after this change — if identity had shifted for any
tested value, those tests would have failed. No accepted byte sequence changed; therefore no
existing `ObjectId` changed.

## Step 3 — test matrix

Two new test modules, split by what each crate can prove:

- `prikk-object::payload::tests::path_validation` (4 tests) — the encode-only half of the matrix:
  valid paths across all four kinds; the four invalid-path categories (Windows-reserved, traversal,
  absolute, `.prikk`-prefixed) across all four kinds; the `RenamePath` two-call guard; the opaque-field
  boundary (`DeleteNode.old_target`, `CreateSymlink.target` still accept arbitrary UTF-8, proving they
  were not tightened by accident).
- `prikk-store::patch_replay::tests::dc54_encode_decode_symmetry` (3 tests) — the cross-crate half:
  the symmetry proof (identical error text, above), a positive symmetry case (valid path, both sides
  accept), and a test pinning the *exact* committed DC-41 reproducer
  (`node_id: [1; 32]`, `old_path: "a"`, `new_path: "com1"`) to fail at **encode** — asserted two ways:
  `RenamePath::validate()` returns an error naming `"com1"`, and the full `PatchPayload::to_canonical_bytes()`
  also fails, so there are no bytes for a decoder to ever see.

## Step 4 — the proof that this is actually fixed

Removed the `path_segment_strategy()` exclusion filter in
`crates/prikk-store/src/patch_replay/tests/proptest_round_trip.rs` entirely (the disclosure comment
retired with it, replaced by a note explaining the filter is no longer needed and why). This required
one behavioral update to `patch_operations_round_trip` itself: since encode can now legitimately
reject a generated operation (unrestricted `path_segment_strategy()` can still produce `"com1"`), the
property no longer assumes `to_canonical_bytes()` always succeeds — it treats an encode failure as an
expected outcome to skip (`let Ok(bytes) = payload.to_canonical_bytes() else { return Ok(()); }`),
not a bug. This is the correct generalization: the property now says "if it encodes, it round-trips,"
which is what was always actually true; the old code merely never observed an encode failure because
none could happen.

**Unfiltered campaign, run twice** (once right after the fix, once again after the final `cargo fmt`
pass touched the file): `PROPTEST_CASES=100000 cargo test -p prikk-store --lib --release
patch_replay::tests::` — **zero mismatches, zero panics**, both times. The committed regression file
(`proptest-regressions/patch_replay/tests/proptest_round_trip.txt`) was replayed first on every run
per proptest's own mechanism and passed cleanly, confirming the graceful-skip path is exercised for
real, not merely reachable in theory.

## Test counts

- `prikk-object`: **72 → 76** (+4, `path_validation`).
- `prikk-store`: **540 → 543** (+3, `dc54_encode_decode_symmetry`).
- `prikk-hash`: **13**, unchanged.
- `prikk-replay`: **4**, unchanged (the pure move needed no new tests there).

## Frozen identities

| Identity | Status |
|---|---|
| `Cargo.toml` (workspace root) | unchanged |
| `Cargo.lock` | **unchanged** — `601d0678…5da31`, 180 packages (this increment adds no dependency) |
| All package manifests | unchanged |
| Command inventories (both) | unchanged |
| Oracle manifest | unchanged |
| `release-signers.toml` | unchanged; signer set still empty and fail-closed |

## Gate output

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | clean |
| `cargo test --workspace --locked` | `prikk-object` 76, `prikk-store` 543, all else unchanged, no failures |
| `cargo +1.85.0 test --workspace --locked` | same counts, no failures |
| `git diff --check` | clean |
| `cargo audit --no-fetch` | 180 dependencies scanned, 0 advisories |
| release-policy `check` | all 154 oracle cases passed |
| `boundary-check` / `reference-check` | `valid: true` |
| Unfiltered campaign (100,000 cases, `--release`, run twice) | zero findings both times |

**Decoder, `RepoPath` grammar, wire format, and opaque target fields are unchanged** — only the four
`validate()` methods in `prikk-object` gained a call to the (moved, not rewritten) grammar.

## Commit-sequencing note

DC-41 stage 4 was still uncommitted when this work started (its implementation review was accepted,
but landing was deferred pending this DC-54 detour). This candidate's diff is layered on top of that
uncommitted state. The two are logically separate increments and should land as two separate commits,
DC-41 stage 4 first (its own file scope: `wal/tests/`, `refs/log/`, `file_codec/`,
`payload/tests/proptest_decoders.rs`, the DC-41 stage-4 evidence note, and the workspace
`proptest`/`Cargo.lock` change) and DC-54 second (`prikk-object/src/path.rs`, the `payload/patch.rs`
validation calls, `prikk-replay/src/path.rs`'s re-export change, `path_validation.rs`,
`dc54_encode_decode_symmetry.rs`, this evidence note, and the DC-54 RFC/lifecycle bookkeeping) —
matching "one increment per candidate, no bundling."
