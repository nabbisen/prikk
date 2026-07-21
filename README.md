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

**Prikk is a standalone distributed version control system built around block-oriented patch theory.**

Prikk uses a native `.prikk/` repository format. It is not a Git wrapper and does not use `.git/` as a
storage backend. The project aims to combine patch-based semantic precision with practical performance
by sealing history into immutable blocks and keeping expensive patch reasoning bounded to active work.

## Project Goals

Prikk is designed to be:

- easy to use for ordinary local development workflows;
- safe and secure by default, with role-bound signatures and fail-closed validation;
- resilient against corruption, interrupted operations, and lost mutable pointers;
- flexible enough for local, peer, and future hosted workflows;
- fast for long-lived repositories by separating active patch reasoning from sealed block history;
- explainable when patch reasoning cannot prove a safe result.

## Current Status

Latest released implementation: **0.17.7**, adding the concurrency and locking reference.

Next increment candidates are tracked in `ROADMAP.md`.

This is an early implementation suitable for architecture review, experimentation, and contribution.
Do not use Prikk as the sole store for important project history yet. The repository format and command
surface are still evolving, and future releases may require migration.
See the [release, versioning, and compatibility reference](./docs/src/reference/release-compatibility.md)
for the pre-1.0 compatibility and official-release boundary.

The local core can initialize a repository, author signed patches, seal them into blocks, inspect
history, verify integrity, diagnose common repository issues, perform safe checkout planning and
materialization for the supported subset, and display read-only merge evidence and merge plans for
explicit sealed candidates.

## Good Fit

Prikk may be a good match if you are:

- evaluating next-generation VCS architecture;
- interested in patch theory, commutation, conflict evidence, or signed history;
- building tools that need verifiable local history and conservative recovery behavior;
- contributing to a Rust implementation of a correctness-sensitive CLI and storage system;
- reviewing security, durability, and publication-trust boundaries.

## Not a Good Fit Yet

Prikk is not yet the right tool if you need:

- a production replacement for Git;
- stable repository-format compatibility;
- Git object compatibility or transparent Git interoperability;
- hosted forge workflows, remotes, or sync;
- complete branch management, tags, semantic merge, or merge execution;
- plugin/audit execution, attestations, or automated publication controls;
- mature key lifecycle features such as revocation, rotation, hardware signing, or thresholds.

## Core Ideas

- **Patch**: an atomic logical change with ordered operations and an AUTHOR signature.
- **Block**: an immutable sealed collection of patches; blocks are the scalability boundary.
- **Ref state**: signed reference state; ref files are pointers, not the root of trust.
- **Ref update**: append-only publication evidence for a ref transition.
- **WAL**: active signed patch envelopes before sealing.
- **Repository layout**: `.prikk/` stores native Prikk objects, refs, active WAL state, and local trust
  data; see the [repository layout reference](./docs/src/reference/repository-layout.md).
- **Concurrency and locking**: local lock files guard active-session and ref publication writes; see the
  [concurrency and locking reference](./docs/src/reference/concurrency-locking.md).
- **Path safety**: repository paths use a conservative validated subset; see the
  [path and worktree safety reference](./docs/src/reference/path-safety.md).
- **Attestation**: future audit/policy evidence targeting blocks without defining block identity.

## Quick Start

```sh
cargo build -p prikk
export PRIKK="$PWD/target/debug/prikk"

$PRIKK init ./sample-repo

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
```

For a fresh repository, the first `commit` authors a genesis patch set and the first `seal` publishes a
Root block on `heads/main`. The current key-input mechanism is intentionally minimal: seeds are passed
through environment variables for local experimentation, not as a complete key-management system. The
sample values above are public examples and must never be used for real signing. See the
[security and signing setup guide](./docs/src/guide/security-setup.md) for the current setup boundary.

## Useful Commands

```text
prikk init [path]
prikk trust maintainer add --key-id ID --public-key HEX
prikk commit [--ref heads/<branch>] -m <message>
prikk seal --allow-no-audit [--ref heads/<branch>]
prikk status
prikk log [path] [--limit N] [--ref REF]
prikk checkout --plan-only [path] [--ref REF]
prikk checkout --snapshot-plan [path] [--ref REF]
prikk checkout --snapshot-materialize [path] [--ref REF]
prikk checkout --patch-plan [path] [--ref REF]
prikk checkout --patch-materialize [path] [--ref REF]
prikk checkout --patch-delete-plan [path] [--ref REF]
prikk checkout --patch-materialize-delete [path] [--ref REF]
prikk merge-evidence --baseline-block ID (--left-block ID|--left-ref REF) (--right-block ID|--right-ref REF) [path]
prikk merge-plan --baseline-block ID (--left-block ID|--left-ref REF) (--right-block ID|--right-ref REF) [path]
prikk inverse-plan [path] [--ref REF]
prikk rollback-preview [path] [--ref REF]
prikk rollback-draft --append-inverse [path] [--ref REF] -m <message>
prikk rollback-draft-verify [path] [--ref REF]
prikk worktree-status [path] [--ref REF]
prikk verify [path]
prikk doctor [path]
prikk doctor [path] --repair-wal-tail
```

## Project Structure

- `crates/` — Rust workspace crates for the CLI, object model, crypto, repository store, replay
  semantics, hash primitives, and shared errors.
- `docs/` — mdBook documentation.
- `release/` — release-policy schemas and review fixtures; root `release-signers.toml` is the fail-closed
  official signer allowlist.
- `rfcs/` — design records and lifecycle state. `rfcs/done/000-rfc-lifecycle-policy.md` defines how
  `proposed/`, `accepted/`, `done/`, `archive/`, and `handoffs/` are used.
- `ROADMAP.md` — current release and upcoming theme summary.
- `CHANGELOG.md` — released changes.

## Development Gates

Before proposing changes, run the relevant subset of:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
```

In restricted environments where the default temporary directory is read-only, use a workspace-local
temporary directory for integration tests:

```sh
mkdir -p target/tmp
TMPDIR="$PWD/target/tmp" cargo test --workspace --locked
```

## More Detail

The roadmap, RFCs, and mdBook docs are the best entry points for design details:

- [Documentation](https://nabbisen.github.io/prikk/) 
- [ROADMAP.md](./ROADMAP.md)
- [rfcs/README.md](./rfcs/README.md)
- [Current data model](./docs/src/reference/data-model.md)
- [Current trust and threat model](./docs/src/reference/trust-threat-model.md)
- [Security and signing setup](./docs/src/guide/security-setup.md)
- [Current patch algebra and merge evidence concepts](./docs/src/reference/patch-algebra.md)
- [docs/src](./docs/src)
