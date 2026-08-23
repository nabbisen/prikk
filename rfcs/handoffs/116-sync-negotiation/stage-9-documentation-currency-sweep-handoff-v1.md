# Documentation currency sweep: implementation handoff

**Base:** current `main` (`5307c34`). **Documentation only.**
**Origin:** the RFC 116 stage 8 review. Writing one guide page turned up **four** reference sentences
still calling `sync` deferred, eight stages after it landed — and the same four sentences also call
*production merge execution* deferred, which `guide/merge.md` has contradicted since DC-74. **Two stale
features in the first four lines anyone happened to read means the problem is not sync-shaped.**

**This is the first increment under `003-landing-work-on-main.md`:** work on `main`, commit locally,
**do not push**, report the SHA.

---

## 1. The deliverable is an enumeration with a verdict per line — not a diff

A sweep proved by its diff is a sweep that fixed what it noticed. **Thoroughness here has to be
checkable**, so the deliverable is:

> **every candidate line, listed, each with one of three verdicts and a one-line reason.**

The diff falls out of that. A short diff with a complete enumeration is a good result; a long diff with
no enumeration is not.

## 2. Scope — mechanically defined, so it is not a judgment call

```sh
grep -rniE "deferred|not implemented|not yet|planned|unimplemented|remains? open" \
  docs/src/reference/ docs/src/guide/ docs/src/index.md
```

**72 lines at time of writing.** That set is the scope. If your run finds a different number, say so and
say why before proceeding.

**`README.md` and `ROADMAP.md` are out of scope** (§6) — they have their own audiences and their own
review history, and mixing them in makes this unreviewable.

## 3. The hazard: "deferred" is a technical term in one of these pages

**`reference/patch-algebra.md` uses `Deferred` as a real classification**, matching
`UnknownReason::SameNodeTextCommutationDeferred` and friends in `patch_algebra/types.rs`. Lines like

> `| same_node_text_transform_deferred | Same-node text operational transforms are intentionally deferred. |`

are **correct and current**, and a grep-and-fix pass would corrupt that page.

**This is the same discrimination the stage 8 work already had to make** between `sync`-the-feature and
`fsync`/durability mentions. **Distinguish per line, individually. Do not pattern-match.**

## 4. The three verdicts

- **`STALE`** — the claim was true and the feature has since shipped. **Correct it**, with an anchor
  (§5).
- **`CURRENT`** — still accurate. **Leave it untouched** and say what makes it still true. Several will
  be: `architecture.md:169`'s note that commit cost is not bounded independently of repository size is
  still real (criterion 3's row records `seal`'s O(N)-per-call residual explicitly).
- **`TERM`** — not a claim about roadmap state at all, but a technical word (§3). **Leave untouched.**

## 5. Corrections carry anchors, like everything else in `docs/`

`reference/` pages state facts about the system. A correction that replaces one unsourced claim with
another unsourced claim has not improved anything.

**Each `STALE` correction names what makes it true now** — the increment, the merge SHA, or the file.
Follow `guide/sync.md`'s own table shape if a page already has one; otherwise an inline citation is
enough. **Do not add an anchor table to a page that has none** — that is a larger restructuring and not
this increment.

## 6. Three already confirmed `STALE`, to seed the list — not to bound it

I verified these while scoping; **they are examples, not the answer.**

1. **`reference/architecture.md:158` — and take this one first.** It says *"Repository-wide **author**
   trust verification is not yet implemented, so a patch's author signature is carried and preserved but
   not checked repository-wide by `verify`."* **DC-53 shipped exactly that**, and badge criterion 5
   records it MET: `verify` cryptographically checks every reachable Patch's AUTHOR signature.
   **This page currently understates the security prikk provides.** Understating is the safer
   direction, but it is still wrong, and it is the kind of sentence a security-conscious reader acts on.
   **Carry criterion 5's stated limit across with it** — this is trust-on-first-use continuity, not
   first-contact authenticity — because a correction that overshoots into overclaiming would be worse
   than the stale line.
2. **`reference/patch-algebra.md:138-139, 160`** — *"merge execution is not implemented"*, *"Still
   deferred: `prikk merge`, merge execution…"*. DC-74 shipped `prikk merge`.
3. **`reference/data-model.md:20, :126` and `reference/repository-layout.md:32, :319`** — the sync and
   merge mentions stage 8 already found.

## 7. The completeness check — this increment's substitute for a control

There are no tests here, so thoroughness needs its own proof:

**After the corrections, re-run §2's command. Every remaining hit must appear in your enumeration with
a `CURRENT` or `TERM` verdict.** A hit that is neither corrected nor adjudicated means the sweep missed
it, and that is the failure mode this check exists to catch.

Report the before and after counts.

## 8. Out of scope

- **`README.md`, `ROADMAP.md`, `MILESTONES.md`, `rfcs/`.** Different audiences, different review paths.
  **`MILESTONES.md` is the architect's alone** — if you find it stale, report it, never edit it.
- **Adding anchor tables to pages that lack them** (§5).
- **Any code change.** If a doc is stale because the code is wrong rather than the doc, **stop and
  escalate** — that is a defect, not a documentation finding.
- **Rewriting a page's structure.** Correct sentences; do not reorganise.

## 9. What to report

1. **The full enumeration** — every candidate line, its verdict, and a one-line reason. This is the
   deliverable; the diff is secondary.
2. **§7's before/after counts**, and confirmation that every remaining hit is adjudicated.
3. Anything you judged `CURRENT` that you were unsure about. **A doubtful `CURRENT` is more dangerous
   than a doubtful `STALE`**, because it leaves a wrong claim standing with a verdict attached — say
   which ones you were least confident in.
4. The **full gate set against the exact commit, after the last edit** — the standard nine.
   **`reference-check` matters most here.** Run `mdbook build` too, as stage 8 did.
5. Test counts before and after — **expected unchanged**.
6. Anything here that turned out to be wrong. **Say so plainly.**

**Stop and escalate, do not guess**, if: a claim is stale because the **code** is wrong (§8); a
correction would require asserting something you cannot anchor; or the enumeration's size differs
materially from §2's 72.
