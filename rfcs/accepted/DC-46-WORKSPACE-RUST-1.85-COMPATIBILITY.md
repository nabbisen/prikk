# DC-46 - Workspace Rust 1.85 Compatibility

**Status:** Implementation accepted after architect implementation review v1 on 2026-07-21; owner
commit and post-commit checkout/archive evidence pending
**Milestone:** M2
**Release target:** Before the 0.19.0 release candidate
**Trigger:** Satisfied by architect acceptance of the DC-45 Rust policy-command cutover on 2026-07-21

## Decision Summary

Preserve the declared Rust 1.85 minimum. Repair four incompatible Edition 2024 let-chain expressions
in `prikk-store`, add a pinned Rust 1.85 CI job, and keep the current dependency graph and lockfile
unchanged. Rust 1.85 compile and test gates become release-blocking; strict Clippy remains a current-
stable gate because Clippy's version-specific lint set is not part of the language compatibility
contract.

Design acceptance authorizes only the bounded implementation and evidence sequence defined here.

## Problem And Reproduced Baseline

The root workspace declares:

```toml
[workspace.package]
edition = "2024"
rust-version = "1.85"
```

All seven product crates and the unpublished `prikk-release-policy` tool inherit that declaration.
Rust 1.85 can resolve the locked package graph, and the release-policy tool independently passes its
Rust 1.85 tests, but the complete workspace does not compile as declared.

At baseline commit `e4e45f0e2a8611d6253a8740584d14c034645c11`, this exact command was observed:

```text
cargo +1.85.0 test --workspace --locked
```

It failed with four `E0658` diagnostics for let expressions in condition-chain position:

1. `crates/prikk-store/src/fsutil/anchored/failpoints.rs` - one `if let` plus Boolean guard;
2. `crates/prikk-store/src/verify/trust.rs` - one `if let` plus one `let Err` chain; and
3. `crates/prikk-store/src/wal.rs` - one `if let` plus Boolean guard.

A horizontal syntax scan found no other let-chain site under `crates/` or `tools/`. The failure occurs
in product source before tests complete; it is not a dependency `rust-version` rejection, lockfile
resolution failure, DC-45 tool regression, or product-format defect.

## Compatibility Authority

`workspace.package.rust-version` in the root `Cargo.toml` is the sole source declaration of the minimum
supported Rust version. Every workspace package must continue to use `rust-version.workspace = true`.
The exact minimum toolchain is `1.85.0`; a moving `stable` channel, an unqualified `1.85`, and the
developer's default toolchain are not substitutes for minimum-version evidence.

Compatibility has two independent surfaces:

- **Language/MSRV compatibility:** locked workspace check, build, and tests on Rust 1.85.0.
- **Current quality:** formatting, strict Clippy, check, build, and tests on the current stable toolchain.

Passing one surface does not imply the other. The release-policy command remains the separately
accepted Rust command and is not redefined by DC-46.

## Selected Repair

The implementation may make semantics-preserving control-flow rewrites only at the three identified
files:

- use a guarded `match` or equivalent Rust 1.85 expression for failpoint selection;
- convert optional trust-policy verification into one Rust 1.85-compatible condition without changing
  issue creation, ordering, or early-return behavior; and
- use a guarded `match` or equivalent for duplicate-WAL-envelope detection while preserving the empty
  durability append and returned sequence.

The rewrites must preserve these observable contracts:

- a failpoint decrements only when its selected point matches, fires at the same count, and clears the
  pending selection at the same time;
- a missing policy, invalid policy, accepted envelope, and rejected envelope produce the same checked-
  record count and issue list in the same order; and
- duplicate WAL append remains idempotent, performs the required empty append/sync path, returns the
  existing sequence, and does not create another record.

No public API, serialized byte, object identity, repository format, error text, diagnostic category, or
filesystem durability ordering may change. Existing focused tests and the full workspace suite remain
the broad behavioral oracle. Architect design review v1 demonstrated that publication-trust issue
generation is not pinned directly, so implementation must add `crates/prikk-store/src/verify/tests/trust.rs`
and declare it from `crates/prikk-store/src/verify/tests.rs`.

The focused tests must exercise the production verifier path and assert:

1. missing and malformed policy cases each add exactly one issue with code
   `PRIKK-TRUST-POLICY-INVALID`, continue incrementing the checked-record count across at least two
   envelopes, return early on the first load failure, and never duplicate the policy issue;
2. a trusted publication envelope increments the checked-record count without adding an issue;
3. an untrusted publication envelope increments the count and adds exactly
   `PRIKK-TRUST-PUBLICATION-UNTRUSTED`; and
4. mixed trusted/untrusted records preserve deterministic encounter order in the issue vector.

Tests may use `verify_repository` or the private `PublicationTrustVerifier`, but must call production
policy loading and envelope verification rather than reproduce the rewritten condition in test logic.
No broader trust refactor or test reorganization is authorized.

## Dependency And Lockfile Boundary

The selected repair authorizes no dependency, feature, workspace membership, package metadata, or
`Cargo.lock` change. The accepted baseline lockfile SHA-256 is:

`0cd51cbdc98210bc745dd6a7190fbcde30b35dfea4d1cd66b7f0b8459527c616`

Implementation evidence must compare this identity before and after every MSRV/current-toolchain gate.
`cargo metadata --locked --no-deps --format-version 1` must continue to report eight workspace members,
the seven product default members, Rust 1.85 inheritance, and `publish = false` for the internal tool.

If an implementation attempt discovers a dependency-level MSRV failure, it must stop. It may not run
an unconstrained update, pin or downgrade a crate, alter features, or raise the minimum under this
design. Any such remedy requires an amended RFC with dependency/license/advisory/transitive-impact
evidence and a new architect design ruling.

## CI Contract

`.github/workflows/ci.yml` must retain the current stable job and add a separately named MSRV job using
exact toolchain `1.85.0`. The MSRV job runs, with `--locked`:

```text
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
cargo build --workspace --locked
```

The stable job remains responsible for formatting and warning-denied workspace Clippy. Every stable and
MSRV CI command that resolves, checks, builds, lints, or tests the workspace must use `--locked`.
Specifically, existing stable Clippy becomes
`cargo clippy --workspace --all-targets --locked -- -D warnings`, and existing stable tests become
`cargo test --workspace --locked`. `cargo fmt --all -- --check` is the sole Cargo-frontend exception:
rustfmt does not resolve dependencies and that command does not accept `--locked`. CI labels and failure
output must distinguish `stable` from `msrv-1.85.0`; a stable success cannot mask an MSRV failure.

Architect command-grammar amendment QA v1 authorizes the five unique exact vectors above in the
DC-45 governed-procedure classifier. The classifier applies repository-wide to governed shell and YAML
under `.github`, `scripts`, and `release`; this is not a path-specific CI exception. Existing assignment,
`env`, and `command` prefix normalization remains unchanged. The implementation must remain
default-closed and add no toolchain-prefix, dynamic-argument, opaque-shell, or project-local-wrapper
production. These ordinary commands must emit no policy or publication invocation.

Rust 1.85 Clippy is deliberately not a required gate. A disposable repair probe showed that Rust 1.85
check and the full test suite pass after the three control-flow rewrites, while Rust 1.85 Clippy rejects
unrelated accepted source for version-specific lints including duplicated attributes, needless
lifetimes, and format collection. Requiring both old and current Clippy warning sets would turn lint
drift into an undocumented source-compatibility contract. Current stable Clippy remains strict.

## Implementation Scope

The initial implementation candidate is limited to:

- `crates/prikk-store/src/fsutil/anchored/failpoints.rs`;
- `crates/prikk-store/src/verify/trust.rs`;
- `crates/prikk-store/src/wal.rs`;
- `crates/prikk-store/src/verify/tests.rs` only to declare the focused module;
- `crates/prikk-store/src/verify/tests/trust.rs` for the required production-path regressions;
- `.github/workflows/ci.yml`;
- `tools/release-policy/src/command_scan/procedure.rs` only for the five exact ordinary-Cargo vectors;
- existing `tools/release-policy/src/command_scan` test modules for positive and adversarial grammar
  coverage;
- public contributor/release compatibility documentation that states the exact MSRV gate; and
- RFC, roadmap, milestone, and implementation-status bookkeeping.

It must not change `Cargo.toml`, `Cargo.lock`, package manifests, the release-policy command inventory,
semantic policy evaluators, `command_scan/prefix.rs`, policy/publication invocation recognition,
publication grammar, oracle files, signer authority, release state, or product behavior outside the
three compatibility rewrites. Test support changes outside the authorized verification and command-scan
test paths require design rereview.

## Required Implementation Evidence

### Rust 1.85 language and package evidence

Run with a writable repository-local `TMPDIR` where the execution environment requires it:

```text
cargo +1.85.0 metadata --locked --no-deps --format-version 1
cargo +1.85.0 check --workspace --all-targets --locked
cargo +1.85.0 test --workspace --locked
cargo +1.85.0 build --workspace --locked
```

For each product package in the accepted publication order (`prikk-error`, `prikk-hash`, `prikk-crypto`,
`prikk-object`, `prikk-replay`, `prikk-store`, `prikk`), run:

```text
cargo +1.85.0 package --locked --allow-dirty --list -p <package>
```

Each listing must exclude `tools/release-policy/` and `release/oracle/`. Package listing proves the
publish payload boundary; the all-target workspace check/test/build proves source compatibility. The
unpublished tool must never enter the publication list.

### Current-toolchain and authority evidence

```text
cargo fmt --all -- --check
RUSTFLAGS="-D warnings" cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
cargo run --locked -p prikk-release-policy -- boundary-check --format json
cargo run --locked -p prikk-release-policy -- reference-check --format json
mdbook build docs
cargo audit --no-fetch
git diff --check
```

The inventory-selected authoritative Rust release-policy check must also pass all frozen cases;
boundary/reference reports must be valid with no errors. Git-visible state under
`--untracked-files=all`, `Cargo.lock`, the oracle manifest, and `release-signers.toml` must be byte-
identical before and after the gates.

## Review, Commit, And Archive Sequence

1. Submit the uncommitted implementation plus all evidence above for architect implementation review.
2. After acceptance, the project owner creates one isolated implementation/status commit.
3. Reproduce the commit from a clean no-hardlink checkout and deterministic extracted source archive.
4. Run the exact Rust 1.85 check/test/build and current authority/boundary/reference gates in both
   committed environments, and verify product package payload equivalence.
5. Submit post-commit evidence for architect acceptance before claiming DC-46 complete or opening the
   0.19.0 release candidate.

The source archive must contain the CI workflow and all repaired source, and all seven product `.crate`
payloads must remain free of release-policy/oracle internals.

## Rollback And Failure Policy

The implementation is source- and CI-only and creates no format migration. Its rollback anchor is the
accepted pre-implementation commit. Reversing only the DC-46 implementation diff must restore the
previous current-toolchain behavior and the known Rust 1.85 `E0658` failure; this is an emergency
rollback state, not a releasable compatibility state.

Any regression in repository behavior, current stable gates, package boundaries, policy authority,
lockfile identity, or signer identity blocks acceptance. If the exact Rust 1.85 contract cannot be met,
0.19.0 remains blocked and the team returns to design review. A minimum-version increase is never an
implicit fallback.

An amendment proposing a higher minimum must name the exact toolchain, explain why source compatibility
is not reasonably recoverable, update root authority/CI/public docs and any release-policy boundary
expectations, assess downstream users and package metadata, and receive architect acceptance before
implementation.

## Non-goals

- No minimum-version increase or rolling-MSRV policy.
- No dependency refresh, lockfile regeneration, feature change, or package-graph refactor.
- No product behavior, storage format, cryptographic, trust, WAL, or failpoint semantics change.
- No semantic release-policy evaluator, command authority, inventory, oracle, signer, release-state,
  tag, publication, or release action. The architect-approved exact ordinary-command classifier
  amendment is the sole release-policy tooling exception.
- No Python retirement or DC-45 stability claim.
- No broad syntax downgrade beyond the four reproduced let-chain expressions.
- No trust implementation/refactor beyond the one compatibility rewrite and focused regression tests.

## Completion Gate

DC-46 is complete only when the selected Rust 1.85 repair and pinned CI gate are committed; deterministic
checkout/archive evidence passes Rust 1.85 and current-toolchain contracts; lockfile, package, policy,
oracle, and signer identities remain within the boundaries above; and architect post-commit review
accepts the evidence. Only then is the Rust-version blocker closed for the 0.19.0 release candidate.
