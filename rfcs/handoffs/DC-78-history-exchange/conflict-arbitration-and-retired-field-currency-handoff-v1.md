# Two rulings landed; four documents still describe the world before them

**Base:** current `main` (`7e6e90e`). **Under `003-landing-work-on-main.md`.**
**Origin:** checking what themes remain, and finding that two settled decisions never reached the
documents that state them. **Both omissions are mine.**

---

## 1. The conflict-arbitration theme was never closed in the ROADMAP

DC-21 ruled that **automatic conflict resolution is refused by design**, and the ruling **did** land —
`docs/src/reference/patch-algebra.md:166` carries *"Conflict Resolution Is Refused By Design"*.

**`ROADMAP.md:84` is untouched.** It still poses *"The question that decides the design:"* as open —
the question DC-21 answered — and closes with *"Depends on merge execution existing. Not scoped."*
**Merge execution shipped in `0.19.0`**, and it will never be scoped as an arbitrator.

**My DC-21 handoff header said "Closes: the ROADMAP 'Conflict arbitration' theme" and I never
instructed the deletion or checked it.** I have told you repeatedly to delete a theme when its work
lands; I did not hold myself to it.

**Delete the section**, per the precedent set by patch aggregation (`10a2a13`), structured output
(`3717220`), MSRV, and repository layout. **The substance now lives in `patch-algebra.md`, which is
the right home for a user-facing ruling** — a refused theme does not belong in a plan.

**Adjudicate one thing**: whether anything should point a ROADMAP reader at the ruling, or whether
deletion alone is right. **A reasoned "delete outright" is acceptable** — the four precedents did
exactly that.

## 2. `merge.md` lists a refused theme as merely deferred

`docs/src/guide/merge.md:98`, under **`## Deferred`**:

```
- Conflict arbitration / resolution.
```

**"Deferred" means eventually built. This will not be built.** DC-21's own report drew exactly this
distinction when it corrected `patch-algebra.md`'s deferred list — **and `merge.md` was missed.**

**Apply the same correction here**: move it out of `Deferred`, and say plainly that it is refused by
design, pointing at the reference page.

## 3. The same list describes a field that no longer exists

Same `## Deferred` list, `merge.md:101`:

```
- Populating `PatchPayload.parent_patch_ids` — no construction site sets it; …
```

**`parent_patch_ids` was deleted in `0.24.0`** at `Patch` schema 2. It is not unpopulated — **it is
gone.** `patch.rs` mentions it only in comments explaining its retirement.

**Three more in `docs/src/reference/data-model-lifecycle.md`**, all describing it as present-but-inert:

- **`:60`** — *"`Patch → Patch` **exists in the format** as `parent_patch_ids` but is **inert**"*
- **`:63`** — *"an incoming Patch carrying a non-empty `parent_patch_ids` is …"*
- **`:289`** — *"**No patch DAG.** `parent_patch_ids` is inert — every construction site sets it empty"*

**All four are false.** The field does not exist in the current format.

**Re-derive the full list yourself** — `grep -rn "parent_patch_ids" docs/src/` — and report anything
beyond these four.

**This is my omission too**: I authored the handoff that deleted the field and never instructed a docs
sweep for it. **It shipped in `0.24.0` and has been false through two releases.**

**Do not simply delete these sentences.** They answer a real question — *why is there no patch DAG?* —
and that question survives the field's removal. **The answer is now stronger, not absent**: there is no
patch DAG because the field was **removed**, not merely left empty. **Say the true thing; do not leave
a hole where an explanation was.**

## 4. Out of scope

- **The `### Sync` heading** (`ROADMAP.md:29`), which still reads *"prerequisite is a threat model"*
  over an accurate body. **Stale in the same class. Report it, do not fix it** — it is a separate
  decision and I would rather it be visible than bundled.
- **Any code change.** All four sites are documentation.
- **`rfcs/done/`** and other historical records, which correctly describe what was true then.
- **Rewriting `patch-algebra.md`**, which is already correct.

## 5. Controls

1. **No live document claims `parent_patch_ids` exists** — show it mechanically across `docs/src`,
   `README.md`, and `ROADMAP.md`, excluding historical `rfcs/`.
2. **No live document lists conflict arbitration as deferred or pending** — same, mechanically.
3. **`mdbook build` clean**, and every link on the touched pages still resolves.
4. **Full gate set green, test count unmoved** — this is documentation only.

**If the count moves, something other than documentation changed. Stop and say so.**

## 6. What to report

1. **Your re-derived `parent_patch_ids` list** (§3), including anything beyond my four.
2. **Your §1 adjudication** — delete outright, or leave a pointer.
3. **The `### Sync` heading**, reported not fixed (§4).
4. All four controls (§5), quoted.
5. **Full gate set against the exact commit, after the last edit**, including `mdbook build`.
6. **Every numbered requirement's disposition, including ones that went without incident.**
7. Anything here was wrong.

**Stop and escalate, do not guess**, if: a `parent_patch_ids` reference turns out to describe
something still true that I have misread — **that would mean the field's removal was narrower than the
`0.24.0` CHANGELOG claims, and that outranks this increment.**
