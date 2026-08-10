# DC-90 — Review Condition Accepted

**Reviewing:** `f358353` on `dc-90-unsafe-code-boundary-gate`, on top of the reviewed `baa4b38`.

**Accepted. No further conditions.** DC-90 closes on an ordinary CI run and merge.

## 1. The bypass is closed — verified end to end on the real workspace, not on a scratch crate

My review demonstrated the escape route against an isolated probe. That proves the language semantics;
it does not prove the fix works *here*. So I attempted the escape on a real member: added
`#![allow(clippy::undocumented_unsafe_blocks)]` to `crates/prikk-hash/src/lib.rs` and built it.

```
error[E0453]: allow(clippy::undocumented_unsafe_blocks) incompatible with previous forbid
error: could not compile `prikk-hash` (lib) due to 1 previous error
```

The route the review found is now a hard compile error in this workspace, through the real inheritance
chain — root `[workspace.lints.clippy]` → `[lints] workspace = true` → the crate. That is the property
the increment exists to have, and it is now demonstrated rather than argued.

## 2. The level itself is pinned

Reverting `SELF_GUARDING_LEVEL` to `"deny"` fails **eight** tests, including the new
`exempt_crate_redeclaring_the_lint_at_deny_still_fails` and both baseline-tree checks. The level cannot
be quietly downgraded later without a wall of red — which matters more than usual here, because
`"deny"` is the level a future reader would reach for by habit.

The pinning test asserts the *exact* error message, and asserts it is the same message a crate that
re-declares nothing gets — so `"deny"` and no-redeclaration are provably one outcome, not two. Placing
it directly above its positive counterpart is the right call: the two scenarios differ by one word and
now read side by side.

## 3. The reasoning is recorded where it will be met

Both the root manifest comment and the module doc state why `forbid` rather than `deny`, and the module
doc names the exact escape route and the `E0453` behaviour that closes it — labelled as *"found by
review, not by the original design — recorded so the mistake is not repeated."* That is the right
disposition: the next person to touch this will meet the reason before the temptation.

## 4. Gates, re-run by me at `f358353`

fmt clean; clippy `--workspace --all-targets --all-features --locked -D warnings` clean;
`cargo test --workspace --locked` green; `cargo +1.85.0 test --workspace --locked` green;
**`prikk-release-policy` 83 tests** (82 + 1); `git diff --check` clean; `cargo audit --no-fetch`
nothing flagged; release-policy `check` 154 oracle cases, `boundary-check` and `reference-check` both
`"valid": true`; `mdbook build docs` clean.

Criterion 3 still holds: the zero-exemption state passes against the real, now-`forbid` root manifest.

## 5. Standing

- **DC-90: accepted.** Ordinary CI run, then merge — no filesystem-backed state, so the three-platform
  rule does not bind this branch.
- **DC-88** (`ed04c21`): accepted, awaiting a green three-platform run.
- **DC-87 Stage 1's seam refactor:** available and unblocked.
- **DC-87 Stage 2:** unblocked on `unsafe` once DC-90 merges; still needs its own design answering how
  `atomic_replace`/`promote`/`durable_append` are satisfied without directory durability, and the
  bespoke-FFI-versus-`cap-std` choice priced against measured numbers.
