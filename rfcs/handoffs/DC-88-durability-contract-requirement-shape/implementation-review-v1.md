# DC-88 — Implementation Review v1

**Reviewing:** `ed04c21` on `dc-88-durability-contract-requirement-shape`, off `main`.

**Verdict: ACCEPT. No conditions.** Merges after a green three-platform CI run, which the standing rule
binds here because this touches filesystem-backed state.

Six files, +76/−18, and the increment does exactly what the ruling scoped and nothing else.

## 1. The change is better pinned than they claimed

They verified their new conformance test fails when `linux.rs`'s fix is reverted. I reproduced that and
went wider — reverting the fix and running the whole `prikk-store` suite fails **five** tests, not one:

```
fsutil::tests::conformance::durable_directory_entry_accepts_the_named_files_own_path   FAILED
fsutil::caller_tests::sync_matrix::worktree_parent_sync_failure_is_repaired_before_unchanged_success  FAILED
patch_checkout::tests::patch_materialization_is_idempotent_for_same_bytes              FAILED
patch_checkout::tests::patch_deletion_retry_resyncs_observed_absent_parent             FAILED
snapshot::tests::snapshot_materialization_is_idempotent_for_same_bytes                 FAILED
```

The four pre-existing ones fail through the *callers*, which now pass a file path the reverted
implementor would treat as a directory. So the restatement is held by the new interface test **and** by
the existing caller-level suite from both directions. Worth stating because it is stronger evidence than
the report claims for itself.

## 2. The edge case the new test does not cover — checked, and clean

The new test uses a nested path (`nested/object`). **The case where old and new parent resolution could
most plausibly diverge is a file at the repository root**, where `relative.parent()` is `Some("")` rather
than a real directory — and that is the common case in a real worktree (`README.md` at repo root goes
through `materialize_entry` exactly this way).

Old callers computed `relative.parent().unwrap_or_else(|| Path::new(""))`. The new implementors call
`required_parent(relative)?`, which is `path.parent().ok_or_else(...)` (`regular.rs:18-21`). For
`"README.md"` that is `Ok("")` — identical. `None` is only reachable for a path that is a root or
prefix, which `validate_relative` already rejects.

**I did not stop at reading `Path::parent()`'s semantics.** I added a probe test for a
repository-root file and ran it against `LinuxDurability`: passes. The behaviour is equivalent, measured
rather than reasoned.

**Non-blocking suggestion, not a condition:** the top-level case is worth folding into the conformance
test as a second assertion, since it is the one a future refactor of `required_parent` would break
silently and the nested case would not catch. It is a two-line addition whenever this file is next
touched; I am not holding the increment for it.

## 3. Scope discipline

- `none.rs` untouched — it errors unconditionally regardless of parameter meaning. Correct not to touch
  it.
- **DC-88 §3's two-slot shape was not implemented.** The ruling reattributed it to DC-87 Stage 2's
  inputs, and they respected that rather than treating a reassigned sketch as licence.
- No method-set change, no `MutationRoot` change, no dispatch change. The one method's parameter
  *meaning* moved; its type did not.
- I swept `docs/src` and `rfcs/accepted` for other statements of the old directory-scoped semantics.
  The only `docs/src` hit is `platform-support.md:28`, which names `sync_directory_required` in a list
  of mutation-set functions and never describes its parameter — accurately unaffected, as they said.
  The RFC hits are DC-87/DC-88/DC-90's own historical text, which is meant to read as of its own date.

## 4. The contract text

The module doc now says the method *was* the worked example and *was itself the one that missed the
bar*, with the reason — no caller wanted batching; every other method bundles its own transition-scoped
sync. It keeps the `fsync`-versus-`fcntl_fullfsync` divergence intact and correctly frames it as what
*satisfies* the guarantee rather than what the guarantee *is*. That distinction is the whole point of
DC-76's thesis and it survives the edit.

The conformance table's new row states plainly that the test is a parameter-resolution check and **not**
a G3 durability-under-crash proof. That is the honest framing, and it is the standard I asked for in
DC-90 applied here without being asked.

## 5. Gates, re-run by me at `ed04c21`

fmt clean; clippy `--workspace --all-targets --all-features --locked -D warnings` clean;
`cargo test --workspace --locked` green; `cargo +1.85.0 test --workspace --locked` green; **607
prikk-store lib tests** (606 + 1); `git diff --check` clean; `cargo audit --no-fetch` nothing flagged;
release-policy `check` 154 oracle cases, `boundary-check` and `reference-check` both `"valid": true`;
`mdbook build docs` clean. **Cross-target clippy for `x86_64-pc-windows-gnu` and
`x86_64-apple-darwin`: both clean** — required here, since `linux.rs`/`macos.rs`/`anchored.rs` are
`#[cfg(target_os)]`-gated.

## 6. Standing

- **Merges after a green three-platform CI run.** Filesystem-backed state; the standing rule binds.
- **DC-87 Stage 1's seam refactor** is available and unblocked.
- **DC-87 Stage 2** waits on DC-90 landing (before any `unsafe`) and on its own design answering how
  `atomic_replace`/`promote`/`durable_append` are satisfied without directory durability — the question
  this increment's investigation located correctly.
