# Path and Worktree Safety

This page is the authoritative current-state reference for Prikk's repository path validation and
worktree write-safety boundaries. It describes the current implementation through 0.17.6 and is
grounded in the code, released RFCs, and implementation status records listed in the anchor table at
the foot of the page.

For physical repository layout and `.prikk/` authority boundaries, see
[repository layout and authority](./repository-layout.md). For trust and threat boundaries, see the
[trust and threat model](./trust-threat-model.md). For local lock and stale-lock behavior, see
[concurrency and locking](./concurrency-locking.md).

## Core Caveats

- Prikk is early implementation software and is not a production Git replacement.
- Repository paths currently use a conservative ASCII-only subset.
- Unicode NFC normalization is not implemented; non-ASCII repository paths are rejected.
- Cross-platform conservative checks are enforced even on Unix, including Windows reserved names and
  case-insensitive collision rejection.
- Symlink authoring and symlink materialization are deferred.
- Current materialization safety is check-then-write. It is not an `openat`/`O_NOFOLLOW` design, not a
  canonical realpath proof, and not a race-free guarantee under concurrent worktree modification.
- Linux is the only platform exercised by the current project gates; full cross-platform filesystem
  semantics remain design targets.
- Stable path-format policy, path-policy configuration, Git path compatibility, stable
  repository-format migration, and complete checkout semantics remain deferred.

## Repository Path Shape

Prikk's `RepoPath` is a validated repository-relative path string. The current accepted shape is:

- non-empty;
- ASCII only;
- repository-relative, with no leading `/`;
- slash-separated with `/`;
- made of non-empty components;
- not targeting the top-level `.prikk` metadata directory;
- free of the rejected component forms listed below.

`RepoPath` is a logical repository path, not a host path. Store code joins it to the repository root
only after validation.

## Rejected Path Forms

The current validator rejects:

- empty paths;
- absolute paths that start with `/`;
- backslashes;
- colon characters;
- non-ASCII bytes;
- control bytes `0x00` through `0x1F` and `0x7F`;
- empty path components;
- `.` and `..` components;
- `.prikk` as the first component, case-insensitively;
- components ending in a space or dot;
- Windows reserved component basenames: `CON`, `PRN`, `AUX`, `NUL`, `COM1` through `COM9`, and `LPT1`
  through `LPT9`;
- duplicate paths in a path set; and
- case-insensitive collisions in a path set.

The Windows reserved-name check is matched on the component basename before the first `.`. It is not a
complete Windows path policy and does not include `COM0` or `LPT0`.

The `.prikk` rejection applies to the first component only. A later `.prikk` component is not rejected
by that specific validator rule. Worktree authoring separately skips the top-level `.prikk/` directory.

## Snapshot Manifest Paths

Snapshot manifests decode path bytes as UTF-8 text, parse each path through `RepoPath`, and then
validate path ordering and collisions. Manifest entries must be sorted by repository path. Duplicate
paths and case-insensitive collisions are rejected.

Snapshot entries also carry length-framed content bytes. The path-safety check does not inspect file
content; it validates where the content may be represented or materialized.

## Materialization Safety

Snapshot materialization is opt-in through `prikk checkout --snapshot-materialize`. It writes files only
from a validated snapshot manifest. Patch materialization is opt-in through
`prikk checkout --patch-materialize` and writes the supported patch replay result through the same
shared materializer.

For each materialized file, the current implementation:

- joins the validated `RepoPath` to the repository root;
- checks that the joined path lexically starts with the repository root;
- checks each existing parent directory with symlink-aware metadata and refuses symlink parents;
- refuses non-directory parent paths;
- checks an existing final target with symlink-aware metadata;
- refuses symlink targets;
- refuses non-file targets;
- leaves existing files unchanged when bytes already match;
- refuses to overwrite existing files with different bytes;
- writes new file bytes through the current atomic file-write helper; and
- never removes extra worktree files during ordinary snapshot or patch materialization.

This is intentionally conservative, but it is not complete symlink-escape protection. The containment
check is lexical rather than canonicalized realpath proof. Parent and target checks happen before the
write. A concurrent process that mutates the worktree between checks and writes is outside the current
guarantee.

## Deletion Safety

Patch deletion is a separate opt-in path:

```text
prikk checkout --patch-materialize-delete [path] [--ref REF]
```

The command removes only files that the replayed supported patch chain explicitly removed with a
`DeleteFile` operation. Before removal, Prikk checks the current worktree target with symlink-aware
metadata, refuses symlink targets, refuses non-regular targets, and requires the current bytes to match
the deleted file's old Blob precondition bytes.

Already-absent deletion targets are counted separately. Arbitrary untracked files are never deleted,
and general checkout pruning remains deferred.

## Worktree Authoring Safety

`prikk commit` enumerates regular worktree files, skips the top-level `.prikk/` metadata directory, and
validates identity-bearing paths through `RepoPath`.

The current authoring path:

- rejects symlink entries because symlink authoring is out of scope;
- rejects non-regular entries;
- rejects non-UTF-8 host paths before they can become repository paths;
- validates each repository-relative path through `RepoPath`;
- normalizes regular file modes into Prikk's supported mode representation; and
- rejects snapshot-only published baselines as worktree-authoring identity authority.

Worktree authoring does not infer renames. A move is represented as a deletion plus a creation in the
current supported authoring model.

## Deferred and Not Promised

Still deferred: Unicode NFC normalization, non-ASCII repository paths, symlink authoring, symlink
materialization, full platform path matrix, Git path compatibility, path-policy configuration, stable
repository-format migration, complete checkout pruning, complete branch switching, production merge
execution, and race-free worktree mutation hardening.

Current checks are deliberately strict so future path policy can expand from a conservative baseline.

## Claim-to-Source Anchors

| Claim | Source anchors |
|---|---|
| `RepoPath` accepts only the current ASCII, repository-relative, slash-separated subset. | [`path.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-replay/src/path.rs), [`path/tests.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-replay/src/path/tests.rs), [DC-32](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-32-PATH-WORKTREE-SAFETY-REFERENCE.md) |
| The `.prikk` validator rule applies to the first component case-insensitively. | [`path.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-replay/src/path.rs), [repository layout](./repository-layout.md) |
| Duplicate paths and case-insensitive collisions are rejected. | [`path.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-replay/src/path.rs), [`snapshot.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/snapshot.rs) |
| Snapshot manifests decode UTF-8 path bytes, parse `RepoPath`, enforce sorted paths, and length-frame content bytes. | [`snapshot.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/snapshot.rs), [snapshot checkout guide](../guide/checkout/snapshot-checkout.md) |
| Snapshot materialization writes only validated snapshot entries and uses the shared safe materializer. | [`worktree.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/worktree.rs), [snapshot materialization guide](../guide/checkout/snapshot-materialization.md) |
| Patch materialization writes supported replay results through the shared safe materializer. | [`patch_checkout.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/patch_checkout.rs), [patch materialization guide](../guide/patches/patch-materialization.md) |
| Materialization checks lexical root-containment and refuses symlink parents, symlink targets, non-file targets, and conflicting existing files. | [`worktree.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/worktree.rs), [`path.rs` store adapter](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/path.rs) |
| Materialization writes use the current atomic file-write helper. | [`fsutil.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/fsutil.rs), [`worktree.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/worktree.rs) |
| Patch deletion is opt-in, deletes only explicit replay deletions, and requires current bytes to match old Blob precondition bytes. | [`patch_checkout.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/patch_checkout.rs), [patch deletions guide](../guide/patches/patch-deletions.md) |
| Worktree authoring skips top-level `.prikk/`, rejects symlinks/non-regular entries, rejects non-UTF-8 paths, validates through `RepoPath`, and rejects snapshot-only baselines. | [`node_authoring.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/worktree_patch/node_authoring.rs), [worktree patch guide](../guide/patches/worktree-patch.md) |
| Current trust/threat docs treat `.prikk` private paths and absolute host paths as sensitive diagnostics material. | [trust and threat model](./trust-threat-model.md), [patch algebra reference](./patch-algebra.md) |

## Provenance

This reference implements DC-32 as a documentation-only extension of the current-state reference series.
It adds no code, schema, CLI behavior, checkout behavior, materialization behavior, worktree authoring
behavior, repository behavior, trust behavior, verification behavior, release semantics, or stable path
policy guarantee.
