# DC-78 Handoff v2 — Addendum 4: Stage 1 reverted, one security regression, my error

**Date:** 2026-08-09. **Authored by** the architect. **Record:**
`.git-exclude/reviewed/DC-78-stage1-macos-regression-v1.md`.

## 1. First: I merged without the evidence I required of you elsewhere

**I required a green macOS CI run before merging DC-81 and DC-82, then merged Stage 1 without one.**
Local gates passed and I treated that as sufficient. DC-81 established that macOS behaviour is
**CI-only verifiable**, and Stage 1 touched filesystem-backed trust storage — squarely in that category.

**Main is reverted at `24b85bf`** and green again. Your branch is untouched.

**Standing correction:** any increment touching filesystem-backed state merges only after a green macOS
run on its branch. I will not merge on local gates alone again.

## 2. The regression — security-relevant, macOS-only

`dc72_path_safety_collisions.rs::maintainer_key_id_rejects_case_insensitive_collision` fails on macOS.

The test adds `Dev-Maintainer`, then `dev-maintainer` **with the same public key**, requiring refusal —
DC-72's guarantee for the trust key-id surface.

**`trust.rs:86-95` places the collision check inside the `None` arm only:**

```rust
Some(existing) if existing == public_key => {}   // idempotent — collision check skipped
Some(_) => { /* TOFU refusal */ }
None    => { validate_no_maintainer_key_id_collision(...)?; … }
```

**APFS is case-insensitive, so `read_existing_key("dev-maintainer")` finds `Dev-Maintainer`'s file.**
Keys match, first arm taken, **check never runs.**

**Consequence: two maintainers whose key ids differ only in case are silently conflated into one adopted
entry.** DC-72 §2 named this exact surface as where a collision could silently reduce a maintainer
threshold; with a *set*, it now merges two identities. **On Linux the same code is correct** — distinct
files, `read_existing_key` returns `None`, check runs.

## 3. Fix shape

Run `validate_no_maintainer_key_id_collision` **regardless of what `read_existing_key` returns**, not
only in the `None` arm. It already excludes exact self-matches (`existing_id != key_id` before folding),
so running it unconditionally does not break legitimate idempotent re-adds.

**Please verify that claim rather than take it from me** — I have been wrong about code I only read
before.

## 4. Why neither of us caught it, which is worth more than the fix

**DC-81 §1 asked exactly this question** — does case-insensitivity break a DC-72 guarantee — and your
answer was **"no", and correct at the time**: no raw name reached the filesystem, and repository paths
were whole-tree collision-checked.

**Trust key ids were the exception nobody re-examined.** They *are* filenames, and Stage 1 changed the
control flow around them. **A platform answer that was right when given became wrong when this landed.**

That is the lesson: **re-ask platform questions when the code beneath them moves**, rather than treating
an accepted answer as settled for the increment's lifetime.

## 5. Re-merge path — do not simply merge the branch again

`main` **contains** the Stage 1 merge and then reverts it. A plain re-merge will **not** restore the
reverted work — the trap DC-81 hit. Either revert the revert (`24b85bf`) and put the fix on top, or
rebuild your branch on current `main`. Say which you prefer and I will do my half accordingly.

**And this time the branch gets a green macOS CI run before I merge anything.**
