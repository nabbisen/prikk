# Multi-parent lineage currency — `ROADMAP.md:44` and the "fails closed" doc-comment class

**Base:** current `main` (`e9a3a50`). **Under `003-landing-work-on-main.md`.**
**Origin:** `.git-exclude/reviewed/cluster-a-dag-or-chain-investigation-v1.md`.

**Documentation and comments only. No behaviour change.**

---

## 1. `ROADMAP.md:44` — a third false theme

The bullet reads *"**Multi-parent block lineage** — deferred out of DC-74 on 2026-08-08, **not
rejected**"* and gives as its open question:

> *"under DC-74's adoption model **the patch DAG already records a merge structurally**, so block
> parentage may be bookkeeping that duplicates it."*

**Both halves are wrong.**

- **It shipped.** DC-75, 0.19.0. `merge_execute.rs:168-171` stores both parents;
  **`:176` sets `mainline_parent_id`**; `BlockPayload:63` carries the field.
- **The premise was refuted by DC-75 itself**, as its stated reason for existing
  (`DC-75-MERGE-BLOCK-LINEAGE.md:26`): *"`parent_patch_ids` is `Vec::new()` at every construction
  site... and is read nowhere. **There is no patch DAG.**"*

**This is the third false theme in this file**, after the two corrected at `970bf5f` — and it is the
escalation trigger that increment carried. **It was missed because my own handoff listed it as a theme
to adjudicate**, and `patch_replay.rs:206` genuinely does still fail closed — a true observation that
does not support the bullet's framing (§2).

**Correct it the way `970bf5f` corrected the other two**: say it shipped, when, and keep what is
genuinely still open — **which is `parent_patch_ids`' fate, an owner ruling, not a theme.**

## 2. The doc-comment class — adjudicate, do NOT sweep

**Roughly twenty-five comments across the store reference single-parent walks.** Some are accurate, some
are misleading shorthand, and **at least one describes genuinely different behaviour.** A string sweep
would corrupt the accurate ones.

**The misleading shorthand**, e.g. `patch_replay.rs:210`:

> *"Fails closed on a multi-parent lineage (v1 single-parent only)."*

**It does not.** `single_parent_chain` (`patch_replay/read.rs:48-71`) calls `mainline_or_sole_parent`,
which for a `BlockKind::Merge` **follows `mainline_parent_id`**. The refusal fires only on a merge with
a **missing or invalid** mainline — a malformed-block case, not the ordinary one.

**The model wording already exists in this repo.** `lifecycle_cache/replay.rs:190-193` states it
correctly and completely:

> *"state derivation and replay **follow the mainline only**, never the secondary parent, so the walk
> never needs to change shape for `Merge`. A `Merge` block with a missing or invalid mainline parent
> falls through to its raw (two-element) parent list, so the existing `>1 parent` guard below fails
> closed on it **exactly as it would on any other malformed multi-parent block**."*

**Use that as the pattern.** Where a comment says "fails closed on multi-parent" but the code follows a
mainline, say what `replay.rs` says.

**Known not to be shorthand — verify, then leave:**
- **`merge_evidence.rs:295`** — *"following **all** parents (DC-75; previously a single-parent-only
  walk, replaced because...)"*. **Genuinely different behaviour.**
- **`cache_ladder.rs:295`** — *"Fails closed on a merge (multi-parent) block"*. Reached via
  `walk_single_parent_chain`, which `replay.rs:190-193` documents as mainline-following. **My reading is
  that this is shorthand too — verify it rather than trusting me.**

**Adjudicate every one you touch against the function it describes.** **This handoff's own origin was me
inferring behaviour from a doc comment without reading the function** — do not repeat it.

## 3. Out of scope

- **All code behaviour.** If a comment turns out to describe the code correctly and the *code* is
  wrong, **report it — do not fix.**
- **`parent_patch_ids`' fate and patch aggregation** — owner rulings, not this increment.
- **`MILESTONES.md`, the badge.**

## 4. What to report

1. **`ROADMAP.md:44`'s correction**, and what you kept as genuinely open.
2. **Every comment adjudicated** — `ACCURATE` / `SHORTHAND` / `DIFFERENT`, with the function checked.
   **The `ACCURATE` ones are as much of the deliverable as the corrections.**
3. **Your verdict on `cache_ladder.rs:295`** specifically (§2) — including "the architect was wrong".
4. **Any comment that is accurate about code you think is wrong** (§3).
5. **Full gate set against the exact commit, after the last edit.**
6. Test counts — **expected unchanged**.
7. Anything here that was wrong, **including my ~25 count and my line numbers**.

**Stop and escalate, do not guess**, if: a walk turns out **not** to follow the mainline where §2 assumes
it does — **that would be a behaviour finding, not a comment one, and it is what I would most want to
hear**; or correcting a comment would require asserting something about merge handling no code
establishes.
