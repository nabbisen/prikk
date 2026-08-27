# Prikk

![Status](https://img.shields.io/badge/status-early--implementation-orange)
[![CI](https://github.com/nabbisen/prikk/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/nabbisen/prikk/actions/workflows/ci.yml)
[![license](https://img.shields.io/crates/l/prikk.svg)](LICENSE)
[![crates.io](https://img.shields.io/crates/v/prikk.svg?label=prikk)](https://crates.io/crates/prikk)
[![documentation](https://img.shields.io/badge/docs-github_pages-brightgreen)](https://nabbisen.github.io/prikk/)
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

Latest released implementation: **0.27.0**. Windows became a mutating platform in 0.21.0: Prikk now authors, commits, and checks out on Linux, macOS, and Windows, and CI requires a repository authored on Linux, mutated on Windows, and verified back on Linux to produce byte-identical object ids — so the claim that anyone can verify anyone's history is tested across platforms rather than assumed.

Next increment candidates are tracked in `ROADMAP.md`.

This is an early implementation suitable for architecture review, experimentation, and contribution.
Do not use Prikk as the sole store for important project history yet. The repository format and command
surface are still evolving, and future releases may require migration.
See the [release, versioning, and compatibility reference](./docs/src/reference/release-compatibility.md)
for the pre-1.0 compatibility and official-release boundary.

The local core can initialize a repository, author signed patches, seal them into blocks, inspect
history, verify integrity, diagnose common repository issues, perform safe checkout planning and
materialization for the supported subset, display merge evidence and merge plans for explicit sealed
candidates, and **execute a merge** when the two sides are proven confluent — refusing cleanly, with no
object, WAL, or ref write, when they are not.

Known limits worth stating up front: merge-base discovery is manual; conflicts are detected and refused
but never resolved; sync exists between repositories, but **prikk does not move the bytes itself** —
confidentiality is the user's channel's property, not prikk's — negotiation is branch-scoped (tags
travel and are adopted separately, under the receiver's own key), and there is no discovery or
remote-tracking; `verify` cost is linear in history length; `verify` checks author signatures
repository-wide, but only as trust-on-first-use continuity — it proves the same author signed as last
time, not who that author is on first contact; and `verify` checks a locally-published tag's
maintainer signature against this repository's own trust policy, but a received, not-yet-adopted tag
is deliberately exempt — its signature is the sender's, under a key this repository has not adopted.

**Mutation runs on Linux, macOS, and Windows** as of 0.21.0. Windows has narrower guarantees in two
named places — see the [platform support
reference](./docs/src/reference/platform-support.md).

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
- hosted forge workflows, or remotes;
- complete branch management, or semantic merge;
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

## Install

Prebuilt binary, no Rust toolchain required — Linux (`x86_64`/`aarch64`), macOS (`aarch64`), and
Windows (`x86_64`):

```sh
cargo binstall prikk
```

Or download directly from the [release page](https://github.com/nabbisen/prikk/releases), verify the
attached `.sha256` checksum, and extract. Every target archive contains the `prikk` binary, `LICENSE`,
and a `.build-info.txt` recording the exact toolchain and command used to build it — reproducible from
the tag with `cargo build -p prikk --release --target <triple> --locked`.

**Release authority.** Prebuilt binaries carry no more signer authority than the source tarball already
carries none of — `release-signers.toml` is empty and fail-closed, so no release, including its
attached binaries, passes the DC-35 signer-authority audit yet. A checksum proves integrity of
transport, not authority of origin; see the [release-compatibility
reference](./docs/src/reference/release-compatibility.md).

From crates.io, requires a Rust toolchain:

```sh
cargo install prikk
```

**Repository *mutation* runs on Linux, macOS, and Windows** (as of 0.21.0; Windows carries two named
narrower guarantees — see the platform support reference below). **Read-only commands build and run on
all three**
(`verify`, `log`, `status`, `doctor`, `checkout --plan-only`/`--snapshot-plan`/`--patch-plan`/
`--patch-delete-plan`, `merge-evidence`, `merge-plan`, `inverse-plan`, `rollback-preview`,
`rollback-draft-verify`, `branch [list]`, `tag [list]` — the full, durable list, including one
capability-gap caveat, is in the [platform support
reference](./docs/src/reference/platform-support.md)). This closes a defect fixed by DC-71:
`prikk-store` previously failed to compile at all off Linux due to inconsistently
`#[cfg(target_os = "linux")]`-gated imports; CI now builds and actually *runs* the read-only command
set against a real repository on GitHub's `windows-latest` and `macos-latest` runners on every
change, so this cannot silently rot again.

To build from a clone instead — the path to use when working on prikk itself:

```sh
cargo build -p prikk && export PATH="$PWD/target/debug:$PATH"
```

## Quick Start

```sh
mkdir -p ./sample-repo && cd ./sample-repo
prikk init .

export PRIKK_AUTHOR_KEY_ID="dev-author"
export PRIKK_AUTHOR_SEED="00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff"
export PRIKK_MAINTAINER_KEY_ID="dev-maintainer"
export PRIKK_MAINTAINER_SEED="111122223333444455556666777788889999aaaabbbbccccddddeeeeffff0000"

prikk trust maintainer add \
  --key-id "$PRIKK_MAINTAINER_KEY_ID" \
  --public-key "a00899dfd3357aee69729405913f9324dfc033cec04a2215239eda64ae6d9d91"

echo "hello prikk" > readme.txt
prikk commit -m "genesis"
prikk seal --allow-no-audit

prikk log
prikk verify
prikk doctor
```

### Ref names are fully qualified

`branch create`, `branch close`, and `tag create` take a **fully-qualified** ref — `heads/topic`, not
`topic`; `tags/v1`, not `v1`. A bare name is rejected: `invalid name: ref topic is not a local branch ref;
expected heads/<name>`. There is no current-branch pointer and no `branch switch`, so every command that
targets a ref resolves `--ref` explicitly.

### Committing more than once before sealing

`commit` may run repeatedly without an intervening `seal`; the active session queues the patches and
`seal` batches them into one block. `status` reports the queue — `queued patches: 2 targeting heads/main`.
Committing and sealing one-for-one still works exactly as before; nothing forces accumulation.

Two environment variables bound the queue, both fail-closed on a malformed value:

- `PRIKK_ACTIVE_PATCH_WARN` — warn at this many queued patches (default 800)
- `PRIKK_ACTIVE_PATCH_LIMIT` — refuse further commits at this many (default 1000)

The limit is checked before any write, so a refused commit leaves no partial state.

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
prikk merge --allow-no-audit --baseline-block ID --into REF --from REF [path]
prikk inverse-plan [path] [--ref REF]
prikk rollback-preview [path] [--ref REF]
prikk rollback-draft --append-inverse [path] [--ref REF] -m <message>
prikk rollback-draft-verify [path] [--ref REF]

prikk branch [list] [--all]
prikk branch create heads/<name> [--from REF]
prikk branch close heads/<name>
prikk tag [list]
prikk tag create tags/<name> --target <ref|block> [-m <message>]
prikk bundle export --ref REF --output <file>
prikk bundle import --input <file>
prikk sync summary --output <file>
prikk sync compare --summary <file>
prikk sync have <ref> --output <file>
prikk sync build <ref> --have <file> --output <file>
prikk sync accept <file> [--claims-out <file>]
prikk sync pending
prikk sync seal <ref> --claim <id>
prikk sync tags
prikk sync adopt-tag <name>
prikk worktree-status [path] [--ref REF]
prikk verify [path]
prikk doctor [path]
prikk doctor [path] --repair-wal-tail
prikk unlock
prikk unlock --lock <path> [--yes]
prikk compact --pointer-index|--received-index|--trust-policy|--all [--plan-only]
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
