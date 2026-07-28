# DC-49 Portable-Logic Platform Matrix - Implementation Handoff

**Prepared in advance. Currently BLOCKED — do not start.** See §1.
**Authored by** the architect (function-designer role). Implementation review remains independent.
**Size:** small — one CI job plus one classifier amendment.
**Touches:** `.github/workflows/ci.yml` and `tools/release-policy/src/command_scan/procedure.rs`. No
product code, no test-corpus modification.

## 1. Blocking precondition — check this first

`MILESTONES.md` assigns correction of the public "Linux-only exercised gates" wording
(`docs/src/reference/durability-recovery.md`, `docs/src/reference/concurrency-locking.md`) to an **M1
release gate**, performed inside the mandatory hold of an **activated** release. The release lane is
currently `parked`.

That wording describes the non-Linux posture as a *coverage gap*. It is a *functional impossibility*:
DC-37 makes repository mutation unsupported off Linux by construction. Adding a green non-Linux CI badge
while the wording still says "exercised gates" would appear to support a claim the code contradicts —
inverting the purpose of the evidence.

**Verify the correction has shipped before starting.** If the owner would rather unblock this sooner, the
alternative is a reviewed decision to move the documentation correction into the development lane. That is
an owner decision, not an implementation one — do not make it by proceeding.

## 2. The job

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

At the baseline this was 115 tests (`prikk-object` 64, `prikk-replay` 44, `prikk-crypto` 5, `prikk-hash` 2,
`prikk-error` 0) with zero expected failures. DC-41 stages 2-4 grow `prikk-hash` and `prikk-object`, so
re-measure rather than quoting 115.

**`prikk-store` and `prikk-cli` are excluded by design**, not by omission: DC-37 makes
`RepositoryLayout::init` and everything downstream return `unsupported_mutation()` off Linux, and 45
`prikk-store` source files depend on it. Including them would fail on the first run.

## 3. The classifier amendment (plan for it, do not discover it)

`.github/workflows/ci.yml` is a governed procedure file under DC-45/DC-47/DC-48. The
`cargo test --locked -p … -p …` form is **not** an accepted production in
`tools/release-policy/src/command_scan/procedure.rs`, so `boundary-check` and `reference-check` will fail
closed until the exact vector is added.

Follow the DC-46/DC-47 pattern precisely:

- add **exactly** the required vector, retaining every existing production;
- prove the vector emits **no** policy or publication `Invocation` (positive test asserting empty errors
  *and* empty invocations, in both shell and YAML strict modes);
- add near-miss negatives (missing `--locked`, reordered `-p` arguments, a dynamic package name);
- `procedure.rs` is a review-gated policy artifact — this is a policy change, not a refactor.

## 4. Traps

- **Do not** add `#[cfg]` or `#[ignore]` to any existing test to widen what passes elsewhere. Explicitly
  forbidden; it would be a large semantic edit to a correctness-critical corpus under a CI increment.
- **Do not** cite a green result as durability, mutation, filesystem, or platform-support evidence. The
  labelling in the job name and the RFC exists to prevent exactly that, in both directions — including
  informal citation in a later release note.
- **Do not** widen the crate list opportunistically. Adding `prikk-store` "to see what happens" produces a
  red job and tempts the `#[cfg]` remedy above.

## 5. Definition of done

- The M1 portability-claim correction has shipped (§1).
- Job green on both operating systems; measured test count reported.
- Non-durability labelling present in the job name, the RFC, and the classifier amendment.
- Classifier amendment adds exactly one vector, retains all existing productions, emits no invocation,
  and has positive plus near-miss coverage.
- `boundary-check` and `reference-check` `valid: true` after the amendment.
- No existing test modified; no `#[cfg]`/`#[ignore]` added.
- Full gate set green (`rfcs/EXECUTION-ORDER.md` §6.8).

## 6. Submit with

Diff; evidence note stating the precondition is satisfied and how it was verified, the measured test count
per OS, and the classifier amendment's positive/negative coverage; gate output; explicit statement that no
existing test changed and no platform-support claim is being made.
