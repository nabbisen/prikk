# RFC (proposed) - DC-73 Node-Model Operation Apply

**Status.** **Proposed 2026-08-04.** Awaits owner acceptance.
**Authored by** the architect. **Independence.** Author-reviewed — the standing ceiling.
**Arises from.** The forward-roadmap proposal accepted 2026-08-04, item A.
**Requirement.** None names it. It closes three user-visible failures against capabilities the object
format already defines.

## 1. What is actually missing — narrower than the roadmap said

**Correction, made before scoping rather than after.** The roadmap proposal implied these operations were
unimplemented generally. They are not. `lifecycle_cache/replay/effect.rs:3-4` states that `CreateFile`,
`CreateSymlink`, `DeleteNode`, `RenamePath`, `ChangePerm`, `ReplaceBinary`, and `EditText` are **all exact**
— lifecycle-state application is done for every one.

The gap is in **two specific layers**, both marked "increment 4.4" in the code:

| Layer | State |
|---|---|
| Lifecycle-state apply (`effect.rs`) | **complete for all seven** |
| **Checkout materialization** (`patch_replay/decode.rs:135-143`) | `RenamePath`, `ChangePerm`, `CreateSymlink` return `unsupported_operation` |
| **Patch inverse** (`patch_inverse.rs:304-307`) | `ReplaceBinary`, `RenamePath`, `ChangePerm`, `CreateSymlink` return `UnsupportedObjectType` |

So this increment is about **writing these operations to a filesystem** and **inverting them** — not about
the node model, despite the marker's name.

**Authoring asymmetry, which matters for scope:** `ReplaceBinary` and `ChangePerm` are already *authored*
by `commit` (`node_authoring.rs:391,406`), so repositories containing them exist today and cannot be fully
checked out or rolled back. `RenamePath` is never authored (renames become delete+create) and
`CreateSymlink` authoring is refused outright — so those two are unreachable in practice, and their apply
paths are currently dead.

## 2. What this closes

- **Rollback refuses any span containing `ReplaceBinary` or `ChangePerm`** — reachable today.
- **`checkout --patch-materialize` cannot replay `ChangePerm`** — DC-67's finding, reachable today.
- Symlink and rename support become *possible* later, but are not delivered here (§5).

## 3. What must be established before designing — blocking

| Question | Why it blocks |
|---|---|
| **Which of the four are reachable today?** `ReplaceBinary` and `ChangePerm` appear authorable, the other two not — confirm by attempting to author each | Scope should not spend effort on apply paths nothing can produce |
| **Does `checkout` materialization already handle `ReplaceBinary`?** DC-67 reported both it and `ChangePerm` failing, but `patch_replay.rs:5` says `ReplaceBinary` *is* reconciled | The two sources disagree. **Resolve it by running a checkout against a real binary edit**, not by reading |
| **What does inverting `ChangePerm` require?** The prior mode must come from somewhere — the tombstone, the baseline state, or the operation itself | Determines whether inverse is a small addition or needs new recorded data |
| **Is `RenamePath` inverse meaningful given renames are never authored?** | If nothing produces it, implementing its inverse is dead code with a maintenance cost |

**The second is the one I would get wrong.** Two in-tree sources contradict each other about
`ReplaceBinary`; only execution settles it.

## 4. Acceptance criteria

1. §3's four questions answered and reported **before** a fix is designed.
2. **Reachable operations first**: `ChangePerm` and `ReplaceBinary` materialize on checkout and invert for
   rollback, tested end to end through the compiled binary at N ≥ 3 sealed generations, on DC-67's pattern.
3. Each test ends by **rebuilding the worktree from sealed history and asserting byte-exact content**,
   including the mode bit for `ChangePerm`.
4. Unreachable operations (`RenamePath`, `CreateSymlink`) are **either implemented with a stated reason, or
   left unimplemented with their marker updated** to say they await an *authoring* path rather than the node
   model. **Do not leave a stale deferral marker naming the wrong blocker.**
5. `MILESTONES.md`'s checkout-replay-gap row closed or narrowed; the rollback limitation in
   `docs/src/guide/rollback/` updated to match what now works.
6. No object format change, no new operation kind, no identity movement.
7. Full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9, plus test counts before and after.

## 5. Non-goals

- **Symlink authoring.** Refused by `node_authoring` for its own reasons (FDD-04 §5.4a); this increment does
  not open it.
- **Rename authoring.** `commit` produces delete+create by design; changing that is a separate question.
- **Rollback authorization by policy** — documented as absent, unowned, and not this.
- Bounding rollback's lineage walk — DC-69's horizon question.
