# RFC (accepted) - DC-34 Publication and Identity Authority

**Status.** Accepted after architect re-review on 2026-07-14; downstream implementation remains in
DC-38 through DC-40.
**Target milestone.** M0 - corrective architecture ratification; no release by itself.
**Tracks.** Architect review B1 and B5, and the authority prerequisite for DC-38 through DC-40.
**Touches.** Ref publication state-machine authority, signature-preimage authority, compatibility
decisions, and corrective-program gates. No Rust implementation in this RFC.

## Context

The independent 0.17.7 architecture review found two unresolved identity-bearing contracts. Current
ref publication appends a committed RefUpdate before promoting the authoritative pointer, allowing a
stale valid pointer and an ahead committed log after interruption. Separately, the implemented
signature preimage differs from the older FDD-03 byte contract without a tracked byte-level erratum.

Changing either area without first selecting one authority would risk another internally consistent
implementation that still lacks a reviewable cross-implementation contract. DC-34 is the architecture
decision record required before DC-38 ref publication work, DC-39 signature-contract work, or DC-40
Block identity work begins.

## Decisions

### Ref publication authority

For the current inline committed-log format, the ref pointer is the publication commit point. The
required order is:

1. validate the complete publication and persist all referenced content-addressed objects;
2. durably write the candidate pointer;
3. atomically promote the pointer and complete required parent-directory durability;
4. append and fsync the already-constructed signed RefUpdate as committed audit evidence;
5. clear active WAL and active-ref metadata only after pointer/log agreement is established.

No committed RefUpdate may be appended before pointer promotion. A crash before step 3 leaves the old
published state. A crash after step 3 but before step 4 leaves a new authoritative pointer with a log
lag of exactly one expected transition and an undrained active WAL. That lag is a recognized interrupted
publication state, not a healthy repository state: `verify` must diagnose it and fail closed, while an
idempotent `seal` retry may append the exact expected signed update only after revalidating pointer,
RefState, Block, WAL, sequence, old/new ids, and maintainer trust. Doctor must not synthesize a signed
RefUpdate without signing authority.

The design must reject a pointer/log divergence greater than one transition, an ahead log, a transition
that cannot be derived exactly from retained active state, and duplicate append on retry.

The sole ahead-log exception is bounded compatibility recovery for a format-1 repository written by
the released log-first implementation. Under the ref lock, signer-backed `seal` may promote the
already-signed next transition without appending another record only when all of these agree exactly:
the old pointer, one complete next RefUpdate, its RefState and Block, update sequence, old/new ids,
publication trust, active-ref metadata, and retained WAL Patch ids. Verification reports this as an
interrupted legacy publication and returns non-zero until recovery completes. Doctor diagnoses it but
does not mutate it. Missing evidence, a second ahead record, a partial ahead transition, or any mismatch
is corruption/manual recovery, not a compatibility repair.

### Publication state and command matrix

In this table, "mutation blocked" includes `commit`, trust mutation, rollback-draft append, ref repair,
and any future command that changes repository or active-session state. Read-only commands may inspect
the repository but must preserve the verification/doctor diagnosis. `seal retry` always acquires the
active lock and ref lock in the accepted order before changing retained state.

| Persisted state | `verify` | `doctor` | `seal` retry | `commit` / other mutation |
|---|---|---|---|---|
| Old pointer/log agree; no candidate | Healthy if all other checks pass. | No publication issue. | Starts a normal publication from retained valid WAL. | Existing active-session rules apply. |
| Old pointer/log agree; complete candidate remains | Warning: candidate debris; published state is old. | Reports removable candidate debris but does not remove it automatically. | Validates/replaces the candidate and publishes normally. | Mutation blocked until candidate debris is resolved under the ref lock. |
| New pointer leads log by one; no trailing frame | Non-zero interrupted-publication diagnosis. | Recommends signer-backed seal retry; no append. | Reconstructs and appends the exact expected signed RefUpdate, syncs it, then cleans active state. | Blocked. |
| New pointer leads log by one; structurally incomplete final frame | Non-zero interrupted-publication plus torn-tail diagnosis. | Reports exact truncatable suffix; no signature or append. | Validates the complete prefix and expected transition, truncates only the structurally incomplete final frame under the ref lock, syncs, appends the exact signed update, and syncs again. | Blocked. |
| New pointer with fully framed checksum-invalid or malformed final record | Integrity failure. | Manual recovery; not truncation-safe. | Refuses mutation. | Blocked. |
| New pointer and complete expected log record after reported log-sync error | Pointer/log agreement; active-state retention is reported. | Reports incomplete cleanup only. | Reads and verifies the existing exact record, does not append, then completes cleanup. | Blocked until cleanup. |
| Pointer/log agree; WAL and active-ref metadata retained | Non-zero incomplete-cleanup diagnosis, not chain corruption. | Recommends seal retry. | Revalidates pointer, log, Block, RefState, WAL, and trust; appends nothing; truncates WAL and then removes metadata. | Blocked. |
| Pointer/log agree; WAL empty; active-ref metadata retained | Warning: empty-WAL metadata debris. | Reports cleanup-safe debris. | Removes metadata under the active lock after rechecking empty WAL. | A mutation may first perform the same locked debris cleanup or fail with the prescribed action; it may not append before cleanup. |
| Format-1 old pointer with exactly one complete signed ahead log record and matching retained active state | Non-zero legacy interrupted-publication diagnosis. | Recommends signer-backed seal retry; no mutation. | Validates the bounded compatibility predicate, promotes the existing RefState pointer, appends nothing, checks agreement, then cleans active state. | Blocked. |
| Log ahead in format 2, ahead by more than one, duplicate transition, sequence gap, pointer/log mismatch, or unexplained divergence | Integrity failure. | Manual recovery; no automatic repair. | Refuses mutation. | Blocked. |

A final record is truncation-safe only when framing proves that the file ends before the declared record
body is complete, including a partial header. A fully present declared body with a checksum mismatch,
invalid magic/version, malformed envelope, or invalid signature is not classified as an incomplete
tail and is never auto-truncated. Truncation does not grant signing authority; the subsequent append
still requires signer-backed `seal`.

### Signature-preimage authority

The released implementation is ratified as signature preimage version 1 to preserve signatures already
written by released Prikk versions. The canonical byte sequence is:

1. literal domain bytes `prikk.sig.v1`;
2. signature algorithm as big-endian `u16`;
3. object type as big-endian `u16`;
4. 32-byte ObjectId;
5. signer role as big-endian `u16`;
6. key-id byte length as big-endian `u16`;
7. ASCII key-id bytes.

The immutable version-1 registries are:

| Registry | Code |
|---|---:|
| Signature algorithm Ed25519 | `0x0001` |
| Object type Patch | `0x0001` |
| Object type Block | `0x0002` |
| Object type RefState | `0x0003` |
| Object type RefUpdate | `0x0004` |
| Object type Tag | `0x0005` |
| Object type Attestation | `0x0006` |
| Object type Blob | `0x0007` |
| Object type BlockSummaryCache | `0x0008` |
| Object type RecoveryNote | `0x0009` |
| Object type ProjectGenesis | `0x000a` |
| Signer role AUTHOR | `0x0001` |
| Signer role MAINTAINER | `0x0002` |
| Signer role CI | `0x0003` |
| Signer role AUDIT | `0x0004` |

All numeric fields above are unsigned big-endian values of the stated width. Unknown algorithm,
object-type, or signer-role codes are rejected; there is no pass-through extension behavior in version
1. A key id is valid only when it is non-empty, at most 128 bytes, and every byte is ASCII
alphanumeric, `-`, or `_`. Invalid key ids and values whose length cannot be represented by `u16` are
rejected before signing and while reconstructing a verification preimage.

This is an intentional tracked erratum to the older FDD-03 domain and field order. DC-39 must add a
literal preimage vector and deterministic Ed25519 signature vector. Future preimage changes require a
new explicit version/domain and migration design; they must not silently reinterpret version 1.

### RefUpdate time policy

For the current schema, `created_at == 0` is the canonical no-clock sentinel. It is not an event-time
claim. This preserves deterministic retry construction. A real authoritative event timestamp requires
a versioned schema and a persistence design that retains the exact signed update across interruption.

Production schema-1 RefUpdate writes in every repository format require zero. Format-1 read-only
compatibility accepts a structurally valid historical non-zero value because released decoders allowed
it, but verification reports a non-canonical legacy timestamp warning and does not interpret it as
trusted time. Format-2 verification and every mutation path reject a schema-1 RefUpdate with a non-zero
value. Migration of a historical non-zero value requires a later explicit migration rule; bytes and
signatures are never normalized in place.

## Goals

- Establish one durable publication state machine with a named commit point.
- Establish one byte-level signature-preimage authority.
- Preserve fail-closed verification and prevent unsigned repair authority.
- Give DC-38, DC-39, and DC-40 explicit compatibility constraints.

## Non-goals

- No code, schema, repository-format, CLI, or release-version change.
- No distributed transaction, remote ref, multi-writer consensus, or new lock model.
- No key rotation, timestamp service, threshold signature, or migration implementation.
- No production-readiness or stable-format claim.

## Required follow-up RFCs

- DC-38 implements the ref state machine, retry, verifier, doctor diagnostics, and crash tests.
- DC-39 pins signature vectors, canonical envelope rules, and the no-clock sentinel documentation.
- DC-40 introduces the real state root behind an explicit format/schema transition.

## Review requirements

Architect review must verify that the pointer-first state machine has no unsigned doctor repair path,
that every persisted crash state has one deterministic classification, that retry cannot append a
duplicate transition, that legacy ahead-log recovery is bounded to the exact released format-1 state,
and that the signature-preimage decision is complete at the byte level.

## Acceptance criteria

Architect re-review accepted these decisions on 2026-07-14. Identity-bearing implementation remains
subject to each downstream RFC's own design and dependency gates. DC-34 moves to `done/` only when
DC-38 through DC-40 have shipped or a later RFC explicitly supersedes the remaining authority.
