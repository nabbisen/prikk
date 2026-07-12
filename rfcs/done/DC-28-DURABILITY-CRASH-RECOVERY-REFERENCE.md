# RFC (done) - DC-28 Durability and Crash-Recovery Reference

**Status.** Released in 0.17.2.
**Target release.** 0.17.2.
**Tracks.** TASK-06 durability and crash-recovery reference.
**Touches.** mdBook reference documentation, durability/crash-recovery wording, claim-to-source
anchors, roadmap/status docs.
**Companion handoff.** None. This is a current-state documentation reference and does not create a
gating FDD.

## Context

DC-24 added the current data-model and trust/threat references. DC-26 moved current-state references
into the published mdBook. DC-27 added the patch-algebra and merge-evidence concept reference. The
largest remaining Tier-1 documentation gap is the transactional durability and crash-recovery
contract.

The current public docs mention active WAL, ref publication, verification, and doctor, but they do not
give reviewers one authoritative page for questions such as:

- what is durable after `commit` returns;
- what `seal` attempts to make atomic;
- how WAL and ref-log replay treat incomplete trailing bytes;
- what doctor may repair automatically and what it refuses to synthesize;
- why stale active locks and cross-platform filesystem semantics remain honest limits.

That gap can produce both under-trust and over-trust. Under-trust appears when readers cannot tell
which state is recoverable after an interrupted command. Over-trust appears when words like "WAL",
"atomic", or "doctor" are read as proof of crash-matrix validation, cross-platform fsync semantics, or
automatic reconstruction of missing objects. DC-28 closes the documentation gap without changing
storage behavior.

## Problem

1. **Durability claims are scattered.** WAL append, object persistence, ref publication, verify, and
   doctor behavior are currently explained across the data-model page, historical handoffs, code, and
   release status notes.
2. **Crash semantics need bounded wording.** Current code is designed around fsync, atomic rename, ref
   locks, and compare-and-swap checks, but release claims must not imply a completed crash-matrix or
   fuzzing campaign.
3. **Recovery posture is conservative but not obvious.** Doctor can truncate an incomplete WAL tail
   and reconstruct a missing `heads/main` pointer from already-valid ref-log and RefState data. It
   does not synthesize missing objects, repair malformed logs, auto-trust keys, or clear unsafe active
   sessions.
4. **TASK-07 and TASK-12 depend on this boundary.** Verify/doctor and concurrency/locking references
   should reuse a reviewed durability baseline instead of restating the crash contract from scratch.

## Design Goals

1. Add a self-contained current-state reference page at
   `docs/src/reference/durability-recovery.md`.
2. Explain the active durability boundary: a successful commit appends an exact signed Patch envelope
   to the active WAL and fsyncs the WAL file.
3. Explain WAL replay: valid records are read from the start, incomplete trailing bytes are reported,
   and checksum or malformed complete records fail closed.
4. Explain seal publication order and intended recovery shape: persist WAL Patches, create signed
   Block and RefState objects, append signed RefUpdate log evidence, promote the ref pointer, then
   drain the active WAL/ref metadata after successful publication.
5. Explain ref publication guardrails: branch-ref validation, ref-specific locking, compare-and-swap
   checks, signed RefState/RefUpdate envelopes, and candidate pointer promotion.
6. Explain the old-or-new-valid-state claim carefully: current implementation is designed so recovery
   should return either to the previous valid ref state or to a new valid published state, never to a
   trusted ref pointer that cannot be checked against objects/log evidence.
7. Explain doctor repairs and non-repairs: opt-in WAL-tail truncation and guarded `heads/main`
   pointer reconstruction only; no object synthesis, malformed-log repair, trust repair, or broad
   active-session cleanup.
8. Preserve mandatory honesty caveats: current durability evidence is unit/integration-test based,
   not a completed crash-matrix/fuzzing campaign; Linux is the only exercised platform; stale
   `active.lock` after a crash still needs manual cleanup and belongs with TASK-12.
9. Include visible claim-to-source anchors linking each major claim to code paths or released DCs.

## Non-goals

DC-28 does not add:

- code, schema, or CLI behavior;
- new fsync, rename, lock, WAL, object-store, ref-store, verify, or doctor behavior;
- crash-matrix or fuzz testing;
- platform certification for macOS, Windows, or non-Linux filesystems;
- automatic stale-lock cleanup;
- automatic reconstruction of missing Patch, Block, RefState, RefUpdate, Blob, trust, or key material;
- broad active-session recovery policy;
- repository-format stability or migration guarantees;
- backup/restore tooling;
- production-readiness claims;
- a new current-state FDD under `rfcs/fdds/`.

## Proposed Documentation Shape

Create:

```text
docs/src/reference/durability-recovery.md
```

Add it under the mdBook `# Reference` section near the existing data-model and trust/threat pages:

```md
- [Durability and Crash Recovery](reference/durability-recovery.md)
```

The page should be written as a current-state reference, not a recovery tutorial and not a future
design. It should link to existing or future related references instead of duplicating them:

- `docs/src/reference/data-model.md`;
- `docs/src/reference/trust-threat-model.md`;
- future TASK-07 `docs/src/reference/integrity-recovery.md`;
- future TASK-12 `docs/src/reference/concurrency-locking.md`.

### Boundary With TASK-07

DC-28 owns the durability and crash-recovery framing of `verify` and `doctor`: WAL-tail truncation and
guarded `heads/main` pointer reconstruction as recovery actions, plus the refusal posture around
unsafe or underspecified repairs. The future TASK-07 verify/doctor reference owns the full diagnostic
catalog: repository verification checks, `DoctorIssue` codes, severities, and diagnostic
interpretation.

The DC-28 implementation should reference that future catalog boundary where useful, but it must not
duplicate the full verify/doctor catalog. This keeps durability recovery and integrity diagnostics
from becoming two drifting explanations of the same command surface.

### Required Sections

The implemented page should contain at least:

1. **Core Caveats.** Early implementation status, test-evidence boundary, Linux-only exercised
   platform, no stable repository-format migration, and no production-readiness claim.
2. **Commit Durability Boundary.** Successful commit means a signed Patch envelope has been appended
   to the active WAL and the WAL file has been fsynced; it does not mean the Patch is sealed into a
   Block or published through a ref.
3. **WAL Replay and Tail Handling.** Valid records replay from the start; incomplete trailing bytes are
   reported; complete-record checksum/malformed errors are integrity failures, not safe truncation
   candidates.
4. **Active Ref Metadata.** Non-empty active WALs require valid active-ref ownership metadata; missing
   or malformed metadata is an active-session integrity issue. Empty-WAL metadata debris is local
   cleanup/debris, not sealed-history corruption.
5. **Seal Publication Flow.** Seal verifies active state and maintainer trust, persists WAL Patches,
   creates signed Block and RefState objects, appends signed RefUpdate log evidence, promotes the ref
   pointer, and clears active WAL/ref metadata after success.
6. **Ref Pointer and Ref Log Recovery.** Ref pointer files are mutable convenience pointers; ref logs
   and RefState objects are used to validate or reconstruct a missing pointer under narrow rules.
7. **Old-or-New Valid State.** Explain the intended post-crash recovery shape in bounded terms and
   state that it is supported by current tests rather than exhaustive crash injection.
8. **Doctor Repair Boundary.** Document only the current opt-in repairs and the refusal posture for
   unsafe or underspecified repairs.
9. **Stale Locks and Manual Repair.** State that stale `active.lock` cleanup is manual today and should
   be cross-linked to the future concurrency/locking reference.
10. **Deferred Work.** Crash-matrix/fuzz evidence, cross-platform fsync validation, stale-lock policy,
    broader repair policy, backup/restore, stable migration, and production readiness remain deferred.
11. **Claim-to-Source Anchors.** A visible table tying claims to released DCs and code paths.
12. **Provenance.** State that the page consolidates current released records through DC-27 and follows
    the DC-26 documentation-home model.

## Required Claim Boundaries

The implementation must say, in public docs:

- successful commit durability is active-WAL durability, not sealed-history publication;
- active WAL entries store exact signed Patch envelopes;
- WAL append fsyncs the WAL file, and new WAL creation best-effort syncs the parent directory;
- WAL replay reports incomplete trailing bytes separately from complete-record integrity failures;
- `seal` refuses non-empty active WALs with trailing partial bytes, missing active-ref metadata, or
  malformed active-ref metadata;
- publication uses signed Block, RefState, and RefUpdate envelopes plus local maintainer trust checks;
- ref pointer files are mutable pointers, not roots of trust;
- doctor repairs are opt-in and narrow;
- current durability evidence is based on unit/integration tests, not completed crash-matrix or fuzz
  evidence;
- Linux is the only exercised platform in current project gates;
- stale `active.lock` after a crash still needs manual cleanup.

Crash-safety language must stay bounded. The implementation may use words such as "atomic" or
"durable" only when the adjacent text also states the current evidence limit: unit/integration tests,
no completed crash-matrix or fuzzing campaign, and Linux-only exercised platform. It must not use
"guarantee", "guaranteed", "crash-safe", or unqualified "atomic/durable across crashes" wording.

The implementation must not say or imply:

- that Prikk has completed crash-matrix testing or filesystem fault injection;
- that macOS or Windows fsync/rename behavior is verified;
- that `doctor` can repair arbitrary corruption;
- that malformed complete WAL/ref-log records are safe to truncate automatically;
- that missing objects, signatures, trust policy, or key material can be synthesized;
- that a ref pointer alone is a root of trust;
- that the repository format is stable or migration-safe;
- that current durability claims make Prikk production-ready.

## Source Audit Requirements

Implementation must audit at least:

- `rfcs/handoffs/DC-24-data-model-trust-threat-docs/baseline-recap.md`;
- `rfcs/done/DC-15-ACTIVE-SESSION-INTEGRITY-HARDENING.md`;
- `rfcs/done/DC-13-NONDEFAULT-REF-GENESIS.md`;
- `rfcs/done/DC-11-MAINTAINER-TRUST-STORE.md`;
- `rfcs/done/PR-004-WAL-HANDOFF.md`;
- `rfcs/done/PR-006-VERIFY-HANDOFF.md`;
- `rfcs/done/PR-007-REF-PUBLICATION-HANDOFF.md`;
- `rfcs/done/PR-009-SEAL-SCAFFOLD-HANDOFF.md`;
- `rfcs/done/PR-011-DOCTOR-HANDOFF.md`;
- `rfcs/done/PR-012-DOCTOR-REPAIR-HANDOFF.md`;
- `rfcs/done/PR-013-REF-RECOVERY-HANDOFF.md`;
- `rfcs/IMPLEMENTATION-STATUS.md`;
- `docs/src/reference/data-model.md`;
- `docs/src/reference/trust-threat-model.md`;
- `crates/prikk-store/src/wal.rs`;
- `crates/prikk-store/src/refs.rs`;
- `crates/prikk-store/src/refs/log.rs`;
- `crates/prikk-store/src/refs/pointer.rs`;
- `crates/prikk-store/src/fsutil.rs`;
- `crates/prikk-store/src/verify.rs`;
- `crates/prikk-store/src/doctor.rs`;
- `crates/prikk-cli/src/seal.rs`.

The writer may use `.git-exclude/tasks/002-update-management/TASK-06-doc-durability-crash-recovery.md`
as scheduling context, but claims must be grounded in tracked code or released RFCs. Local
`.git-exclude/specs/` files are not reviewer-facing authority unless recapped into tracked material.

Anchor fidelity is part of the implementation contract. The implementation must cite tracked code,
PR-011/PR-012/PR-013, and released DCs for doctor and recovery claims. It must not cite a standalone
`FDD-02` as if that file exists, and it must not attribute doctor conservatism to PR-014. Any stale
code comments using those labels are not documentation authority for DC-28.

## Implementation Plan

1. Create `docs/src/reference/durability-recovery.md`.
2. Add it to `docs/src/SUMMARY.md` under `# Reference`.
3. Cross-link from `docs/src/reference/data-model.md` and `docs/src/reference/trust-threat-model.md`
   where durability caveats already appear.
4. Update `README.md`, `ROADMAP.md`, `rfcs/README.md`, and `rfcs/IMPLEMENTATION-STATUS.md` only enough
   to reflect the active documentation increment and the new reference after implementation.
5. Do not change Rust code, command output, object schema, release version, or repository behavior
   during implementation.
6. Prepare an implementation review package after the page is drafted.

## Review Gates

Design review should verify:

- the page scope is current-state reference documentation, not a new durability feature design;
- the old-or-new-valid-state wording is bounded enough for current evidence;
- the mandatory caveats prevent overclaiming crash safety, cross-platform guarantees, and production
  readiness;
- the source audit list is sufficient for WAL, ref publication, seal, verify, and doctor claims;
- TASK-06 is correctly implemented through the DC-26 documentation-home model;
- no current-state FDD under `rfcs/fdds/` is introduced.

Implementation review should verify:

```text
mdbook build docs
git diff --check
```

and should additionally include:

- proof that `docs/src/reference/durability-recovery.md` is reachable from `docs/src/SUMMARY.md`;
- built-book link/reachability checks for the generated durability, data-model, and trust/threat
  pages, including checks that no dangling relative links escape `docs/src/`;
- a source-audit checklist showing which released DCs, PR handoffs, and code paths were checked;
- verification that the DC-28 page owns durability/recovery framing only and does not duplicate the
  future TASK-07 verify/doctor diagnostic catalog;
- verification that no "guarantee", "guaranteed", "crash-safe", or unqualified "atomic/durable across
  crashes" wording appears;
- verification that crash-safety wording is adjacent to the test-evidence and Linux-only caveats;
- verification that claim anchors use tracked code, PR-011/PR-012/PR-013, and released DCs, and do not
  cite a standalone `FDD-02` or attribute doctor conservatism to PR-014;
- claim-to-source anchor table review;
- line-count evidence for new/changed docs.

## Acceptance Criteria

DC-28 is complete when:

- `docs/src/reference/durability-recovery.md` exists and is reachable from the mdBook summary;
- the page explains active-WAL durability, WAL replay/tail handling, seal publication flow, ref
  recovery, doctor repair limits, stale-lock limits, and deferred crash/platform evidence;
- the page has visible claim-to-source anchors;
- related current-state reference pages cross-link where useful;
- ROADMAP/status docs track the documentation increment honestly;
- implementation review accepts the documentation; and
- the completed release records DC-28 as documentation-only with no code, schema, CLI, or repository
  behavior change.
