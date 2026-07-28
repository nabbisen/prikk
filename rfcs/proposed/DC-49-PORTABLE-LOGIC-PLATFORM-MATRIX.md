# RFC (proposed) - DC-49 Portable-Logic Platform Matrix

**Status.** Proposed; design review required. Descoped from DC-41 during design re-review v1 and recorded
here so it is owned rather than lost.
**Target milestone.** M2 - post-correction assurance milestone.
**Schedule position.** Blocked. Not startable until the M1 portability-claim correction ships (see
Trigger). Independent of DC-41's own acceptance or completion.
**Tracks.** The cross-platform-evidence portion of architect review N4 that DC-41 could not deliver inside
the parked development lane.
**Touches.** `.github/workflows/ci.yml` and the DC-45 governed-procedure command classifier. No production
code, no test-corpus modification.

## Problem

`prikk-object`, `prikk-replay`, `prikk-crypto`, `prikk-error`, and `prikk-hash` contain pure logic with no
filesystem dependency, and none of it is exercised outside Linux. `prikk-store` and `prikk-cli` cannot be
exercised elsewhere: DC-37 makes repository mutation definitionally unsupported off Linux
(`RepositoryLayout::init` and everything downstream returns `unsupported_mutation()`), so those crates are
excluded by design rather than by omission.

## Trigger (blocking precondition)

`MILESTONES.md` assigns correction of the public "Linux-only exercised gates" wording
(`docs/src/reference/durability-recovery.md`, `docs/src/reference/concurrency-locking.md`) to an M1
release gate performed inside the mandatory hold of an activated release. That wording describes the
non-Linux posture as a coverage gap when it is a functional impossibility. **DC-49 may not land before
that correction ships**, because a green non-Linux CI badge alongside uncorrected wording would appear to
support a claim the code contradicts — inverting the purpose of the evidence.

## Design

Add one CI job, matrix'd over macOS and Windows, scoped exactly to the crates that need no repository
mutation:

```yaml
  portable-logic:
    name: portable-logic (macos / windows) - NOT durability evidence
    strategy:
      matrix:
        os: [macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Test portable crates
        run: cargo test --locked -p prikk-hash -p prikk-error -p prikk-object -p prikk-crypto -p prikk-replay
```

The job name and this RFC label the result **portable-logic evidence only** — never durability, mutation,
filesystem, or platform-support evidence.

`.github/workflows/ci.yml` is a governed procedure file under DC-45/DC-47/DC-48. The
`cargo test --locked -p … -p …` form is not an accepted production in
`tools/release-policy/src/command_scan/procedure.rs`, so DC-49 requires a reviewed classifier amendment in
the same increment — planned here rather than discovered during implementation, following the DC-46
pattern.

## Non-goals

- No `#[cfg]` or `#[ignore]` added to the existing test suite to make more of it "pass" elsewhere.
- No inclusion of `prikk-store` or `prikk-cli`.
- No public macOS/Windows platform-support claim derived from this job.
- No change to DC-37's Linux-only mutation boundary.

## Acceptance criteria

The M1 portability-claim correction has shipped; the job is green on both operating systems; the
non-durability labelling is present in the job name, this RFC, and the classifier amendment; the classifier
amendment adds exactly the required vector while retaining all existing productions and emitting no
policy or publication invocation; and no existing test was modified.
