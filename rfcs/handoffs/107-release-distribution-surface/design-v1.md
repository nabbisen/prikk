# RFC 107 — Release distribution surface — design v1

**RFC:** `rfcs/accepted/107-release-distribution-surface.md`. Read §0 and §2 first — §2 is why the fix is
not "write better notes."

**Owner direction 2026-08-17:** published release pages are **not** being retroactively corrected; the
fix reaches users through the next release, and that release should be soon (0.22.1). **Stage 1 is
therefore the near-term path.** Whether Stage 2 rides in the same release is the owner's call and is being
put to them; build Stage 1 so it does not depend on the answer.

## Stage 1 — the notes describe the release, and cannot go stale again

### The rule that matters

**Anything on the release page that depends on a fact the project can change must be derived from that
fact, not restated beside it.** The current template's platform sentence is false today precisely because
it was prose describing a moving target. Fixing the sentence and leaving the shape sets the same trap for
the next platform change.

Concretely, that means the page is assembled at release time from:

- **the version's own `CHANGELOG.md` entry** — the per-release content, which already exists in the right
  shape and is the only place it should live;
- **the build matrix** — whatever the page says about which platforms have binaries follows from what was
  actually built, not from a sentence someone maintains;
- **the standing sections** — release authority and reproduce-from-source, which are genuinely static and
  correct, and stay.

### Report before implementing

1. **How the version's section is extracted from `CHANGELOG.md`.** Entries are `## X.Y.Z — DATE` and run
   until the next `## `. Say what you use and why it will not silently match the wrong heading.
2. **What happens when the tag has no matching entry.** A release with no changelog section is a mistake,
   not an occasion for empty notes. **Fail the release** rather than publish a page that says nothing —
   and say so, because the alternative is a silent regression to exactly today's situation.
3. **Where the assembly happens** — workflow step, or a small script the workflow calls. Prefer whichever
   can be exercised without cutting a release (criterion 7).
4. **Whether the release-policy publication boundary scanner covers `release.yml`.** Commit `38a8cff`
   extended it for DC-70's workflow; a new step or a new command may need an allowlist entry, and it is
   better to know before `boundary-check` says so.

### The false claim

Delete the platform sentence in its current form. It is not being corrected — it is being replaced by
derivation, per the rule above. **If you find you cannot derive it and must write prose, stop and report**;
that would mean criterion 2 is not achievable as specified, which is a design problem and mine.

## Stage 2 — binaries for the platforms prikk actually supports

### The matrix

`release.yml` builds two Linux targets. macOS and Windows both mutate as of 0.21.0 and 0.22.0. Extend to
what support claims — and **report the target list before building it**, with a reason per target rather
than a list.

### What differs per platform, and must not be papered over

- **Windows**: the binary is `prikk.exe`, and `.zip` is the ordinary distribution form there. A `.tar.gz`
  containing an `.exe` is technically fine and practically wrong.
- **macOS**: the artifact is **unsigned**, so Gatekeeper warns on first run. Criterion 5 requires this
  stated, in the same register as the existing release-authority section — plainly, as a known gap, with
  what the user can do about it. **Do not soften it.** Notarization needs an Apple identity and belongs
  with DC-43; saying so is the honest position and is a non-goal here.
- **Checksums and `.build-info.txt`** exist per artifact today and must continue to, per artifact.

### Criterion 3 — DC-70's carried criterion

*"Release evidence describes what was actually published — every artifact."* Deferred behind DC-45's
frozen baseline "until the 0.19.0 cutover", which happened four releases ago. **Establish what the
evidence models today before changing it** — my reading is one archive, but that reading predates two
releases and the schema may already have moved.

## Both stages

### Criterion 7 — the part this increment is weakest on

`release.yml` fires only on a tag matching `[0-9]+.[0-9]+.[0-9]+`, so **nothing here can be exercised
without cutting a release**. That is the opposite of every recent increment, where a control was watched
to fail before being trusted.

**Report how you demonstrate the change rather than asserting it.** Options worth weighing, not a menu to
pick from silently: a `workflow_dispatch` trigger so the assembly can be run against a chosen ref without
publishing; extracting the notes assembly into something unit-testable; a dry-run mode that prints the
notes it would publish. **Say what you chose and what it does not cover** — if some part is only provable
by an actual release, that is acceptable and must be stated, not omitted.

### Gates

The standing set per `EXECUTION-ORDER.md` §6 rule 9, and green three-platform CI. `boundary-check`
specifically, since this touches the release workflow the publication scanner reads.

### Stop-and-report

- The platform statement cannot be derived and would have to stay prose (Stage 1).
- A target builds but produces an artifact that cannot be verified or reproduced by the documented
  command (Stage 2).
- The evidence schema turns out to already describe N artifacts, making criterion 3 already closed — that
  is a finding worth having, not a step to skip quietly.
