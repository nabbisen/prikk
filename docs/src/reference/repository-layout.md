# Repository Layout and Authority

This page is the authoritative current-state reference for Prikk's on-disk repository layout and
storage authority boundaries. It describes the current implementation prepared for 0.18.0 and is grounded
in the code, released RFCs, and implementation status records listed in the anchor table at the foot of
the page.

For logical object concepts, see the [data model](./data-model.md). For repository path and worktree
write-safety rules, see [path and worktree safety](./path-safety.md). For local persistence and
recovery behavior, see [durability and crash recovery](./durability-recovery.md). For trust and
signature scope, see the [trust and threat model](./trust-threat-model.md). For lock and ref
compare-and-swap behavior, see [concurrency and locking](./concurrency-locking.md).
For format stability, migration limits, and release identity, see
[release, versioning, and compatibility](./release-compatibility.md).

## Core Caveats

- Prikk is early implementation software and is not a production Git replacement.
- `.prikk/` is Prikk's native repository format and is not Git-compatible storage.
- `.prikk/FORMAT` is a current format-version gate, not a stable-format or migration guarantee.
- Ref files are mutable pointers for convenience and recovery, not roots of trust.
- Cache-like and quarantine-like directories are initialized today, but are not current roots of
  trust.
- Durability and recovery claims are supported by current unit and integration tests, not by a
  completed crash-matrix or fuzzing campaign.
- Repository *mutation* is exercised by project gates on Linux only; cross-platform fsync and path
  semantics for mutation remain design targets. Read-only commands are CI-gated on macOS and Windows
  too — see [platform support](./platform-support.md).
- Stable repository-format migration, garbage collection, quarantine enforcement, cache rebuilding,
  hosted forge trust, remotes/sync, and production merge execution remain deferred.

## Initialized Layout

A fresh `prikk init` creates the repository directory, the initialized directories below, and the
format marker file. It does not create runtime leaf files such as WALs, ref pointers, ref logs, trust
policy files, or maintainer key files.

```text
.prikk/
  FORMAT
  objects/
    patch/
    block/
    ref-state/
    tag/
    attestation/
    blob/
  active/
    default/
  refs/
    by-id/
    logs/
    locks/
    tmp/
  trust/
    keys/
      maintainer/
  cache/
  quarantine/
```

New repositories contain `2` in `FORMAT`. Opening value `2` selects the current writable format;
opening value `1` selects bounded legacy read-only behavior. Every other value is unsupported. The
marker is load-bearing and is never inferred from individual objects.

Format 2 admits schema-2 Blocks and schema-1 Patch, RefState, RefUpdate, Tag, Attestation, and Blob
envelopes in their authorized locations. Ordinary object, WAL, ref, trust, repair, and worktree writes
are refused for format 1. Read-only inspection and planning remain available with a warning. `verify`
performs bounded structural/signature checks but returns nonzero because format-1 scaffold roots are
not state commitments. The sole legacy mutation is exact signer-backed completion of DC-34's retained
one-record-ahead interrupted publication; it promotes existing signed state without rewriting identity
bytes or appending another log record.

`cache/` and `quarantine/` are initialized directories. Current released behavior does not use either
directory as authority for verification, publication, recovery, or trust.

There is no initialized `gc/` directory today.

## Object Store

Prikk stores persistent object envelopes under object-type directories. Current initialized persistent
object directories are:

- `objects/patch/`
- `objects/block/`
- `objects/ref-state/`
- `objects/tag/`
- `objects/attestation/`
- `objects/blob/`

When an object is written, its storage path is:

```text
objects/<object-type>/<first-two-object-id-hex>/<object-id>.pobj
```

The two-hex fanout directory is created when the object is written. It is not present in a fresh
repository unless an object with that prefix has been persisted.

The current object type enum contains additional internal or future-facing type names, but only the
six directories listed above are initialized persistent object directories today. `RefUpdate` is stored
inline in ref logs, not as an object-store file. Current docs should not describe
`objects/genesis/`, `objects/block-summary-cache-rebuildable/`, or
`objects/recovery-note-inline-only/` as present directories.

Object files are authority only when their envelope decodes, validates, has the expected type, and its
computed content-addressed object id matches the requested id and path.

Object publication is immutable and no-clobber. A new object is written and synced through a unique
same-shard temp, installed without replacing an existing final name, and followed by required shard
sync and invocation-owned temp cleanup. An existing final name succeeds only when one no-follow
regular-file read proves valid identity/type and exact persisted-byte equality with the candidate.

Crash-left names matching `<object-id>.pobj.tmp.<pid>.<random>` are non-authoritative local debris.
Canonical reads ignore them; `verify` and `doctor` warn without deleting them or inferring ownership.

## Refs and Ref Logs

Ref storage paths use a storage key derived from the human-readable ref name:

```text
refs/by-id/<ref-name-storage-key>.ref
refs/logs/<ref-name-storage-key>.log
refs/locks/<ref-name-storage-key>.lock
refs/tmp/<ref-name-storage-key>.tmp
```

The storage key is the hex SHA-256 digest of the ref name bytes.

`refs/by-id/*.ref` files are mutable pointer files. A ref pointer stores the human-readable ref name
and the current RefState object id. It is useful for lookup and recovery, but is not trusted alone.
Verification and repair check pointer content against RefState objects and ref-log evidence.

`refs/logs/*.log` files contain append-only RefUpdate log records. Each record carries a signed
RefUpdate envelope inline with log framing, versioning, length, and checksum. Ref logs are publication
evidence when their record chain, referenced RefState objects, target Blocks, signatures, and trust
policy checks all hold.

`refs/locks/*.lock` files are local synchronization files for ref-specific publication and repair.
`refs/tmp/*.tmp` files are temporary pointer candidates used during pointer promotion. Neither is a
root of trust.

## Active Session

The default active-session paths are:

```text
active/default/queue.wal
active/default/active.lock
active/default/ref-name
```

These files are runtime-written, not guaranteed members of a bare initialized repository.

`queue.wal` stores exact signed Patch envelopes before sealing. WAL records are load-bearing local
session state: they are the pending changes that `seal` replays and publishes, but they are not sealed
history until publication succeeds.

`ref-name` records which local branch ref owns a non-empty active WAL. Missing or malformed active ref
metadata on a non-empty WAL is an integrity issue because seal must not guess the publication target.

`active.lock` is a local synchronization file. It prevents concurrent active-session writers, but it is
not evidence of repository history or trust. Stale lock cleanup after a crash is manual today; see
[concurrency and locking](./concurrency-locking.md) for the current operator boundary.

## Trust Store

The current repository-local MAINTAINER trust paths are:

```text
trust/policy.toml
trust/keys/maintainer/<key-id>.pub
```

These files are written by `prikk trust maintainer add`, not by bare repository initialization.

The trust policy currently supports one trusted MAINTAINER key with `required = 1`. The maintainer key
file contains the trusted Ed25519 public key for that storage-safe key id. Seal checks the configured
MAINTAINER signer against this repository-local policy before publication, and verify checks
publication envelopes against the same local trust boundary.

This trust store is authority for current publication-trust checks. It is not remote trust, global
identity, key rotation, key revocation, hosted forge policy, or a multi-maintainer threshold system.

## Authority Model

| Path or data | Classification | Current meaning |
|---|---|---|
| `.prikk/FORMAT` | Format gate | Required by repository open; `2` is current writable format and `1` is bounded legacy read-only mode. |
| `objects/<type>/<prefix>/<id>.pobj` | Content-addressed object authority | Authority when the envelope validates and its computed id/type match the path and expected object. |
| `refs/logs/*.log` | Publication evidence | Append-only signed RefUpdate records; authoritative only with valid chain, object, signature, and trust checks. |
| `trust/policy.toml` and `trust/keys/maintainer/*.pub` | Repository-local trust authority | Current local MAINTAINER trust input for seal and verify publication-trust checks. |
| `active/default/queue.wal` | Local active-session state | Pending signed Patch envelopes before seal; load-bearing for the active session, not sealed history. |
| `active/default/ref-name` | Local active-session metadata | Identifies which ref owns a non-empty active WAL; not sealed history. |
| `refs/by-id/*.ref` | Mutable convenience pointer | Fast current RefState lookup and recovery target; not trusted without RefState/ref-log checks. |
| `active/default/active.lock` and `refs/locks/*.lock` | Local synchronization | Prevent concurrent writers; not history or trust evidence. |
| `refs/tmp/*.tmp` | Local promotion workspace | Temporary ref pointer candidate during publication or repair. |
| `cache/` and `quarantine/` | Initialized but non-root | Present after init; not current roots of trust and not used for current verify/publication authority. |
| `gc/` | Deferred/not present | No current initialized directory or released behavior. |

## Deferred and Not Stable

Prikk does not provide in-place or history-preserving migration from format 1 to format 2. The
documented writable path is a newly initialized format-2 repository followed by deliberate worktree
re-authoring, which creates new NodeIds, objects, signatures, and history. Copying `.prikk/` data or
editing `FORMAT` is not migration. This explicit transition does not promise general format stability.

Still deferred: garbage collection, cache rebuild semantics, quarantine enforcement, stable
repository-format migration, backup/restore workflows, remote trust, hosted forge semantics, complete
branch management, tags/remotes, sync, production merge execution, and full cross-platform filesystem
validation.

## Claim-to-Source Anchors

| Claim | Source anchors |
|---|---|
| Repository initialization creates the listed directories and writes `.prikk/FORMAT`. | [`layout.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/layout.rs), [DC-31](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-31-REPOSITORY-LAYOUT-AUTHORITY-REFERENCE.md) |
| `.prikk/FORMAT` selects current writable format 2 or bounded legacy format 1. | [`layout.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/layout.rs), [DC-40](https://github.com/nabbisen/prikk/blob/main/rfcs/accepted/DC-40-STATE-MERKLE-FORMAT-TRANSITION.md) |
| Persistent object placement uses object-type directories, two-hex fanout, and `.pobj` files. | [`layout.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/layout.rs), [`object_store.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/object_store.rs) |
| Six object types currently have initialized persistent object directories; `RefUpdate` is inline-only in ref logs. | [`layout.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/layout.rs), [`object_store.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/object_store.rs), [`refs/log.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs/log.rs) |
| Ref storage keys are SHA-256 hex digests of human-readable ref names. | [`layout.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/layout.rs), [`refs.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs.rs) |
| Ref pointer files are mutable pointer files containing the ref name and RefState id. | [`refs/pointer.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs/pointer.rs), [`refs.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs.rs), [data model](./data-model.md) |
| Ref logs contain signed RefUpdate envelopes inline with log framing, checksums, and replay semantics. | [`refs/log.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs/log.rs), [`refs.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs.rs), [durability and crash recovery](./durability-recovery.md) |
| Active WAL and active ref metadata are runtime active-session state, not fresh-init files. | [`active.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/active.rs), [`wal.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/wal.rs), [durability and crash recovery](./durability-recovery.md) |
| Trust policy and maintainer public-key files are written by the trust command and define current repository-local MAINTAINER trust. | [`trust.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/trust.rs), [`layout.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/layout.rs), [security and signing setup](../guide/security-setup.md) |
| Verification checks object placement, ref pointer/log consistency, active WAL state, and publication trust within current limits. | [`verify.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/verify.rs), [integrity and recovery diagnostics](./integrity-recovery.md), [trust and threat model](./trust-threat-model.md) |
| `cache/` and `quarantine/` are initialized but not current roots of trust, and `gc/` is not a current initialized directory. | [`layout.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/layout.rs), [DC-31](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-31-REPOSITORY-LAYOUT-AUTHORITY-REFERENCE.md) |

## Provenance

This reference implements DC-31 as a documentation-only extension of the DC-24 current-state reference
series. It adds no code, schema, CLI behavior, repository behavior, trust behavior, verification
behavior, repair behavior, or repository-format stability guarantee.
