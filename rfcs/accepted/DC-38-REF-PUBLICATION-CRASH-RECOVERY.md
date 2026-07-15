# RFC (accepted) - DC-38 Ref Publication Crash Recovery

**Status.** Accepted after architect re-review on 2026-07-14; current M1 implementation increment
after accepted DC-37 and DC-36 storage semantics.
**Target milestone.** M1 - 0.18.0 corrective release.
**Tracks.** Architect review B1.
**Touches.** Ref publication order, seal retry, ref verification, doctor diagnostics, active-session
cleanup, failpoints, and durability/recovery documentation.

## Problem

The released log-first publication order permits an ahead committed log with a stale valid pointer.
Verification accepts that split state, and seal retry can append the same transition again, corrupting
the ref-log chain.

## Design contract

Implement DC-34's pointer-first state machine under the ref-specific lock:

1. validate publication and persist required objects;
2. recheck CAS;
3. write and required-sync the pointer candidate;
4. recheck CAS;
5. promote and required-sync the authoritative pointer;
6. append/fsync exactly one committed RefUpdate;
7. confirm pointer/log agreement before active WAL cleanup.

Verification must jointly compare pointer, RefState chain, and ref-log chain. It must reject an ahead
log, stale pointer, duplicate transition, sequence mismatch, or unexplained pointer lead. A pointer lead
of exactly one transition is classified as an interrupted publication only when immutable objects and
retained active state prove the expected transition; it still causes a non-zero verification result.

Seal retry may finish that state using the caller's trusted maintainer signer and the deterministic
version-1 no-clock RefUpdate contract. It must verify that the reconstructed signed envelope exactly
matches the expected transition before append. Doctor diagnoses the condition and recommends seal retry;
it does not sign or append the record.

Format-1 missing-pointer-from-log doctor repair is refused in 0.18.0. The released repair cannot be
made compatible with the format-1 read-only boundary without defining a second mutation authority, so
DC-38 accepts this explicit compatibility loss. Doctor diagnoses the missing pointer and valid log but
does not reconstruct it; users must restore from backup or preserve the repository for DC-44 migration/
recovery tooling. The sole format-1 mutation exception remains the exact signer-backed one-record-ahead
seal completion defined by DC-34.

For a structurally incomplete final ref-log frame after pointer promotion, signer-backed seal retry is
the only automatic finishing path. Under the ref lock it validates the complete log prefix, pointer,
retained WAL, expected transition, and trust; truncates and syncs only the exact framing-incomplete
suffix defined by DC-34; then appends/syncs the expected record. A fully framed checksum-invalid or
malformed record is not truncation-safe. Commit and every unrelated mutation remain blocked while
pointer/log agreement or publication cleanup is incomplete.

For the released format-1 log-first split state, retry follows DC-34's bounded compatibility path: it
validates the one already-signed ahead transition and promotes its RefState pointer without appending a
record. Format-2 publication never permits an ahead log.

## Required failpoint matrix

Cover interruption before and after object finalization, candidate write/sync, pointer rename/sync,
log record write/sync, WAL truncate, and active-ref metadata removal. Include unborn and existing refs,
a stale but valid pointer, retries repeated more than once, partial log tails, and injected directory
sync failure. Include a complete record retained after reported log-sync failure, candidate debris,
commit attempts during every incomplete-publication state, the exact released format-1 ahead-log state,
format-1 missing-pointer repair refusal, and every greater-than-one/duplicate/sequence divergence. Every
state must have expected `verify`, `doctor`, retry, read-only, and mutation outcomes.

## Non-goals

- No prepared/finalized RefUpdate schema, distributed transaction, remote ref, or multi-ref commit.
- No unsigned doctor reconstruction and no broad malformed-log repair.
- No active-WAL redesign beyond publication retry requirements.

## Acceptance criteria

The reproduced review sequence no longer returns successful verification and cannot create a duplicate
log transition. The exact legacy split state is recoverable through one pointer promotion and zero new
log appends; unsupported variants fail closed. The full failpoint matrix passes, docs describe the
actual order, and adversarial implementation review accepts the state model.
