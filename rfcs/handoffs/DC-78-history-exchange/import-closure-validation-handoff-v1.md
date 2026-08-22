# DC-78 — `import_bundle` closure validation: implementation handoff

**Origin:** `.git-exclude/reviewed/DC-78-bundle-tag-gap-implementation-review-v1.md` §5, recorded open
since 2026-08-20 and unowned until now.
**Base:** current `main` (`222bd0a`).
**Precedent to follow:** `accept_exchange_artifact` — **it already does this correctly, and this
increment makes `import_bundle` match it.**

---

## 1. The gap

**`import_bundle` validates counts and byte limits, and nothing else.** It does not check that the
objects a bundle references are actually present. Its only `contains_object` call is the write-dedup
counter.

So a bundle carrying a RefState whose target object it never shipped **imports successfully** and lands
a dangling received pointer. Since RFC 115 Stage 3 added the received-namespace verify stage, that
dangling ref is now *visible* — but only at the next `verify`, long after the import that caused it and
with no indication of which bundle was at fault.

**`accept_exchange_artifact` refuses the same class of defect at receipt** (`accept.rs:186-190`: a blob
referenced by a carried patch must be in the artifact or already local, else the whole exchange is
refused). **Two receiving paths, one rule — and only one of them follows it.** This is the same
inconsistency D7 corrected between Stage 3 and Stage 4, and the fix is the same shape: make the
outlier match the path that already gets it right.

## 2. What to validate

Before **any** write, for the decoded bundle:

1. **The exported ref's target resolves.** Reuse `ensure_ref_target_valid`
   (`refs/verify/scan.rs`) — it is already `pub(crate)`, widened in RFC 115 Stage 3, and it is already
   kind-aware: `Branch` → a Block; `Tag` → a Tag object whose own `target_block_id` is a Block.
   **Do not write a second version of that logic.**
2. **Every blob referenced by a carried patch's operations** is present — in the bundle or already in
   this repository. Mirror `accept.rs`'s check, including the "or already present locally" half.
3. **Every patch named by a carried block** is present, same rule.
4. **Every parent named by a carried block** is present, same rule. A bundle is genesis-complete by
   construction (`export_bundle` walks the full ancestor closure), so this holds for anything the
   current exporter produces; checking it is set membership and costs nothing.

**"Present" always means: carried by this bundle, or already in this repository.** An incremental
import onto a repository that already holds part of the history must not be refused for objects it
already has — that is D7's rule again (already-held is fine; absent is the refusal).

## 3. Ordering — refuse before writing anything

Validate everything in §2 **before the first object write.** `import_bundle` currently writes objects,
then records author key material under `ActiveLock`, then writes the received pointer.

The pointer is what makes a dangling ref observable, and it is written last — so validating up front
means a refused bundle leaves **no pointer at all**, which is the outcome that matters. Do not
reorder the existing write phases; only insert validation ahead of them.

## 4. This is an intentional behaviour change, and one case is worth naming

**A bundle that previously imported may now be refused.** That is the point — silently landing a
dangling ref is the defect. But say it in the module doc, because "it used to work" will otherwise read
as a regression.

**The concrete case: a tag bundle produced before the DC-78 fix** (`d605c10`) carries the RefState but
not the Tag object. Under this change such a bundle is **refused at import** rather than importing and
failing `verify` afterwards. That is strictly better — the failure now names the bundle, at the moment
it is offered — and it should have its own test.

**`PBNDL001` bundles are still accepted on import** (`RETIRED_BUNDLE_MAGIC_V1`) and are genesis-complete
by construction, so validation must not refuse a well-formed one. Test that explicitly; a closure check
that broke retired-format import would be a real regression.

## 5. Tests and controls

Each needs a test **and** an observed-failing control.

| # | Property | Control |
|---|---|---|
| 1 | A bundle whose ref target is absent is refused | Drop the `ensure_ref_target_valid` call → the pre-DC-78-shaped tag bundle imports |
| 2 | A bundle missing a referenced blob is refused | Drop the blob check → imports with a dangling blob reference |
| 3 | A bundle missing a block's patch is refused | Drop the patch check → imports |
| 4 | A bundle missing a block's parent is refused | Drop the parent check → imports |
| 5 | Objects already held locally satisfy "present" | Require presence *in the bundle* only → an incremental import of a partial bundle is wrongly refused |
| 6 | A refused import writes **no received pointer** and records **no key material** | Move validation after the writes → the pointer exists after a refusal |
| 7 | A well-formed bundle still imports, `PBNDL002` **and** `PBNDL001` | — (regression guard; assert both) |

**Row 5 is the one most likely to be got wrong**, and getting it wrong turns a correctness fix into a
usability regression: incremental import onto a repository that already holds part of the history is
the ordinary case, not an edge case.

**Row 6 is the one whose failure would be invisible.** Assert the received index and the author-key
container are byte-identical to before the refused import — not merely that the call returned `Err`.
The Stage 3 review's row 1 is the model: pre-seed the receiver so "unchanged" is a real comparison
rather than an empty-to-empty one.

## 6. Out of scope

- **`export_bundle`.** It is already genesis-complete; this validates what arrives, not what leaves.
- **The received-namespace verify stage.** Already landed in RFC 115 Stage 3; this makes it a
  second line of defence rather than the first.
- **Any bundle format change.** `PBNDL002` stays as it is, `PBNDL001` stays accepted on import.
- **`accept_exchange_artifact`.** It is the reference implementation here. **Do not "align" it to
  whatever you write; align to it.**

## 7. What to report

1. Control output for every row of §5 — actual failure text, and the single line mutated.
2. **For row 6:** what you compared byte-for-byte, and how you pre-seeded the receiver so the
   comparison is meaningful.
3. **Whether a pre-DC-78-shaped tag bundle can actually be constructed in a test** — if it cannot,
   §4's headline case is untestable and I want to know rather than have it quietly dropped.
4. The **full gate set against the exact commit, after the last edit**: `cargo fmt --all -- --check`;
   `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
   `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`; `git diff --check`;
   `cargo audit --no-fetch`; release-policy `check` / `boundary-check` / `reference-check`.
   Cross-target clippy pair only if this diff contains `#[cfg(target_os)]`.
5. Test counts before and after, per crate. **`snapshot.txt` must not change.**
6. Anything here that turned out to be wrong. **Say so plainly.**

**Stop and escalate, do not guess**, if: `ensure_ref_target_valid` cannot be called from `bundle.rs`
without widening something further; validating up front turns out to need the objects already written
(which would mean §3's ordering is wrong); or any existing bundle test starts failing for a reason
§4 does not predict — **that would mean the current exporter produces bundles this check refuses**, and
I need to know before it lands.
