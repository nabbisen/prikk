# RFC (accepted) - DC-41 Integrity Evidence Campaign

**Status.** Accepted after architect design review v1 (Needs changes: B1, B2, B3), design re-review v1
(Needs changes: B4), and design re-review v2 (Accept) on 2026-07-23. Implementation has not started;
stages 1-4 land as separately reviewed candidates. DC-36 through DC-40 implementation dependencies are
complete. Design acceptance does not require release 0.18.0 to be activated or published. DC-41 does not
discharge the M1 literal DC-38 stale-pointer/ahead-log reproduction, which is rerun when an RC is
explicitly activated.
**Target milestone.** M2 - post-correction assurance milestone.
**Schedule position.** First active post-correction development increment; this scheduling note is not
implementation authority. Release-specific evidence must be rerun when an RC is explicitly selected.
**Tracks.** Architect review N6 (first-party SHA-256 ROI, deferred decision) and the crash-matrix, fuzz,
and hash-differential evidence gaps from N4. The cross-platform-evidence gap from N4 is addressed only up
to the point that stays inside the development lane; see Follow-up below. Repository-wide AUTHOR trust
verification, key lifecycle, and format migration are separate feature/RFC scope and are not delivered
here.
**Touches.** Failpoint/property/fuzz test infrastructure and hash differential evidence. Production
behavior, schema, format, CLI, and cryptographic primitives are unchanged; production behavior changes
require separate RFCs.
**Scope note.** DC-41 is scoped to stages 1-4, all completable inside the parked development lane. The
former stage 5 (platform matrix) is spun out as its own future increment gated on an M1 release-lane
correction DC-41 does not own; see Follow-up.

## Problem

The corrected M1 contracts (DC-36 through DC-40) have unit and integration coverage but no adversarial
evidence campaign: `prikk-hash` has exactly 2 tests (empty input, `"abc"`) for the primitive underpinning
every ObjectId, state root, ref-name path, and signature preimage; there is no property or fuzz
infrastructure anywhere in the workspace; and all 24 DC-38 crash-matrix failpoint variants are already
triggered by existing tests, but the per-variant post-failure assertions have not been audited against a
uniform bar and no durable coverage table exists for adversarial review. Separately, public docs describe
the non-Linux posture as a coverage gap ("Linux-only exercised gates") when DC-37 makes it a functional
impossibility (repository mutation returns `unsupported_mutation()` off Linux by construction); that
correction is M1 release-gate scope, not DC-41's (see Follow-up). This RFC builds repeatable, reproducible
evidence against the corrected contracts without touching production behavior.

## Design

DC-41 is four independent workstreams, staged in dependency order so each lands as its own implementation
review and acceptance rather than one monolithic candidate. No stage may be bundled with another. A fifth
workstream (platform matrix) was descoped from DC-41 into its own future increment; see Follow-up.

| Stage | Workstream | New dependency? | CI change? |
|---|---|---|---|
| 1 | Crash matrix | No | No |
| 2 | Hash vectors | No | No |
| 3 | Hash differential | Yes (`sha2`) | No |
| 4 | Property/fuzz | Yes (`proptest`) | Yes (existing jobs only) |

Stages 1-2 land first because they need no new dependency and no CI change. Stage 3 is the first
dependency change, isolated so the `Cargo.lock` transition is reviewable on its own. Stage 4 is the
largest workstream and benefits from stage 3's dependency precedent. All four stages are completable
entirely inside the parked development lane; none depends on release-lane activation.

### Stage 1 - Crash matrix

The closed boundary list is the existing `Point`/`TestFailPoint` enum in
`crates/prikk-store/src/fsutil/anchored/failpoints.rs` (currently 24 variants spanning directory sync,
mutable file write/rename, promotion source/destination sync and rename, required file/directory sync,
append/truncate/unlink, and immutable install paths, including platform-unsupported branches). This is
already the shared seam used by both `crates/prikk-store/src/refs/tests/publication_recovery/failpoints.rs`
(WAL/candidate/promotion/log boundaries) and the object-store crash tests — DC-41 does not invent a new
mechanism, it drives the existing one to audited, documented closed-list coverage.

**Acceptance:** audit every `Point` variant defined at implementation time (24 at design time — the count
is a snapshot, not a target to preserve) against the bar already met by the strongest existing tests (for
example `refs/tests/publication_recovery/failpoints.rs`, which asserts object presence, pointer value, log
record count, and a specific verify issue code, not merely `is_err()`); add assertions only where a
variant falls short of that bar. Publish the resulting per-variant table as a tracked evidence record —
this table, not new coverage, is Stage 1's primary deliverable, since the coverage itself already exists.
Any variant that cannot be exercised (for example a platform-unsupported branch unreachable on the CI
runner) is listed with the reason, not silently omitted.

### Stage 2 - Hash vectors

Add SHA-256 test vectors at the 55/56/63/64-byte input-length boundaries (the padding-block transition)
and at least one multi-block (>64 byte) vector, using published NIST/RFC 6234 test vectors as the source
of truth. Extract `prikk-hash`'s tests from `crates/prikk-hash/src/lib.rs` (~lines 150-166) into
`crates/prikk-hash/src/tests.rs`, matching the project's test-module-separation rule, before adding to
them.

**Acceptance:** the boundary set (55, 56, 63, 64 bytes) plus the multi-block vector plus the two existing
vectors (empty, `"abc"`) all pass; tests live in `src/tests.rs`, not inline in `lib.rs`.

### Stage 3 - Hash differential

Add `sha2` (RustCrypto, widely audited, pure-Rust, stable-toolchain-compatible) as a dev-only dependency
of `prikk-hash`, used exclusively to cross-check `prikk-hash`'s output against an independent
implementation over randomized inputs with fixed seeds (reproducible failures). This is the "audited
development dependency" the prior draft left unnamed.

**Acceptance:** at least 10,000 randomized cases per CI run, drawn from a stated input-length distribution
that includes empty, sub-block, exact-block-boundary, and multi-block-spanning lengths; zero unexplained
mismatches. See the escalation clause below for what "unexplained" routes to.

### Stage 4 - Property/fuzz

Use `proptest` (dev-only) for both property-style and fuzz-style testing. `proptest` is chosen over
`cargo-fuzz`/libFuzzer specifically because `cargo-fuzz` requires a nightly toolchain, which would
conflict with DC-46's release-blocking Rust 1.85.0 stable `--locked` gate; `proptest` runs entirely on
stable and gives fixed-seed reproducers and a `proptest-regressions` corpus for free.

Closed target list, scoped to what is currently implemented (not the aspirational full object model):

- canonical object-envelope decoding for every current `ObjectType` variant (`Patch`, `Block`,
  `RefState`, `RefUpdate`, `Tag`, `Attestation`, `Blob`, `BlockSummaryCache`, `RecoveryNote`,
  `ProjectGenesis`);
- WAL record framing and ref-log entry framing (decode/re-encode round trip and malformed-input
  rejection);
- replay/lifecycle-cache reconstruction from WAL;
- patch operation decoding, with generated inputs bounded to small fixed ranges chosen for property-test
  tractability (op count, path depth, path segment length, content size) — these are test-generation
  bounds, not the production `NFR-PERF-02` active-block thresholds (800/1000), which govern a different
  concern and are out of scope here.

**Budgets:** a fast per-target case count enforced on every CI run (proposed: 256 cases/target), and a
longer campaign budget run on demand or on a schedule (proposed: 100,000 cases/target), not gating
ordinary CI. **Corpus policy:** only minimized `proptest-regressions` failure files are committed, one
packed file per crate (not one file per case) — the DC-45 237-file vector-set experience is the reason
this is decided now rather than at owner-acceptance time; generated (non-failure) corpora are not
committed and regenerate from the fixed seed.

**Acceptance:** every listed target has a property test wired into CI at the fast budget; the campaign
budget has been run at least once with results recorded; zero unexplained findings — every finding is
either fixed under its own corrective RFC (per the discipline clause below) or recorded with a committed
minimized reproducer and an open follow-up reference.

## Follow-up (out of scope for DC-41): platform matrix

DC-41's original scope included a fifth workstream — a `portable-logic` CI job exercising the crates that
do not require real repository mutation. It is deliberately **not part of DC-41** and is recorded here as
a fully-specified future increment rather than dropped:

- **Why it is out of scope now.** `MILESTONES.md` places the correction of the public "Linux-only
  exercised gates" wording (`docs/src/reference/durability-recovery.md`,
  `docs/src/reference/concurrency-locking.md`) — which must land before any non-Linux CI job can be added
  without manufacturing a false platform-support signal — inside the mandatory 72-hour hold of an
  *activated* release. The release lane is currently **parked**. A workstream gated on an event that can
  only occur inside an activated lane cannot be completed by an increment that is scoped to the parked
  development lane. Scoping DC-41 to stages 1-4 keeps it entirely completable now; the platform matrix
  becomes its own increment triggered once that correction ships.
- **What that future increment should do**, unchanged from this RFC's prior draft: add a `portable-logic`
  CI job scoped exactly to `prikk-hash`, `prikk-error`, `prikk-object`, `prikk-crypto`, `prikk-replay`
  (115 tests today, growing under DC-41 stages 2-4), explicitly excluding `prikk-store`/`prikk-cli`
  because DC-37 makes repository mutation (`RepositoryLayout::init` and everything downstream — 45
  `prikk-store` source files depend on it) definitionally unsupported off Linux, not merely untested. The
  job and RFC must label results **portable-logic evidence only** — never durability, mutation,
  filesystem, or platform-support evidence — and must not add `#[cfg]`/`#[ignore]` to the existing suite.
  `.github/workflows/ci.yml` is a governed procedure file under DC-45/DC-47/DC-48; the
  `cargo test --locked -p … -p …` form is not currently an accepted production in
  `tools/release-policy/src/command_scan/procedure.rs`, so that increment will need a reviewed classifier
  amendment in the same change (the DC-46 pattern), planned rather than discovered.
- **Trigger:** proposable once the M1 portability-claim correction ships, independent of DC-41's own
  acceptance or implementation timeline.

## Dependency and lockfile plan

- **Crates:** `sha2` (stage 3) and `proptest` (stage 4), both `[dev-dependencies]` only, in `prikk-hash`
  (and `prikk-object`/`prikk-replay` as needed for stage 4's decoder targets), never `[dependencies]`.
- **Rust 1.85 gate:** every new dev-dependency version selected must pass
  `cargo +1.85.0 test --workspace --locked`, preserving DC-46's contract; this is verified before the
  dependency is proposed, not discovered after.
- **`Cargo.lock` re-freeze:** the frozen lockfile identity (currently
  `0cd51cbdc98210bc745dd6a7190fbcde30b35dfea4d1cd66b7f0b8459527c616`, unchanged since DC-48) will change at
  stage 3. This is a deliberate, reviewed re-freeze; the new hash becomes the baseline identity that
  subsequent reviews verify, recorded in the stage-3 implementation review.
- **Advisory surface:** report the before/after locked-dependency count (169 today) in the stage-3 and
  stage-4 implementation reviews, and confirm `cargo audit --no-fetch` exits zero after each addition.
- **Boundary preservation:** confirm the seven product package listings still exclude test-only tooling
  under the DC-45 boundary check after each dependency addition.
- **Dev-only enforcement:** dev-only placement is enforced by review, not by an existing mechanical gate.
  Neither the DC-45 package-listing check (which inspects packaged **file paths**, not dependency
  manifests) nor `boundary::check_dependencies` (which guards only the tool↔product edge over **local**
  crates, so a crates.io crate like `sha2` is outside its model) detects a third-party crate misplaced into
  `[dependencies]`. Each dependency-adding stage's implementation review must therefore verify
  `[dev-dependencies]` placement directly in the diff for every product crate touched. A mechanical
  allowlist check over `[dependencies]` is a candidate follow-up increment, not part of DC-41.

## Escalation: hash differential mismatch

A stage-3 mismatch between `prikk-hash` and the audited reference on any input is categorically different
from an ordinary test failure: object IDs, state roots, ref paths, and signature preimages computed from
such an input would be non-standard, making the discovery repository-format-invalidating. This case is
**immediate stop-work with architect/security escalation**, distinct from the normal corrective-RFC path
below. As background evidence (not a substitute for stage 3's own vectors): during the DC-40 review, all
eleven state-root vectors were independently reconstructed with Python's `hashlib` and matched
`prikk-hash` exactly, which lowers the prior on a catastrophic mismatch but was over a handful of inputs
only.

## Discipline

A discovered behavior defect (outside the escalation case above) opens a dedicated corrective RFC instead
of being silently normalized into a test expectation. Fuzzers and differential dependencies remain
development-only and must not enter object identity or runtime trust paths — checked by implementation
review at each dependency-adding stage per the dependency plan above, since no mechanical gate covers this
today. Public evidence claims are updated only from observed, reproducible results; CI presence is not
durability proof, and non-Linux CI presence is never platform-support proof.

## Non-goals

- No formal proof of crash safety, certification, or production-readiness claim.
- No replacement of the first-party SHA-256 implementation in this RFC. Whether a first-party
  implementation still has sufficient ROI once stage 3's differential evidence exists is a **deferred
  decision**, recorded here for a later increment to answer using DC-41's own results as input — not a
  dropped thread.
- No merge-scope expansion or random mutation of real user repositories.
- No new `#[cfg]`/`#[ignore]` attributes added to the existing test suite under this RFC; any such change
  requires its own separate reviewed design authority.
- No platform matrix, non-Linux CI job, or platform-support claim of any kind in this RFC; that workstream
  is out of scope for DC-41 (see Follow-up).
- No release-lane action of any kind. The release lane is parked; nothing in this RFC activates it.

## Acceptance criteria

Each stage lands as its own implementation review against its own acceptance bar above; stages are not
bundled. DC-41 is complete when all four stages are individually accepted and the evidence record contains
the failpoint per-variant table, the hash vector/differential results, and the property/fuzz
campaign-budget run — and an adversarial review receives that table and failure corpus, not only aggregate
pass counts. The platform matrix is explicitly not a DC-41 completion condition; it is a separate future
increment (see Follow-up).
