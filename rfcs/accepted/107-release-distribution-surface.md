# RFC 107 — Release distribution surface

**Status.** Accepted by the project owner 2026-08-17, from two observations the owner made directly:
prebuilt binaries are Linux-only despite macOS and Windows being supported, and a visitor to a release
page cannot see what changed.
**Touches.** `.github/workflows/release.yml`, `.github/release-notes-template.md`, `README.md`, and
whatever release-policy surface covers them. No product code.

**Author-review independence.** Designed and reviewed by the same agent; recorded, not elided.

## 0. Three defects, one surface, and one of them is publicly wrong right now

**A visitor arriving at a prikk release page today is told nothing about that release, and one thing that
is false.**

1. **No release describes itself.** `release.yml` publishes with
   `--notes-file .github/release-notes-template.md` — a **static file, identical on every release**. No
   prikk release page has ever said what changed in it.
2. **The template makes a claim that is now false**, and has been since 0.21.0:

   > *"repository **mutation** is Linux-only project-wide (DC-37), so this is not an artifact-specific
   > limitation"*

   True when written, for 0.20.0. **False for 0.21.0 and 0.22.0 — the two releases whose entire content
   was making mutation work on Windows.** A visitor to 0.22.0's page is told the opposite of what that
   release did. Verified live on all three published pages.
3. **Prebuilt binaries are Linux-only** (`x86_64`, `aarch64`), by matrix configuration that predates
   macOS and Windows mutation support and was never revisited.

## 1. What is already settled

- **DC-70 governs this surface**, and **its criterion 3 is carried with an expired blocker**: *"release
  evidence describes what was actually published — every artifact"*, deferred because extending it was
  *"blocked behind DC-45's frozen baseline until the 0.19.0 cutover."* **0.19.0 released 2026-08-08** —
  four releases ago. The condition discharged itself and nothing rechecked it.
- **`CHANGELOG.md` already holds per-version content** in the right shape. The material for a real release
  page exists; nothing generates from it.
- **The release-authority position is correct and must survive** — `release-signers.toml` is empty and
  fail-closed, and the template says so plainly. That section is the one part of the current template
  doing its job.

## 2. The obstacle, stated as a problem

**A static file was used to describe facts that change.** Defect 2 is not a typo; it is the guaranteed
outcome of that shape. Correcting the sentence without correcting the shape sets the same trap for the
next platform change — and this project has now found the same pattern in a test gate
(`object_store.rs`), an RFC naming rule, and a `caller_tests` comment.

So the fix is not "write better notes." **Anything in the release page that depends on a fact the project
can change must be derived from that fact, not restated beside it.**

A second obstacle is honesty rather than mechanism: **macOS binaries will be unsigned**, so Gatekeeper
will warn on download. That interacts directly with the release-authority section already on the page, and
shipping a macOS artifact without saying so would contradict it.

## 3. Acceptance criteria

1. **Every release page describes that release**, derived from `CHANGELOG.md`'s entry for the version
   rather than hand-written per release or restated in a template.
2. **No platform-dependent claim is hand-maintained prose.** Whatever the page says about which platforms
   have binaries must follow from the build matrix. Defect 2 must be impossible to reproduce, not merely
   fixed once.
3. **Prebuilt binaries for macOS and Windows**, or a stated per-platform reason there are none. A reason
   names what is missing; "not yet" is not a reason.
4. **Windows artifacts are in the form Windows users expect** — `.exe`, and an archive format that is
   ordinary on that platform.
5. **The macOS signing position is stated**, in the same register as the existing release-authority
   section: unsigned, what that means for the user, and that it is a known gap rather than an oversight.
6. **DC-70's criterion 3 is closed** — release evidence describes every published artifact, not one.
7. **The release workflow is exercised before it is trusted.** It runs only on a version tag today, so it
   cannot be tested without cutting a release. Say how this increment demonstrates the change rather than
   asserting it — this project's standing bar, and the one place this increment is structurally weakest.
8. Green three-platform CI.

## 4. Non-goals

- **Signing or notarizing macOS binaries.** That needs an Apple Developer identity and is a key-custody
  question belonging with DC-43 and the signer bootstrap, not here. Criterion 5 states the gap; it does
  not close it.
- **Changing the release-authority position.** It is accurate and stays.
- **Retroactively editing already-published release pages.** Separate from the increment; the owner's
  call, and being handled separately.
- **Adding platforms prikk does not support.** The matrix follows support, which is the whole point of
  defect 3.

## 5. Staging

**Stage 1 — the notes.** Defects 1 and 2, and criterion 2's derivation rule. Smallest, fixes the live
false claim, and independent of packaging questions.

**Stage 2 — the matrix and criterion 3.** Defects 3, plus criteria 3-6.

**Both stages ship in 0.22.1 — owner ruling 2026-08-17.** A patch release existing only to republish
eight byte-identical crates for a `.github/` fix is thin; the binaries are what a user gains. So the
release that stops telling macOS and Windows users that mutation is Linux-only is also the one that gives
them something to download. Both stages land on one branch and merge once, as DC-98 and DC-99 did.

**Report before implementing, per stage.** For Stage 1: how the per-version section is extracted from
`CHANGELOG.md` and what happens when a tag has no matching entry. For Stage 2: what each platform's
artifact looks like, and how criterion 7 is satisfied given the workflow only fires on a tag.
