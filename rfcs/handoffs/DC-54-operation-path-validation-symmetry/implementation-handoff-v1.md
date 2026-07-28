# DC-54 Operation Path Validation Symmetry - Implementation Handoff

**Prepared in advance.** Implementation may **not** begin until `rfcs/proposed/DC-54-…` moves to
`rfcs/accepted/` through design review. This is a **production behaviour change** (encode becomes
stricter), so the design gate is not a formality.
**Authored by** the architect (function-designer role). Implementation review remains independent.
**Size:** small in code, careful in evidence — four `validate()` methods, plus compatibility and
identity-neutrality proof.
**Origin:** discovered by DC-41 stage 4's campaign; reproducer already committed.

## Step 1 — move the validator (do this first; you cannot call it where it currently lives)

`validate()` is in `prikk-object`; `validate_repo_path` is in `prikk-replay`; and `prikk-replay` already
depends on `prikk-object`. Calling it directly would be a **dependency cycle** that Cargo rejects. Do not
work around this by writing a second validator — a parallel implementation recreates this exact defect in
a new form, which is the whole lesson of the finding.

Instead:

1. Move `validate_repo_path` **and** its private helper `validate_component` from
   `crates/prikk-replay/src/path.rs` into a new `crates/prikk-object/src/path.rs`. Both are pure string
   logic over `prikk_error::{PrikkError, Result}` and `std`; `prikk-object` already depends on
   `prikk-error`, so **no new dependency is added**.
2. Have `prikk-replay::RepoPath::parse` call the moved function, and re-export it so
   `prikk_replay::validate_repo_path` and `prikk_store::validate_repo_path` keep working —
   `crates/prikk-replay/src/lib.rs:17` and `crates/prikk-store/src/path.rs:5` re-export it today and every
   current caller must keep compiling unchanged.
3. Leave the `RepoPath` **type** and `validate_no_path_collisions` in `prikk-replay`. Only the lexical
   validator moves.

Do this as a **separable first commit or hunk** — a pure move with no behaviour change — so the reviewer
can confirm neutrality before reading the new validation. Same technique that worked in DC-41 stage 2.

## Step 2 — call it from the four `validate()` methods

In `crates/prikk-object/src/payload/patch.rs`, call the moved `validate_repo_path` from the write-side
`validate()` of four operation kinds, covering these five fields:

| Kind | Field(s) |
|---|---|
| `CreateFile` | `path` |
| `DeleteNode` | `path` |
| `RenamePath` | `old_path`, `new_path` |
| `CreateSymlink` | `path` |

Each `validate()` currently checks only `node_id.is_zero()`. `RenamePath::validate()` at `:515-525` is the
model — **add** the path check alongside the existing node check; keep the node check.

`RenamePath` needs **two** calls, one per field. A single call covering only `new_path` would leave
`old_path` asymmetric and would still pass the committed reproducer, since that case's `old_path` (`"a"`)
is valid — so it would look fixed while remaining half-broken.

## Step 3 — tests

| Case | Expect |
|---|---|
| Each of the four kinds, valid path | encodes successfully |
| Each of the four kinds, Windows-reserved name (`com1`) | **fails at encode** |
| Each of the four kinds, traversal component (`..`) | fails at encode |
| Each of the four kinds, absolute path (`/x`) | fails at encode |
| Each of the four kinds, `.prikk`-prefixed path | fails at encode |
| `RenamePath` with the bad path in `old_path` **only** | fails at encode (guards the two-call requirement above) |
| Same input through encode and through decode | identical error text |
| `DeleteNode.old_target`, `CreateSymlink.target` with arbitrary UTF-8 | still accepted — proves the opaque fields were not tightened |

The symmetry case is the one that proves the increment's actual thesis; do not omit it.

## Do not touch

- **`DeleteNode.old_target` and `CreateSymlink.target`.** Decode reads these as plain strings with no
  `RepoPath::parse` call. Their opacity is an accepted DC-40 decision (schema-1 symlink targets are opaque
  UTF-8 by design). Tightening them would be an unreviewed format change.
- **The decoder, the `RepoPath` grammar, and the FDD-03 §9.3 wire format.** All frozen.
- **`RenamePath`'s deferred application status.** `ensure_apply_supported` still returns
  `UnsupportedObjectType`; this RFC does not lift that.

## Two things to evidence, not assert

1. **Existing-repository compatibility — answer already established, still cite it.** Worktree authoring
   validates paths before constructing operations
   (`worktree_patch/node_authoring/worktree_files.rs:72`, `node_authoring.rs:340`), and an invalid path
   could never have round-tripped because decode rejects it. So no repository produced by this tool can
   contain one; the residual exposure is a direct API caller bypassing the worktree layer, which is
   exactly the hole this closes. Cite those two call sites in the evidence note rather than re-deriving
   or merely restating the conclusion.
2. **Identity neutrality.** Tightening encode rejects previously-encodable values; it must not alter the
   bytes produced for any value that already encodes *and* decodes. State explicitly: no accepted byte
   sequence changes, therefore no existing ObjectId changes. This is the claim a reviewer will check
   hardest, because DC-39 and DC-40 froze identity.

## Step 4 — the proof that this is actually fixed

**Remove the `path_segment_strategy()` exclusion** in
`crates/prikk-store/src/patch_replay/tests/proptest_round_trip.rs` (the filter excluding the eleven
Windows-reserved base names, disclosed at `:44-57`), then run the campaign budget unfiltered:

```
PROPTEST_CASES=100000 cargo test --release -p prikk-store --lib proptest_round_trip
```

A clean unfiltered campaign is the real acceptance signal. Retire the disclosure comment with the filter —
leaving it would describe a defect that no longer exists.

The committed reproducer at
`crates/prikk-store/proptest-regressions/patch_replay/tests/proptest_round_trip.txt` should now fail at
**encode** with a clear error rather than at decode. Keep the file: proptest re-runs saved cases first,
so it becomes a permanent regression guard.

## Error taxonomy — decided

**Propagate `InvalidName` unchanged.** Call `validate_repo_path(&self.path)?` with no wrapping, uniformly
across the four kinds.

Symmetry is the point of the increment, so encode and decode should produce identical error text — that
makes the symmetry verifiable rather than asserted, and wrapping would discard the specific rule broken
(`"Windows reserved path component is not allowed: com1"`). The resulting difference from the sibling
`node_id` check (which returns `CanonicalEncoding`) is **correct, not inconsistent**: a zero `node_id` is
a reserved *encoding* value; an unsafe path is an invalid *name*. Two failure classes, two error types.

## Definition of done

- Validator moved as a separable pure-move hunk; all existing re-export callers compile unchanged.
- All five fields validated at encode with the same grammar decode enforces, `RenamePath` via two calls.
- Test matrix in Step 3 complete, including the encode/decode symmetry case and the opaque-field guard.
- Opaque target fields untouched; decoder, grammar, and wire format unchanged.
- Exclusion filter removed; unfiltered 100,000-case campaign clean; disclosure comment retired.
- Committed reproducer now fails at encode.
- Existing-repository compatibility and identity neutrality each evidenced.
- Error taxonomy uniform across the four kinds.
- Test counts reported before/after (`prikk-object` 72, `prikk-store` 540 at this baseline).
- Frozen identities unchanged — including `Cargo.lock` `601d0678…5da31`; this increment adds no dependency.
- Full gate set green (`rfcs/EXECUTION-ORDER.md` §6.8).

## Submit with

Diff; evidence note covering the compatibility check, the identity-neutrality statement, the error-taxonomy
choice and its rationale, and the unfiltered campaign result; gate output; explicit statement that the
decoder, `RepoPath` grammar, wire format, and opaque target fields are unchanged.
