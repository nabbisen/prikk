<h1><img src="https://raw.githubusercontent.com/prikk-vcs/prikk/main/assets/logo/prikk-header-520.png" alt="Prikk" width="360"></h1>

[![license](https://img.shields.io/crates/l/prikk.svg)](LICENSE)
[![documentation](https://img.shields.io/badge/docs-github_pages-brightgreen)](https://prikk-vcs.github.io/prikk/)
[![CI](https://github.com/prikk-vcs/prikk/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/prikk-vcs/prikk/actions/workflows/ci.yml)

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

## Crates

`prikk` is the command-line tool. The others are the libraries it is built from, published so the
CLI can be built from crates.io — **their APIs may change without notice before 1.0.**

| Crate | Purpose | Version | Docs | Dependencies |
|---|---|---|---|---|
| [`prikk`](https://crates.io/crates/prikk) | the command-line tool | [![crates.io](https://img.shields.io/crates/v/prikk.svg?label=%20)](https://crates.io/crates/prikk) | [![documentation](https://img.shields.io/badge/docs-github_pages-brightgreen)](https://prikk-vcs.github.io/prikk/) | [![Dependency Status](https://deps.rs/crate/prikk/latest/status.svg)](https://deps.rs/crate/prikk) |
| [`prikk-store`](https://crates.io/crates/prikk-store) | repository storage engine — layout, object storage, WAL durability, verification, patch replay | [![crates.io](https://img.shields.io/crates/v/prikk-store.svg?label=%20)](https://crates.io/crates/prikk-store) | [![docs.rs](https://img.shields.io/docsrs/prikk-store?version=latest&label=%20)](https://docs.rs/prikk-store) | [![Dependency Status](https://deps.rs/crate/prikk-store/latest/status.svg)](https://deps.rs/crate/prikk-store) |
| [`prikk-object`](https://crates.io/crates/prikk-object) | object identity, canonical encoding, and payload types | [![crates.io](https://img.shields.io/crates/v/prikk-object.svg?label=%20)](https://crates.io/crates/prikk-object) | [![docs.rs](https://img.shields.io/docsrs/prikk-object?version=latest&label=%20)](https://docs.rs/prikk-object) | [![Dependency Status](https://deps.rs/crate/prikk-object/latest/status.svg)](https://deps.rs/crate/prikk-object) |
| [`prikk-replay`](https://crates.io/crates/prikk-replay) | replay and lifecycle semantics | [![crates.io](https://img.shields.io/crates/v/prikk-replay.svg?label=%20)](https://crates.io/crates/prikk-replay) | [![docs.rs](https://img.shields.io/docsrs/prikk-replay?version=latest&label=%20)](https://docs.rs/prikk-replay) | [![Dependency Status](https://deps.rs/crate/prikk-replay/latest/status.svg)](https://deps.rs/crate/prikk-replay) |
| [`prikk-crypto`](https://crates.io/crates/prikk-crypto) | Ed25519 signing and verification | [![crates.io](https://img.shields.io/crates/v/prikk-crypto.svg?label=%20)](https://crates.io/crates/prikk-crypto) | [![docs.rs](https://img.shields.io/docsrs/prikk-crypto?version=latest&label=%20)](https://docs.rs/prikk-crypto) | [![Dependency Status](https://deps.rs/crate/prikk-crypto/latest/status.svg)](https://deps.rs/crate/prikk-crypto) |
| [`prikk-hash`](https://crates.io/crates/prikk-hash) | SHA-256 primitives | [![crates.io](https://img.shields.io/crates/v/prikk-hash.svg?label=%20)](https://crates.io/crates/prikk-hash) | [![docs.rs](https://img.shields.io/docsrs/prikk-hash?version=latest&label=%20)](https://docs.rs/prikk-hash) | [![Dependency Status](https://deps.rs/crate/prikk-hash/latest/status.svg)](https://deps.rs/crate/prikk-hash) |
| [`prikk-error`](https://crates.io/crates/prikk-error) | shared error taxonomy | [![crates.io](https://img.shields.io/crates/v/prikk-error.svg?label=%20)](https://crates.io/crates/prikk-error) | [![docs.rs](https://img.shields.io/docsrs/prikk-error?version=latest&label=%20)](https://docs.rs/prikk-error) | [![Dependency Status](https://deps.rs/crate/prikk-error/latest/status.svg)](https://deps.rs/crate/prikk-error) |
| [`prikk-ffi`](https://crates.io/crates/prikk-ffi) | Windows filesystem-identity FFI bindings | [![crates.io](https://img.shields.io/crates/v/prikk-ffi.svg?label=%20)](https://crates.io/crates/prikk-ffi) | [![docs.rs](https://img.shields.io/docsrs/prikk-ffi?version=latest&label=%20)](https://docs.rs/prikk-ffi) | [![Dependency Status](https://deps.rs/crate/prikk-ffi/latest/status.svg)](https://deps.rs/crate/prikk-ffi) |

## Current Status

Latest released implementation: **0.27.1**. Windows became a mutating platform in 0.21.0: Prikk now authors, commits, and checks out on Linux, macOS, and Windows, and CI requires a repository authored on Linux, mutated on Windows, and verified back on Linux to produce byte-identical object ids — so the claim that anyone can verify anyone's history is tested across platforms rather than assumed.

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

The fastest path — no Rust toolchain, no `cargo binstall` — on Linux (`x86_64`/`aarch64`) and macOS
(Apple Silicon):

```sh
curl -fsSL https://github.com/prikk-vcs/prikk/releases/latest/download/install.sh | sh
```

Downloads the release page's own prebuilt archive for your platform, verifies its checksum, and
installs to `~/.local/bin` (override with `PRIKK_INSTALL_DIR`, or `--prefix`) — refusing to install
anything if the checksum does not match. **A checksum proves integrity of transport, not authority of
origin** — see **Release authority** below; the script states this itself when it finishes. Pin a
version with `sh install.sh --version X.Y.Z` (or `PRIKK_INSTALL_VERSION`); prefer to read the script
before running it? `curl -fsSL .../install.sh -o install.sh`, inspect it, then `sh install.sh`. To
remove what it installed: `curl -fsSL .../uninstall.sh | sh` (same base URL). Windows is not
supported by this script yet — use `cargo install prikk` below, or download the `.zip` from the
release page by hand. See the [install guide](./docs/src/guide/install.md#the-shell-installer) for
what it writes to disk, install-location details, and the uninstall guarantee.

Prebuilt binary, no Rust toolchain required — Linux (`x86_64`/`aarch64`), macOS (`aarch64`), and
Windows (`x86_64`):

```sh
cargo binstall prikk
```

Or download directly from the [release page](https://github.com/prikk-vcs/prikk/releases), verify the
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
`rollback-draft-verify`, `branch [list]`, `tag [list]`, `worktree-status` — the full, durable list,
including one capability-gap caveat, is in the [platform support
reference](./docs/src/reference/platform-support.md)). This closes a defect fixed by DC-71:
`prikk-store` previously failed to compile at all off Linux due to inconsistently
`#[cfg(target_os = "linux")]`-gated imports; CI now builds and actually *runs* the read-only command
set against a real repository on GitHub's `windows-latest` and `macos-latest` runners on every
change, so this cannot silently rot again.

**On a platform with no prebuilt binary — other Linux architectures, or a BSD — build it yourself.**
Nothing in prikk is gated on CPU architecture, only on the operating system, so any architecture Rust
targets on Linux builds with no reduction in capability. FreeBSD compiles too, but **mutation is
refused at runtime off Linux/macOS/Windows**, so it is read-only there. See the [install
guide](./docs/src/guide/install.md#build-from-source).

To build from a clone — also the path to use when working on prikk itself:

```sh
cargo build -p prikk --release --locked && export PATH="$PWD/target/release:$PATH"
```

## Quick Start

The commands below, with explanation and the two refusals you will actually hit along the way, are
the [Tutorial](./docs/src/guide/tutorial.md) — this block is a copy-pasteable summary of it, not a
second, independent walkthrough; its authority for what each step means and why is the tutorial page.

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

The sample seed and key values above are public examples and must never be used for real signing —
see [Security and Signing Setup](./docs/src/guide/security-setup.md) for the current setup boundary.

Ran into a refusal, or wondering why a step works the way it does? The
[Troubleshooting](./docs/src/guide/troubleshooting.md) and [FAQ](./docs/src/guide/faq.md) pages cover
exactly this walkthrough — including why ref names are fully qualified (`heads/topic`, not `topic`),
why `commit` and `seal` each need their own key, and how commits queue across multiple `commit` calls
before one `seal`.

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
prikk bundle export --ref REF --output <file> [--force]
prikk bundle import --input <file>
prikk bundle verify --input <file>
prikk sync summary --output <file>
prikk sync compare --summary <file>
prikk sync have <ref> --output <file>
prikk sync build <ref> --have <file> --output <file> [--force]
prikk sync accept <file> [--claims-out <file>] [--force]
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

- [Documentation](https://prikk-vcs.github.io/prikk/)
- [ROADMAP.md](./ROADMAP.md)
- [rfcs/README.md](./rfcs/README.md)
- [Current data model](./docs/src/reference/data-model.md)
- [Current trust and threat model](./docs/src/reference/trust-threat-model.md)
- [Security and signing setup](./docs/src/guide/security-setup.md)
- [Current patch algebra and merge evidence concepts](./docs/src/reference/patch-algebra.md)
- [docs/src](./docs/src)
