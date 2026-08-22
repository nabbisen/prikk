# RFC 117 stage 2 — resolving a patch-set digest to a local block: implementation handoff

**Design:** `rfcs/handoffs/117-tag-sync/design-v1.md` **T2. Read it in full.**
**RFC:** `rfcs/accepted/117-tag-sync.md` (ACCEPTED 2026-08-22).
**Base:** current `main` (`09abadf`). **Follows stage 1 (`babf54b`); precedes stage 3.**

**What this is for:** a received tag names a patch set. This increment answers *"which of my blocks is
that?"* — locally, with no exchange involved. Stage 3 uses it; nothing else does yet.

---

## 1. The surface

```
resolve_patch_set_digest(layout, digest: PatchSetDigest) -> Result<PatchSetResolution>

enum PatchSetResolution { NotHeld, Resolved(ObjectId) }
```

- **`NotHeld`** — no local block has that patch set. **`Ok`, not an error.** This is the ordinary "you
  have not synced that far" case and must never be a refusal.
- **`Resolved(block_id)`** — exactly one match.
- **More than one match → `Err`, naming every candidate block id.** Not a variant: T2 rules this a
  refusal, and it should be impossible for a caller to accidentally proceed on an ambiguous answer.
  **Never pick, never prefer a ref tip, never prefer the newest.**

## 2. The candidate set — ruled

**Every block reachable from any local ref, `heads/*` and `tags/*`. `remotes/*` is excluded.**

- **Tags included**: a block reachable only from a tag ref is still a block this repository holds, and
  excluding it would make a previously-tagged release unresolvable.
- **`remotes/*` excluded**: received history the operator has not sealed. Resolving a tag onto it would
  name something this repository has not adopted. Consistent with `compute_patch_set_digest_for_ref`'s
  own refusal of `remotes/` and with stage 2 of RFC 116's summary scope.

Enumerate with `RefStore::list_ref_pointers` → each tip → `merge_evidence::ancestors_inclusive`. **Do
not add a fifth ref-tip resolution helper** — that duplication is already an open finding at four
copies.

## 3. Cost — the part I most want thought about, not just implemented

The naive shape is **O(blocks × closure)**: for each candidate block, re-walk its ancestry and union the
patch ids. **That is quadratic in history length, and this project has been here before** — `verify` was
roughly O(N³) and took RFC 111 to fix (criterion 3).

**Ruled: compute in a single pass over the reachable block DAG, not per-block re-walks.** A block's patch
closure is the union of its parents' closures plus its own `patch_ids`, so processing in topological
order accumulates closures incrementally. A parent's set can be released once every child has consumed
it, which bounds memory by the DAG's width rather than its length.

**Do not add a persisted digest index.** T2 forbids it here; that is a decision for measurement, not for
this increment.

**Required in your report: the actual complexity you achieved, and a measurement** — resolve against a
repository of at least a few hundred blocks, and state the timing. **I am not asking for a cost gate
yet.** I am asking for a number, so that whether a gate is warranted becomes a decision rather than a
guess.

## 4. Ambiguity is uncommon but genuinely reachable — do not treat it as theoretical

Verified while writing this: **every block has at least one patch** — `seal` refuses an empty WAL
(`seal.rs:97`) and `merge_execute` refuses an empty adopted set (`merge_execute.rs:139`). So a child
never shares its parent's closure, and ambiguity requires **two distinct blocks with the same patch
set** — the same patches in a different order, giving a different state root and therefore a different
block id.

**That is reachable in production, not just in fixtures:** since RFC 115 Stage 4, accepted patches are
sealed locally, so two branches can seal the same accepted patch set in different orders. The refusal is
real code for a real case.

## 5. Tests and controls

Each needs a test **and** an observed-failing control.

| # | Property | Control |
|---|---|---|
| 1 | An unknown digest resolves `NotHeld`, **not** an error | Make the miss an `Err` → the not-held test fails |
| 2 | A known digest resolves to the correct block | Return the first candidate unconditionally → the test resolving a non-first block fails |
| 3 | Two blocks with one patch set **refuse**, naming both | Return the first match instead → the ambiguity test fails |
| 4 | A block reachable only from a `tags/*` ref is a candidate | Enumerate `heads/*` only → the tag-only test resolves `NotHeld` |
| 5 | A block reachable only from `remotes/*` is **not** a candidate | Include received refs → the test asserting `NotHeld` fails |
| 6 | Resolution is a single pass, not per-block re-walks | §3's measurement, reported as a number |

**Row 2 needs care.** If the fixture's correct answer happens to be the first block enumerated, the test
cannot distinguish a real match from "returns the first thing it finds" — the same trap that made a
`parent_block_ids` control a no-op in RFC 116 N3, and that row 2 of RFC 116 stage 5 had to brute-force a
fixture to avoid. **Construct the fixture so the answer is not first, and assert that fact in the test.**

**Row 3's fixture** needs two blocks with identical patch sets and different orders. Build them
directly rather than through `seal`, and assert up front that their block ids genuinely differ — if they
collide, the fixture is testing nothing.

## 6. Out of scope

- **The artifact section, the receive path, local tag creation** (design T3, T4). Stage 3.
- **A persisted digest index** (§3).
- **Any change to `TagPayload`** — stage 1 settled it; the schema is not open again.
- **Consolidating the ref-tip resolution copies.** Recorded separately at four occurrences; do not fold
  it in here, but **do not add a fifth**.

## 7. What to report

1. Control output for every row of §5 — actual failure text, and the single line mutated.
2. **§3's complexity and measurement** — the shape you achieved and a real timing at a stated size.
3. **For row 2:** how you ensured the correct answer is not the first candidate enumerated, and how you
   asserted it.
4. The **full gate set against the exact commit, after the last edit**: `cargo fmt --all -- --check`;
   `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
   `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`; `git diff --check`;
   `cargo audit --no-fetch`; release-policy `check` / `boundary-check` / `reference-check`.
   Cross-target clippy pair only if this diff contains `#[cfg(target_os)]`.
5. Test counts before and after, per crate. **`snapshot.txt` must not change.**
6. Anything here that turned out to be wrong. **Say so plainly.**

**Stop and escalate, do not guess**, if: the single-pass shape in §3 cannot be built from the existing
enumeration without a new traversal; ambiguity turns out to be unconstructible in a fixture, which would
mean §4's reachability argument is wrong; or the candidate set in §2 needs a ref kind this handoff does
not name.
