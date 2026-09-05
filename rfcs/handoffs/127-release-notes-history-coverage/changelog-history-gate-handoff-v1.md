# Restore 0.23.0's changelog heading, and gate every released tag — implementation handoff

**Authority:** `rfcs/done/127-release-notes-history-coverage.md`.
**Base:** current `main` (`94b6cb7`). **Under `003-landing-work-on-main.md`.**
**The repository moved to `prikk-vcs/prikk` (RFC 129) — confirm your remote before you start.**

**This is the only audit finding whose damage is already published.** A released version's entry is
wrong in the record right now.

---

## 1. The defect

`0.23.0` has **no heading** in `CHANGELOG.md`. Every other released tag has exactly one:

```
0.27.1 → 1   0.27.0 → 1   0.26.0 → 1   0.25.0 → 1
0.24.0 → 1   0.23.0 → 0   0.22.1 → 1   0.22.0 → 1
```

The cause is `5964ad6` ("release: bump workspace to 0.24.0 and add the changelog entry"), which
**replaced** the heading instead of inserting above it:

```diff
-## 0.23.0 — 2026-08-23
+## 0.24.0 — <cut date, set by the owner at tag time>
```

So 0.23.0's whole body — including the entire `prikk sync` feature — now reads as 0.24.0's.

## 2. Where the boundary is — derived, not guessed

**Do not judge this by where the prose seems to change topic. Derive it from the tagged content.**

At the `0.23.0` tag (`b6cd309`), `CHANGELOG.md` has `## 0.23.0 — 2026-08-23` at line 3 and the next
heading (`## 0.22.1 — 2026-08-17`) at line 111. **0.23.0's body is that tag's lines 4–110.**

That body survives verbatim in today's file: the tag's line 5,
`**History moves between repositories.** …`, is **today's `CHANGELOG.md:255`**.

**So `## 0.23.0 — 2026-08-23` is restored immediately above today's line 255**, with a blank line
around it matching the file's existing shape. Confirm the whole body matches, not just its first
line — diff the tag's lines 4–110 against today's 254-onward and report that they are identical
(or exactly where they are not).

**Change nothing else.** Do not edit either release's text, do not re-word, do not "improve". The
only defect is a missing heading.

## 3. The gate: every released tag keeps its heading

`release_notes::assemble(root, tag, dist_dir)` (`release_notes.rs:57-95`) reads **only the section
for the tag being cut**. Its guarantee is *"the version being released has a heading"*; the failure
that occurred is *"a version released earlier stopped having one"*. Every release edits the top of
this file, so the blind spot sits exactly where the damaging edit happens.

**Add a check that every tag in the repository has exactly one matching `## X.Y.Z — DATE` heading.**

### 3.1 The CI hazard you must solve, or the gate is worse than nothing

RFC 127 §3 says to read the tags and **fail loudly when none are present**, rather than passing
vacuously. That is right, and it collides with this repository's CI:

**`.github/workflows/ci.yml` uses `actions/checkout@v7` with no `fetch-depth` and no `fetch-tags`,
so CI checkouts have _zero_ tags.** A gate that fails on an empty tag list fails every CI run; a
gate that passes on an empty tag list is the vacuous gate RFC 127 exists to prevent.

**So the CI checkout must fetch tags in the same commit as the gate** (`fetch-tags: true`, or
`fetch-depth: 0` — pick the cheaper one and say why). **Verify it in CI, not locally**: a green local
run proves nothing here, because your clone has tags and CI's does not. This is the single most
likely way this increment ships broken.

### 3.2 Where the check lives

Your call, with one constraint: it must run in the standing gate set, not only at release time — the
damage happened between releases. `boundary-check` is the natural home (it already runs under
`cargo test` via `boundary/tests.rs:6`). If you put it elsewhere, say why in the report.

**A tag that is not a version** (should any exist) must not fail the gate for the wrong reason —
decide and state how the tag list is filtered, per `prikk-release-tag-convention`'s unprefixed
`X.Y.Z` shape.

## 4. Controls

1. **The restored section, quoted before and after**, plus the tag-vs-today body diff from §2.
2. **The gate failing on the real defect.** Delete the restored heading again in a scratch worktree,
   run the gate, show it fail naming `0.23.0`; restore, show it pass. **A gate that has never been
   seen to fail is not evidence.**
3. **The gate's behaviour with no tags**, demonstrated — whichever way you resolve §3.1, show what
   happens, because that is the case CI actually runs.
4. **CI green on the pushed commit**, with the tag-fetch change in it. Name the job.

## 5. Gates

Full set from `EXECUTION-ORDER.md` §6 rule 9 against your final commit, **clippy as a single
invocation per target with the exit code captured explicitly**. `release-policy check`,
`boundary-check`, and `reference-check` all matter here — you are editing the tool that runs two of
them.

One commit on `main`, local, **no push, no tag**.

## 6. Scope

No changelog format change. No automation of changelog authoring. **No retroactive editing of any
release body.** Only the missing heading is restored.
