# Prikk

![Status](https://img.shields.io/badge/status-early--implementation-orange)
[![license](https://img.shields.io/crates/l/prikk.svg)](LICENSE)
[![crates.io](https://img.shields.io/crates/v/prikk.svg?label=prikk)](https://crates.io/crates/prikk)
[![docs.rs](https://img.shields.io/docsrs/prikk?version=latest)](https://docs.rs/prikk)
[![Dependency Status](https://deps.rs/crate/prikk/latest/status.svg)](https://deps.rs/crate/prikk)

[![crates.io](https://img.shields.io/crates/v/prikk-crypto.svg?label=crypto)](https://crates.io/crates/prikk-crypto)
[![docs.rs](https://img.shields.io/docsrs/prikk-crypto?version=latest)](https://docs.rs/prikk-crypto)
[![Dependency Status](https://deps.rs/crate/prikk-crypto/latest/status.svg)](https://deps.rs/crate/prikk-crypto)
[![crates.io](https://img.shields.io/crates/v/prikk-error.svg?label=error)](https://crates.io/crates/prikk-error)
[![docs.rs](https://img.shields.io/docsrs/prikk-error?version=latest)](https://docs.rs/prikk-error)
[![Dependency Status](https://deps.rs/crate/prikk-error/latest/status.svg)](https://deps.rs/crate/prikk-error)
[![crates.io](https://img.shields.io/crates/v/prikk-hash.svg?label=hash)](https://crates.io/crates/prikk-hash)
[![docs.rs](https://img.shields.io/docsrs/prikk-hash?version=latest)](https://docs.rs/prikk-hash)
[![Dependency Status](https://deps.rs/crate/prikk-hash/latest/status.svg)](https://deps.rs/crate/prikk-hash)
[![crates.io](https://img.shields.io/crates/v/prikk-object.svg?label=object)](https://crates.io/crates/prikk-object)
[![docs.rs](https://img.shields.io/docsrs/prikk-object?version=latest)](https://docs.rs/prikk-object)
[![Dependency Status](https://deps.rs/crate/prikk-object/latest/status.svg)](https://deps.rs/crate/prikk-object)
[![crates.io](https://img.shields.io/crates/v/prikk-store.svg?label=store)](https://crates.io/crates/prikk-store)
[![docs.rs](https://img.shields.io/docsrs/prikk-store?version=latest)](https://docs.rs/prikk-store)
[![Dependency Status](https://deps.rs/crate/prikk-store/latest/status.svg)](https://deps.rs/crate/prikk-store)

**A next-generation VCS built around block-oriented patch theory.**

## Overview

Prikk is an experimental distributed version control system focused on ease of use, safety,
resilience, flexibility, and long-term performance. The implementation follows the approved FDD
sequence: object identity and storage first, then WAL/ref durability, patch algebra, plugins, and
sync.

## Why / When

Use Prikk development builds when evaluating the architecture or contributing to the implementation.
Do not use Prikk for real project history yet.

## Quick Start

```sh
cargo build -p prikk
export PRIKK="$PWD/target/debug/prikk"

cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
$PRIKK init ./sample-repo
# Author and publish a first commit (genesis) on a fresh repository:
export PRIKK_AUTHOR_KEY_ID="dev-author"
export PRIKK_AUTHOR_SEED="00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff"
export PRIKK_MAINTAINER_KEY_ID="dev-maintainer"
export PRIKK_MAINTAINER_SEED="111122223333444455556666777788889999aaaabbbbccccddddeeeeffff0000"
(cd ./sample-repo && "$PRIKK" trust maintainer add \
  --key-id "$PRIKK_MAINTAINER_KEY_ID" \
  --public-key "a00899dfd3357aee69729405913f9324dfc033cec04a2215239eda64ae6d9d91")
echo "hello prikk" > ./sample-repo/readme.txt
(cd ./sample-repo && "$PRIKK" commit -m "genesis")
(cd ./sample-repo && "$PRIKK" seal --allow-no-audit)
$PRIKK log ./sample-repo
$PRIKK worktree-status ./sample-repo
$PRIKK verify ./sample-repo
$PRIKK doctor ./sample-repo
# If doctor reports only incomplete trailing WAL bytes:
# $PRIKK doctor ./sample-repo --repair-wal-tail
# If doctor reports only a missing heads/main pointer recoverable from the ref log:
# $PRIKK doctor ./sample-repo --repair-main-ref
```

`prikk commit` authors node-addressed worktree patches signed with a real role-bound Ed25519 AUTHOR
signature; key material is supplied via `PRIKK_AUTHOR_KEY_ID` / `PRIKK_AUTHOR_SEED` (a minimal key-input
mechanism, not a trust store). On a fresh repository the first commit is a **genesis** commit (all files
authored as `CreateFile`); the first `seal` publishes a Root block on `heads/main`. An explicit unborn
local branch ref can be started with `commit --ref heads/<branch>` and published with
`seal --ref heads/<branch>` as an independent Root history.

## Design Notes

Current released implementation: **0.14.0** (DC-21 — merge conflict evidence contract). This release
adds an internal, read-only merge/conflict evidence report vocabulary over the existing patch-algebra
analyzers, with
explicit `EvidenceFailure`, `InvalidCandidate`, `Unsupported`, `Deferred`, `Conflict`,
`OrderedDependency`, `NotConfluent`, and `Confluent` outcomes. It does not add CLI merge behavior,
merge execution, branch publication, multi-parent Blocks, persisted proof/witness objects, schema
changes, worktree conflict materialization, patch-algebra extraction, or public `prikk-replay` API
stabilization.

Current development candidate for **0.15.0** (DC-22) adds `prikk merge-evidence` as a read-only public
display over the DC-21 report contract. It requires an explicit `--baseline-block`, resolves exactly
one left and one right target selector from `--*-block` or `--*-ref`, derives sealed candidate
sequences by walking single-parent ancestry back to the baseline, and reports evidence without writing
objects, refs, WAL records, merge commits, or worktree files.

Implemented:

- Rust workspace scaffold.
- Deterministic canonical object identity seed.
- Object envelopes with signatures outside identity.
- Persistent `.prikk/` layout and object store.
- Active-session WAL append/replay for signed patch envelopes.
- Read-only repository verification for objects, block references, sealed rollback Patch classification, ref pointers, ref logs, active WAL, and publication trust.
- `doctor` diagnostics layered on top of verification, with opt-in safe WAL tail and missing-ref-pointer repair.
- Read-only sealed-history inspection from the current RefState chain, including rollback block labels.
- Snapshot-manifest validation, path-safety checks, opt-in snapshot materialization, and read-only worktree status.
- Initial RefState publication primitives with flat hashed ref pointer paths.
- Node-addressed worktree patch authoring (`prikk commit`): against a published local branch baseline reconstructed from authoritative replay — or, on an unborn `heads/*` ref, a **genesis** first commit against an empty baseline (all files authored as `CreateFile`) — worktree changes are authored as node-addressed §9.3 operations (`CreateFile`, `DeleteNode`, `EditText`, `ReplaceBinary`, `ChangePerm`) with CSPRNG-minted node identities in canonical order, normalized file modes, and shared text-span identity. Existing-node kind is authoritative; rename inference, symlink authoring, branch copy/fork, branch switching, and text↔binary transitions are out of scope.
- **Role-bound Ed25519 AUTHOR signing** for production Patch authoring paths: worktree commits and rollback drafts sign through an injected `AuthorSigner`; the production `Ed25519AuthorSigner` produces a real Ed25519 signature over the role-bound preimage (`Ed25519, Patch, unsigned-patch-id, Author, key_id`). Key material is supplied via `PRIKK_AUTHOR_KEY_ID` / `PRIKK_AUTHOR_SEED` (a minimal key-input mechanism, not a trust store).
- Local no-audit seal scaffold that persists WAL patches, creates a Block, signs publication objects with a trusted MAINTAINER key, and advances `heads/main` or an explicit `--ref heads/<branch>`.
- Active-WAL ref ownership metadata prevents sealing queued patches to a different ref than the one they were authored for. Non-empty active WALs with missing, malformed, or mismatched ref metadata fail closed.
- Supported patch replay planning/materialization for `CreateFile`/`DeleteNode` and deterministic arbitrary-span `EditText`, with node-addressed record reconciliation for the remaining §9.3 kinds.
- Explicit deletion planning and opt-in deletion of patch-removed files whose bytes still match the old blob.
- Read-only inverse planning, non-mutating rollback preview, rollback-draft append/verification, and sealed rollback block classification for the supported subset, including deterministic direct inverse for supported arbitrary-span `EditText`. Rollback-draft identity is recorded as `PatchPurpose::RollbackDraft`, not as a reserved AUTHOR key id.
- Minimal local publication trust: `prikk trust maintainer add` records one trusted MAINTAINER public key, and `verify` checks Block/RefState/RefUpdate MAINTAINER signatures against that policy.
- Internal patch-algebra foundation for pair classification (`Independent`, `OrderedDependency`,
  `Conflict`, `Unknown`) plus replay-backed pair commutation and flat two-sequence confluence for the
  supported subset. This is not a public merge/conflict API.
- Internally scoped `prikk-replay` crate for replay/lifecycle semantics. It owns the node lifecycle
  substrate and lexical repository path type needed by lifecycle state, while repository layout, object
  storage, refs, WAL, active sessions, lifecycle-cache persistence, verification, doctor, worktree
  integration, and store-backed resolver construction remain in `prikk-store`.
- Replay-boundary stabilization keeps `prikk-replay` internally scoped and non-stable as an external
  Rust API, keeps `prikk-store` compatibility wrappers import/reexport-oriented, and keeps filesystem
  root joining in `prikk-store`. The crate is publishable for workspace release packaging, but that
  does not make its public items stable API.
- Read-only public merge evidence UX (`prikk merge-evidence`) over sealed object histories, with
  explicit baseline selection, block/ref target selectors, resolved target identities, DC-21 outcome
  and reason-code display, and no repository or worktree mutation.

Signing scope (interim): AUTHOR-role Patch signatures and MAINTAINER publication signatures produced by production commands are real role-bound Ed25519 signatures. Publication trust is local and minimal (`required = 1`); this does **not** yet imply key rotation, revocation, expiration, multi-maintainer thresholds, remote trust, hardware signing, or publication-grade audit policy.

Minimal CLI commands: `init`, `trust maintainer add`, `commit [--from-worktree] [--text-edits] [--ref heads/<branch>] -m`, `seal --allow-no-audit [--ref heads/<branch>]`, `status`, `log`, `checkout --plan-only`, `checkout --snapshot-plan`, `checkout --snapshot-materialize`, `checkout --patch-plan`, `checkout --patch-materialize`, `checkout --patch-delete-plan`, `checkout --patch-materialize-delete`, `merge-evidence --baseline-block`, `inverse-plan`, `rollback-preview`, `rollback-draft --append-inverse`, `rollback-draft-verify`, `worktree-status`, `verify`, `doctor`, `doctor --repair-wal-tail`, `doctor --repair-main-ref`, and `--version`.

Not implemented yet:

- Rename detection, multi-operation text diff minimization, rollback refs, rollback authorization,
  public conflict witnesses, semantic merge, merge execution, and general destructive checkout
  pruning.
- Branch switching, branch copy/fork from an existing tip, merge-base semantics, branch deletion/rename,
  tag or remote ref creation, rollback refs, multi-commit queued active sessions, and per-ref active WALs.
- Key management/rotation, revocation, expiration, multi-maintainer thresholds, remote trust, hardware signing, and broader signature policy.
- Policy-aware audit/attestation publication through seal; plugin/audit execution.
- Remote sync.

## More Detail

Full documentation is kept under `docs/src` and is structured for mdBook.
