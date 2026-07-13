# RFC (done) - DC-31 Repository Layout and Authority Reference

**Status.** Released in 0.17.5.
**Target release.** 0.17.5.
**Tracks.** TASK-10 repository layout and authority model.
**Touches.** mdBook reference documentation, mdBook navigation, README/reference cross-links,
ROADMAP/status docs.
**Companion handoff.** None. This is a current-state reference page and does not create a gating FDD.

## Context

DC-24 established the current data model and trust/threat references. DC-26 moved current-state
references into the published mdBook. DC-28 through DC-30 then filled the durability/recovery,
verify/doctor, and signing setup gaps.

The next documentation gap is the physical repository layout and its authority model. Users can infer
parts of `.prikk/` from command output, implementation code, and scattered references, but there is no
single current-state page that explains which paths exist, what each path is for, and which persisted
facts are authority versus convenience pointers or rebuildable/deferred storage.

DC-31 should close that gap without changing repository behavior or making repository-format stability
claims.

## Problem

1. **The `.prikk/` layout is visible but not centrally documented.** The data-model page explains
   conceptual objects, refs, and active WALs, but it intentionally does not enumerate the current
   on-disk tree.
2. **Authority boundaries are easy to overread.** Ref pointer files, cache-like locations, and local
   active-session metadata are useful operational files, but they are not all equal roots of trust.
3. **Current and future directories can be confused.** The current layout creates `cache/` and
   `quarantine/`, but current public behavior does not use them as trust roots. The layout does not
   create a `gc/` directory today.
4. **Repository format is not stable.** A layout reference must help current operators without
   implying stable storage compatibility or migration guarantees.
5. **Later documentation depends on this vocabulary.** Path safety, concurrency/locking, and release
   compatibility references will be clearer if layout and authority terms are already fixed.

## Design Goals

1. Add a current-state reference page at `docs/src/reference/repository-layout.md`.
2. Document the current initialized top-level `.prikk/` paths from repository initialization:
   `RepositoryLayout::required_directories()` directories plus the `.prikk/FORMAT` file.
3. Document current object placement:
   `objects/{patch,block,ref-state,tag,attestation,blob}/<hex-prefix>/<object-id>.pobj`.
4. Document that `RefUpdate` is stored inline in ref logs, not in an object-store directory.
5. Document current active-session paths:
   `active/default/queue.wal`, `active/default/active.lock`, and `active/default/ref-name`.
6. Document current ref paths:
   `refs/by-id/<ref-name-storage-key>.ref`, `refs/logs/<ref-name-storage-key>.log`,
   `refs/locks/<ref-name-storage-key>.lock`, and `refs/tmp/<ref-name-storage-key>.tmp`.
7. Document current trust-store paths:
   `trust/policy.toml` and `trust/keys/maintainer/<key-id>.pub`.
8. Document `.prikk/FORMAT` as the load-bearing current format-version marker used by
   `RepositoryLayout::open`, with the current value `"1"`, while explicitly stating that this is not a
   stable-format or migration guarantee.
9. Explain the authority model:
   content-addressed object envelopes, signed publication envelopes, signed inline RefUpdate log
   records, WAL records containing exact signed Patch envelopes, and repository-local trust policy/key
   files are load-bearing evidence for current behavior.
10. Explain that ref pointer files are mutable convenience/recovery pointers that must be checked
   against RefState objects and ref logs.
11. Explain that `cache/` and `quarantine/` are initialized directories but are not current roots of
    trust, and that `gc/` is not a current initialized directory.
12. Cross-link the data model, trust/threat model, durability/recovery, integrity/recovery, signing
    setup, and later release/compatibility work where appropriate.
13. Include visible claim-to-source anchors for each storage and authority claim.

## Non-goals

DC-31 does not add:

- code, schema, CLI behavior, repository behavior, trust behavior, verification behavior, or release
  semantics;
- new directories, path encodings, object file formats, ref-log formats, or trust-store formats;
- stable repository-format or migration guarantees;
- garbage collection, quarantine enforcement, cache rebuilding, index semantics, backup/restore, or
  repair behavior;
- new public API guarantees for `prikk-store` or `prikk-replay`;
- a new current-state FDD under `rfcs/fdds/`.

## Proposed Documentation Shape

Create:

```text
docs/src/reference/repository-layout.md
```

Add it under the mdBook `# Reference` section near the data model:

```md
- [Repository Layout and Authority](reference/repository-layout.md)
```

The page should be a current-state reference. It should not present planned storage as implemented
behavior. Where a directory exists but has no current trust-bearing semantics, say so directly.

### Required Sections

The implemented page should contain at least:

1. **Core Caveats.** Early implementation, not a Git storage format, no stable repository-format
   migration policy, Linux-only exercised gates, and no production replacement claim.
2. **Initialized Layout.** A compact tree for paths created by current repository initialization:
   initialized directories plus `.prikk/FORMAT`. The tree must not show runtime leaf files as present
   immediately after `prikk init`.
3. **Object Store.** Object type directories, two-hex-prefix fanout, `.pobj` files, and the inline-only
   status of `RefUpdate`.
4. **Refs and Ref Logs.** Hash-derived ref storage keys, mutable pointer files, append-only log files,
   ref locks, and temporary candidate files.
5. **Active Session.** Default active WAL, lock, and ref-name metadata paths.
6. **Trust Store.** Maintainer public-key files and fixed trust policy path.
7. **Authority Model.** A table classifying paths as authority, format gate, convenience pointer,
   local session state, local synchronization, initialized-but-non-root, or deferred/not present.
   `.prikk/FORMAT` must be classified as the current format-version marker: load-bearing for opening a
   v1 repository, current value `"1"`, and not a stability or migration guarantee.
8. **Deferred and Not Stable.** No `gc/` directory today, no repository-format stability, no migration
   guarantee, and no cache/quarantine trust-root claim.
9. **Claim-to-Source Anchors.** Code/docs/RFC anchors for layout paths, object placement, ref pointer
   behavior, WAL behavior, trust paths, and current caveats.

### Implementation Review Guards

Implementation review must also verify:

1. The initialized-layout tree distinguishes fresh-init paths from runtime-written leaf files.
   `active/default/queue.wal`, `active/default/ref-name`, `refs/by-id/*.ref`, `refs/logs/*.log`,
   `trust/policy.toml`, and `trust/keys/maintainer/*.pub` must be described as written by later
   operations, not as guaranteed files in a bare initialized repository.
2. The object-store section must not imply that Prikk has only six object types. It may say that six
   object types currently have initialized persistent object directories. It must not document
   `objects/genesis/`, `objects/block-summary-cache-rebuildable/`, or
   `objects/recovery-note-inline-only/` as present current directories.

## Required Source Audit

The implementation must check the final page against:

- `crates/prikk-store/src/layout.rs`
- `crates/prikk-store/src/object_store.rs`
- `crates/prikk-store/src/refs.rs`
- `crates/prikk-store/src/refs/log.rs`
- `crates/prikk-store/src/refs/pointer.rs`
- `crates/prikk-store/src/active.rs`
- `crates/prikk-store/src/wal.rs`
- `crates/prikk-store/src/trust.rs`
- `crates/prikk-store/src/verify.rs`
- `docs/src/reference/data-model.md`
- `docs/src/reference/durability-recovery.md`
- `docs/src/reference/integrity-recovery.md`
- `docs/src/reference/trust-threat-model.md`
- `docs/src/guide/security-setup.md`
- released DC-24, DC-28, DC-29, and DC-30 records

## Review Requirements

Architect review should verify:

1. The proposed scope is documentation-only and does not imply code or format changes.
2. Every path claimed as current is present in current source, especially `cache/`, `quarantine/`, and
   the absence of `gc/`, and `.prikk/FORMAT` is included as the current format marker.
3. The authority model does not make mutable ref pointers, initialized cache/quarantine directories,
   or local lock/tmp files roots of trust.
4. The page plan preserves the no-stable-format and no-migration-guarantee caveats.
5. The required source audit is sufficient for implementation review.

## Acceptance Criteria

DC-31 is ready for implementation only after architect design review accepts this RFC or accepts a
repaired version. Implementation is complete when:

- the reference page exists and is linked in mdBook navigation;
- relevant current docs link to it without duplicating the full page;
- claim-to-source anchors are included;
- `ROADMAP.md`, `rfcs/README.md`, and `rfcs/IMPLEMENTATION-STATUS.md` are updated consistently;
- documentation build/check commands pass in the implementing thread.
