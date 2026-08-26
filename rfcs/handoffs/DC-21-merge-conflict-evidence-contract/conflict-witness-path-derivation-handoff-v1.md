# Conflict witnesses — stop discarding path information

**Base:** current `main` (`043192d`, `0.25.0` released). **Under `003-landing-work-on-main.md`.**
**Origin:** the gap found reviewing DC-21 — `DeleteMutationConflict` passes `None` for path while its
`DeleteFile` operand carries a real `RepoPath`.

**It is not one site.** I measured before writing this.

---

## 1. The measurement

Of the `conflict(...)` / `conflict_with_span(...)` call sites I could match mechanically —
**18 sites, 9 pass `None` for path**:

| File | Kinds passing `None` |
|---|---|
| `classify.rs` | `LiveStateMismatch`, `UnknownRelation` |
| `create.rs` | `ModeMismatch`, `NodeIdReuse`, `BlobMismatch`, `KindMismatch` ×2 |
| `delete.rs` | `DeleteMutationConflict` |
| `text_pair.rs` | `LiveStateMismatch` |

**This is a lower bound.** `preimage.rs`, `text_preimage.rs`, and `witness.rs` build witnesses by
**struct literal** rather than through the helper, and my pattern did not match those. **Re-derive the
full list yourself and report it** — the real number is higher than nine.

## 2. `Action::path()` is the wrong primitive — do not build it

The obvious fix is an exhaustive `Action::path() -> Option<&RepoPath>`. **It does not work.**

Only four `Action` variants carry a `path` field: `CreateFile`, `DeleteFile`, `DeleteSymlink`,
`CreateSymlink`. **`RenamePath { node_id }` carries no path at all** — the operation most about paths
has none in its own variant, because a rename's paths live in its effects, not its identity.

**`PathEffects` is the only complete source**, and **every** `OperationFacts` has one:
`occupied_before`, `required_free`, `occupied_after`, `freed`, `newly_occupied` — all
`BTreeSet<RepoPath>`. Whatever a witness reports must come from there.

## 3. The design question: one path, or one per side?

**Adjudicate this and say why. It is the whole increment.**

A witness carries `path: Option<RepoPath>` — **one** path for a conflict between **two** operations.
That is fine when both operands touch the same path, and wrong when they do not:

- `NodeIdReuse` — two `CreateFile`s at **different** paths reusing one node id. **Which path is *the*
  path?** Neither. A single field cannot say what the user needs.
- `DeleteMutationConflict` — a delete and a permission change on one node. **One** path, available from
  the delete side.

**My lean: derive per side**, so the witness reports what each operand touched and never has to pick.
**But price it before you adopt it**: `MergeEvidenceDisplayItem::witness_path` became public **in
`0.25.0`, released today**. Changing its shape is another breaking change one release later.
**Prefer additive** — leave `witness_path` meaning what it means and add per-side fields — **or argue
that the churn is worth it.**

**If you conclude a single derived path is genuinely sufficient for every kind, say so with the
`NodeIdReuse` case answered**, not skipped.

## 4. Property first, sites second

**Write the assertion before you touch any call site.**

> For any conflict witness whose operands touch at least one path, the witness reports at least one
> path.

**Run it. Report exactly which kinds fail today** — that list is the increment's real scope, and it
will not match my nine. **Then** fix the sites, deriving from `PathEffects`.

**A site that legitimately has no path must stay `None`** — and the property must be written so that
such a case passes honestly rather than being excluded by hand. **If you cannot express that
distinction, stop and say so**; a property with a hand-maintained exclusion list is the defect this
project keeps removing.

## 5. Out of scope

- **Changing what any conflict *is*.** Classification is untouched: same kinds, same pairs, same
  outcomes. **Only what the witness reports changes.**
- **`ConflictWitnessKind`** and its labels.
- **Making `patch_algebra` public.**
- **The merge refusal message** and `verify`'s JSON.
- **`node_id`**, which already works.

## 6. Controls

1. **The property fails before the fix and passes after** — quote both runs, and quote the failing kind
   list.
2. **A genuinely path-less conflict still reports `None`** — construct one, show the property passes
   without an exclusion for it.
3. **Classification is unchanged** — every existing `patch_algebra` test passes untouched, and say so
   explicitly rather than letting the suite total imply it.
4. **The public surface** — state exactly which public fields changed and whether anything downstream
   breaks (§3).
5. **Full suite green**, count moved and why.

**Quote every failure.** If a control passes without your assertion firing, say so.

**After any control that deliberately fails a property test, check `proptest-regressions/` for a seed
your probe added** — that bit us in stage 6 and the file is tracked.

## 7. What to report

1. **Your re-derived list** of sites discarding an available path (§1), including the struct-literal
   ones I could not match.
2. **Your §3 adjudication**, with the `NodeIdReuse` case answered and the public-API cost priced.
3. **The property**, and how it distinguishes "no path exists" from "path discarded" without a
   hand-maintained exclusion list.
4. All five controls (§6), quoted.
5. **Full gate set against the exact commit, after the last edit.**
6. **Every numbered requirement's disposition, including ones that went without incident.**
7. Anything here was wrong.

**Stop and escalate, do not guess**, if: the property cannot be expressed without an exclusion list
(§4); deriving a path requires `PathEffects` to become public; or the honest fix needs
`witness_path`'s public shape to change — **that is a breaking change one release after the last one,
and it is my call, not yours.**
