# 0.25.0 — changelog and version bump

**Base:** current `main` (`d07d54f`, CI green on all 12 jobs). **Under `003-landing-work-on-main.md`.**
**Assessment:** `.git-exclude/reviewed/release-0-25-0-readiness-assessment-v1.md`.

**You write the changelog and bump the version. You do not tag and you do not publish** — the tag and
crates.io are mine, per authorized cut.

---

## 1. The changelog entry

**Heading, exactly:**

```
## 0.25.0 — 2026-08-26
```

**The separator must be an em-dash, bytes `e2 80 94`.** `prikk-release-policy release-notes` extracts
by exact `## X.Y.Z — DATE` match and **fails the release** on anything else. **Byte-check it against
`0.24.0`'s heading before you finish** — that check has caught this before.

Follow `0.24.0`'s own shape: a short bold narrative lead, then `### Added`, `### Fixed`,
`### Breaking change`.

**The lead should say what a user can now do that they could not**: `prikk verify --format json` is
this release — the first machine-readable output the tool has ever had, and the thing that makes a CI
publication gate possible without grepping prose. Everything else is smaller.

Also worth `### Added`: conflict witness kinds now reach merge evidence (twelve typed kinds that
previously never left `patch_algebra`); the MSRV rise policy, documented and gated.

## 2. The three breaking changes — and what they are NOT

**All three are API breaks. None is a format break.**

**No repository written by any prior release becomes unreadable. Do not add a `DECLARED_BREAKS`
entry**, and **do not describe these the way `0.24.0`'s entry described its `Patch` schema change** —
that one really did make old code unable to read new repositories. **Conflating an API break with a
format break here would be the same forward/reverse confusion the G1 refresh already ruled on.**

1. **`ObjectType::ProjectGenesis` removed** (`prikk-object`, public). `from_code(0x0A)` now returns a
   retirement error rather than the variant. Breaks downstream exhaustive matches on `ObjectType`.
   **Say plainly that no repository can contain a `0x0A` object** — no code path ever constructed one —
   so nothing on disk is affected.
2. **`RepositoryVerification::has_blocking_defect()` removed** (`prikk-store`, public). **Give the
   remedy**: the verdict in `verify --format json`, or `prikk verify`'s exit code. Worth one sentence
   on why it went — it reported only two of the nine conditions `verify` actually fails on.
3. **`MergeEvidenceDisplayItem` gained three public fields** (`prikk-store`, public). The struct is not
   `#[non_exhaustive]`, so **downstream struct-literal construction breaks.** Name the fields.

**Verify every string you quote against the source.** I have twice given a wrong error string from
memory in this project; do not transcribe mine.

## 3. The version bump

- **`Cargo.toml:26`** — workspace `version`.
- **`Cargo.toml:37–43`** — the **seven** internal crate pins. **Count them; do not assume seven
  because I said seven.**
- **`README.md:45`** — *"Latest released implementation: **0.24.0**"*.

**`README.md:45` is the one claim that would otherwise be true of `main` and false of the released
artifact.** The `0.23.0` arc was burned by exactly that class of error. **Re-read the whole sentence
around it**, not just the number — if anything else in it is now false, say so rather than bumping the
digits and moving on.

**MSRV stays at `1.85`.** No rise, so the new rise policy owes nothing here. **If you find anything
implying otherwise, stop** — that would mean the MSRV gate disagrees with the manifest.

## 4. Out of scope

- **Tagging, pushing a tag, and crates.io.** Mine.
- **Any code change.** If a gate fails, report it; do not fix it inside a release commit.
- **`DECLARED_BREAKS`** (§2).
- **G1's fixture**, which goes one release stale on this cut. Known, structural, follow-up.

## 5. Controls

1. **`release-notes` extracts the new section** — run it and quote the output. This is the control
   that actually protects the release.
2. **The heading's em-dash is `e2 80 94`** — quote the hexdump.
3. **Every version site moved** — show that no `0.24.0` remains outside `CHANGELOG.md` and `rfcs/`.
4. **Full gate set green** against the exact commit, after the last edit.

## 6. What to report

1. The changelog entry as written.
2. **Your own count of the version sites** (§3), and anything I missed.
3. All four controls (§5), quoted.
4. **Full gate set against the exact commit, after the last edit.**
5. **Every numbered requirement's disposition, including ones that went without incident.**
6. Anything here that was wrong.

**Stop and escalate, do not guess**, if: `release-notes` refuses the section; a fourth breaking change
turns up that the assessment missed; or a version site exists that is neither `Cargo.toml`, the seven
pins, nor `README.md:45`.
