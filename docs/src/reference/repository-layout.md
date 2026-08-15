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
- Repository *mutation* is exercised by project gates on Linux and macOS; Windows mutation remains
  unimplemented, so cross-platform fsync and path semantics for mutation remain a design target there.
  Read-only commands are CI-gated on macOS and Windows too — see
  [platform support](./platform-support.md).
- Stable repository-format migration, garbage collection, quarantine enforcement, cache rebuilding,
  hosted forge trust, remotes/sync, and production merge execution remain deferred.

## Initialized Layout

A fresh `prikk init` creates the repository directory, the initialized directories below, and the
format marker file. It does not create runtime leaf files such as the active WAL or active ref
metadata. Ref, received-ref, and trust storage are the exception: the ref-pointer index, ref-log,
received-ref index, and trust containers are all fixed, named files allocated by `init` itself, empty
until first use — there is no per-ref, per-received-ref, or per-key-id file or directory created
later, since none of those names exist at `init` time and a per-name file would have to be.

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
    containers/
      log-a.container
      log-b.container
      pointer-index.container
      received-index.container
    locks/
    by-id/
    logs/
    tmp/
  trust/
    keys.container
    policy.container
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

Ref pointer and ref-log storage is shared, not per-ref: every ref's own pointer entry and log records
live in containers allocated once at `init`, not in a file named for that ref. A ref name does not
exist until `branch create`/`tag create` mints it, well after `init` — a per-ref file could never be
one of `init`'s own fixed names, so pointers and logs for every ref instead interleave inside:

```text
refs/containers/pointer-index.container
refs/containers/log-a.container
refs/containers/log-b.container
refs/locks/<ref-name-storage-key>.lock
```

Locks remain per-ref files, one per ref name actually in use, unaffected by this: the storage key is
the hex SHA-256 digest of the ref name bytes, the same digest used internally to attribute each
container entry to its own ref.

`pointer-index.container` is an append-only, checksum-framed sequence of pointer entries; each entry
records one ref's human-readable name, its published RefState object id, and the SHA-256 key derived
from that name. A ref's current pointer is its *last* matching entry — republishing a ref appends a
new entry rather than rewriting the old one. It is useful for lookup and recovery, but is not trusted
alone: verification checks pointer content against RefState objects and ref-log evidence, the same as
before.

`log-a.container`/`log-b.container` hold every ref's RefUpdate log records, interleaved by append
order; a reader filters to one ref's own subsequence by that same per-record key. Each record carries
a signed RefUpdate envelope inline with frame magic, versioning, length, and checksum. Ref logs are
publication evidence when their record chain, referenced RefState objects, target Blocks, signatures,
and trust policy checks all hold — unchanged in meaning, only in storage shape. (Slot `b` is allocated
alongside slot `a` for future container rotation; current writes always target slot `a`.)

`refs/locks/*.lock` files are local synchronization files for ref-specific publication and repair, not
a root of trust. There is no longer a temporary-candidate mechanism: an append-only pointer entry has
no candidate value to stage before becoming durable, so a publish that used to write, sync, and
promote a candidate now durably appends the pointer entry directly, in one step.

`refs/by-id/`, `refs/logs/`, and `refs/tmp/` are still initialized directories — join `cache/` and
`quarantine/` above as present-but-not-authoritative: nothing writes into any of the three anymore, but
`init` still allocates them, and current released behavior does not remove them or treat their
presence as meaningful.

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

The repository-local MAINTAINER trust containers are:

```text
trust/keys.container
trust/policy.container
```

Both are allocated empty at `init`, then appended to by `prikk trust maintainer add`/`remove` — no
name is created after `init`.

`keys.container` holds one append-only entry per adopted key id: the key id and its Ed25519 public key.
`policy.container` holds a sequence of complete policy snapshots — each `add` or `remove` appends the
*entire* current adopted-key-id list, not an incremental change; readers take the last complete
snapshot, with `required = 1` meaning any one adopted key's signature suffices (never stored — it is a
constant, not configurable). Seal checks the configured MAINTAINER signer against this repository-local
policy before publication, and verify checks publication envelopes against the same local trust
boundary.

This trust store is authority for current publication-trust checks. It is not remote trust, global
identity, key rotation, hosted forge policy, or a multi-maintainer threshold system. Key revocation is
supported (`prikk trust maintainer remove`): a removed key's material is retained internally (so a
different key presented later under the same id is still refused), but it no longer counts toward the
adopted set or reserves its case-folded id.

## Authority Model

| Path or data | Classification | Current meaning |
|---|---|---|
| `.prikk/FORMAT` | Format gate | Required by repository open; `2` is current writable format and `1` is bounded legacy read-only mode. |
| `objects/<type>/<prefix>/<id>.pobj` | Content-addressed object authority | Authority when the envelope validates and its computed id/type match the path and expected object. |
| `refs/containers/log-a.container` (`log-b.container` reserved) | Publication evidence | Shared, append-only signed RefUpdate records for every ref, interleaved; authoritative only with valid chain, object, signature, and trust checks. |
| `trust/policy.container` and `trust/keys.container` | Repository-local trust authority | Current local MAINTAINER trust input for seal and verify publication-trust checks. |
| `active/default/queue.wal` | Local active-session state | Pending signed Patch envelopes before seal; load-bearing for the active session, not sealed history. |
| `active/default/ref-name` | Local active-session metadata | Identifies which ref owns a non-empty active WAL; not sealed history. |
| `refs/containers/pointer-index.container` | Mutable convenience pointer | Shared, append-only, last-entry-wins RefState pointer for every ref; fast current-state lookup and recovery target; not trusted without RefState/ref-log checks. |
| `active/default/active.lock` and `refs/locks/*.lock` | Local synchronization | Prevent concurrent writers; not history or trust evidence. |
| `cache/`, `quarantine/`, `refs/by-id/`, `refs/logs/`, `refs/tmp/` | Initialized but non-root | Present after init; not current roots of trust and not used for current verify/publication authority. The last three are dead: nothing writes into them since ref publication state moved into containers. |
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
| `.prikk/FORMAT` selects current writable format 2 or bounded legacy format 1. | [`layout.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/layout.rs), [DC-40](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-40-STATE-MERKLE-FORMAT-TRANSITION.md) |
| Persistent object placement uses object-type directories, two-hex fanout, and `.pobj` files. | [`layout.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/layout.rs), [`object_store.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/object_store.rs) |
| Six object types currently have initialized persistent object directories; `RefUpdate` is inline-only in ref logs. | [`layout.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/layout.rs), [`object_store.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/object_store.rs), [`refs/container.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs/container.rs) |
| Ref storage keys are SHA-256 hex digests of human-readable ref names, shared by the pointer index, the log container, and per-ref lock files. | [`layout.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/layout.rs), [`refs.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs.rs) |
| The ref-pointer index is a shared, append-only, last-entry-wins container holding every ref's own current-pointer entry (name, RefState id, storage key). | [`refs/pointer_index.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs/pointer_index.rs), [`refs.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs.rs), [data model](./data-model.md) |
| The ref-log container is a shared, append-only sequence holding every ref's own signed RefUpdate envelopes, interleaved, with frame magic, checksums, and per-ref replay semantics. | [`refs/container.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs/container.rs), [`refs.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs.rs), [durability and crash recovery](./durability-recovery.md) |
| Active WAL and active ref metadata are runtime active-session state, not fresh-init files. | [`active.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/active.rs), [`wal.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/wal.rs), [durability and crash recovery](./durability-recovery.md) |
| Trust policy and maintainer public-key files are written by the trust command and define current repository-local MAINTAINER trust. | [`trust.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/trust.rs), [`layout.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/layout.rs), [security and signing setup](../guide/security-setup.md) |
| Verification checks object placement, ref pointer/log consistency, active WAL state, and publication trust within current limits. | [`verify.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/verify.rs), [integrity and recovery diagnostics](./integrity-recovery.md), [trust and threat model](./trust-threat-model.md) |
| `cache/` and `quarantine/` are initialized but not current roots of trust, and `gc/` is not a current initialized directory. | [`layout.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/layout.rs), [DC-31](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-31-REPOSITORY-LAYOUT-AUTHORITY-REFERENCE.md) |

## Provenance

This reference implements DC-31 as a documentation-only extension of the DC-24 current-state reference
series. It adds no code, schema, CLI behavior, repository behavior, trust behavior, verification
behavior, repair behavior, or repository-format stability guarantee.
