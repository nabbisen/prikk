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
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p prikk -- init ./sample-repo
# Author and publish a first commit (genesis) on a fresh repository:
export PRIKK_AUTHOR_KEY_ID="dev-author"
export PRIKK_AUTHOR_SEED="00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff"
export PRIKK_MAINTAINER_KEY_ID="dev-maintainer"
export PRIKK_MAINTAINER_SEED="111122223333444455556666777788889999aaaabbbbccccddddeeeeffff0000"
(cd ./sample-repo && ../target/debug/prikk trust maintainer add \
  --key-id "$PRIKK_MAINTAINER_KEY_ID" \
  --public-key "a00899dfd3357aee69729405913f9324dfc033cec04a2215239eda64ae6d9d91")
echo "hello prikk" > ./sample-repo/readme.txt
(cd ./sample-repo && ../target/debug/prikk commit -m "genesis")
(cd ./sample-repo && ../target/debug/prikk seal --allow-no-audit)
cargo run -p prikk -- log ./sample-repo
cargo run -p prikk -- worktree-status ./sample-repo
cargo run -p prikk -- verify ./sample-repo
cargo run -p prikk -- doctor ./sample-repo
# If doctor reports only incomplete trailing WAL bytes:
# cargo run -p prikk -- doctor ./sample-repo --repair-wal-tail
# If doctor reports only a missing heads/main pointer recoverable from the ref log:
# cargo run -p prikk -- doctor ./sample-repo --repair-main-ref
```

`prikk commit` authors node-addressed worktree patches signed with a real role-bound Ed25519 AUTHOR
signature; key material is supplied via `PRIKK_AUTHOR_KEY_ID` / `PRIKK_AUTHOR_SEED` (a minimal key-input
mechanism, not a trust store). On a fresh repository the first commit is a **genesis** commit (all files
authored as `CreateFile`); the first `seal` publishes a Root block on `heads/main`.

## Design Notes

Current implementation drop: **0.4.0** (DC-11 — publication signing and minimal trust store).

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
- Node-addressed worktree patch authoring (`prikk commit`): against a published `heads/main` baseline reconstructed from authoritative replay — or, on a fresh repository, a **genesis** first commit against an empty baseline (all files authored as `CreateFile`) — worktree changes are authored as node-addressed §9.3 operations (`CreateFile`, `DeleteNode`, `EditText`, `ReplaceBinary`, `ChangePerm`) with CSPRNG-minted node identities in canonical order, normalized file modes, and shared text-span identity. Existing-node kind is authoritative; rename inference, symlink authoring, and text↔binary transitions are out of scope.
- **Role-bound Ed25519 AUTHOR signing** for production Patch authoring paths: worktree commits and rollback drafts sign through an injected `AuthorSigner`; the production `Ed25519AuthorSigner` produces a real Ed25519 signature over the role-bound preimage (`Ed25519, Patch, unsigned-patch-id, Author, key_id`). Key material is supplied via `PRIKK_AUTHOR_KEY_ID` / `PRIKK_AUTHOR_SEED` (a minimal key-input mechanism, not a trust store).
- Local no-audit seal scaffold that persists WAL patches, creates a Block, signs publication objects with a trusted MAINTAINER key, and advances `heads/main`.
- Supported patch replay planning/materialization for `CreateFile`/`DeleteNode`, with full-file `EditText` and node-addressed record reconciliation for the remaining §9.3 kinds.
- Explicit deletion planning and opt-in deletion of patch-removed files whose bytes still match the old blob.
- Read-only inverse planning, non-mutating rollback preview, rollback-draft append/verification, and sealed rollback block classification for the supported subset. Rollback-draft identity is recorded as `PatchPurpose::RollbackDraft`, not as a reserved AUTHOR key id.
- Minimal local publication trust: `prikk trust maintainer add` records one trusted MAINTAINER public key, and `verify` checks Block/RefState/RefUpdate MAINTAINER signatures against that policy.

Signing scope (interim): AUTHOR-role Patch signatures and MAINTAINER publication signatures produced by production commands are real role-bound Ed25519 signatures. Publication trust is local and minimal (`required = 1`); this does **not** yet imply key rotation, revocation, expiration, multi-maintainer thresholds, remote trust, hardware signing, or publication-grade audit policy.

Minimal CLI commands: `init`, `trust maintainer add`, `commit [--from-worktree] [--text-edits] -m`, `seal --allow-no-audit`, `status`, `log`, `checkout --plan-only`, `checkout --snapshot-plan`, `checkout --snapshot-materialize`, `checkout --patch-plan`, `checkout --patch-materialize`, `checkout --patch-delete-plan`, `checkout --patch-materialize-delete`, `inverse-plan`, `rollback-preview`, `rollback-draft --append-inverse`, `rollback-draft-verify`, `worktree-status`, `verify`, `doctor`, `doctor --repair-wal-tail`, `doctor --repair-main-ref`, and `--version`.

Not implemented yet:

- Rename detection, arbitrary text-span discovery/generation, rollback refs, rollback authorization, commutation, full patch algebra, and general destructive checkout pruning.
- Genesis first-commit onto non-default refs (this drop supports genesis on the default `heads/main` only).
- Key management/rotation, revocation, expiration, multi-maintainer thresholds, remote trust, hardware signing, and broader signature policy.
- Policy-aware audit/attestation publication through seal; plugin/audit execution.
- Remote sync.

## More Detail

Full documentation is kept under `docs/src` and is structured for mdBook.
