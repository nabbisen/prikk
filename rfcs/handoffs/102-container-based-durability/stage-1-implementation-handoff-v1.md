# RFC 102, Stage 1 — Implementation Handoff v1

**Cleared to implement Stage 1 only.** Design: `design-v1.md` §7, owner-accepted 2026-08-13.
**Stages 2–6 are not authorized. No container is built in this stage.**

## 1. What Stage 1 is, and why it comes first

Two pieces, neither of which changes any storage format:

1. **The unclean-shutdown marker**, closing T12 — today a worktree file whose name failed to become
   durable is re-authored and **signed** as a deletion the user never made.
2. **The WAL-at-`init` fix** — RFC 101 §5.1's orphaned change, homeless since RFC 101 closed.

**This stage delivers safety before the RFC delivers Windows parity, and stands alone if the rest is
never built.** That is deliberate.

## 2. The marker — construction is the whole risk

**Created at `init`. Set by appending a sentinel. Cleared by `durable_truncate_to_empty`. Never
`atomic_replace`.**

§6.5 established that `atomic_replace` creates a temp name and `renameat`s it, **even over an existing
destination** — a new-name event whose Windows durability is DC-87 §3.4's still-open question. **Every
comparable small state file in this codebase uses `atomic_replace`**, so the wrong construction is the
natural one to reach for. It would silently reintroduce the exact gap this RFC exists to close.

**Ordering:** set the marker *before any worktree write begins*; clear it *after materialization
completes*. A crash before the set means no worktree write happened — nothing to falsely infer. A crash
during the clear leaves it dirty, which is the safe direction: a spurious refusal, never a missed dirty
state.

**While dirty**, commit-authoring refuses to infer deletion until the worktree is re-verified against its
baseline. The inference has exactly one choke point — `worktree_patch/node_authoring.rs:441-446` — and
§6.5 found no competing worktree-write path. **Confirm both still hold before relying on them.**

`patch_checkout.rs`'s explicit deletions are a *second* mutation path, structurally different (they verify
`old_bytes` before removing). §6.5 flagged that they should get the same bracketing. **Decide and report
which, rather than silently covering one and not the other.**

## 3. The WAL-at-`init` fix

Move `queue.wal`'s creation from first append to `init`. Its acceptance evidence is RFC 101 §5.1's —
behaviour-neutral, because every reader treats a missing WAL and an empty WAL identically.

**That evidence is inherited, not re-derived. Confirm the reader-equivalence claim still holds** before
relying on it; the code has moved since, and an inherited proof is the kind that goes stale unnoticed.

## 4. Out of scope

No containers, no index, no framing changes, no read-path changes. If the marker appears to need any of
them, that is a finding to report — not a scope to absorb.

## 5. Acceptance criteria

1. **The marker is never written with `atomic_replace`**, provable by inspection of the call.
2. **A dirty marker blocks deletion inference** — construct the state, show commit-authoring refuses.
   The assertion is the *refusal*.
3. **A crash during the clear leaves it dirty**, not clean.
4. **The WAL exists after `init`**, and every existing WAL test still passes unchanged.
5. Full gate set per `EXECUTION-ORDER.md` §6 rule 9, plus **green three-platform CI**.

## 6. Standing

A stop-and-report is a complete outcome. Stage 1 merges before Stage 2 is scoped.
