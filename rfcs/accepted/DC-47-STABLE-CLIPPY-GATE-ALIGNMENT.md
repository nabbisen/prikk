# DC-47 - Stable Clippy Gate Alignment

**Status.** Accepted after architect design review v1 and legacy-vector test-contract QA v1 on
2026-07-21; architect implementation review v1 accepted the candidate on 2026-07-21; owner commit and
post-commit evidence review pending.
**Target milestone.** M2 - before the 0.19.0 release candidate.
**Tracks.** DC-46 implementation-review N1 and post-commit-review N1.
**Touches.** Stable CI Clippy command, governed ordinary-Cargo procedure grammar and tests,
contributor gate documentation, and durable status records.

## Problem

DC-35 defines the applicable release Clippy gate as:

```text
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

DC-46 subsequently made locked resolution mandatory and registered the existing stable CI form:

```text
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The public release reference therefore correctly documents DC-35's stronger gate with `--locked`, but
the DC-45 default-closed governed-procedure classifier rejects that exact command because it recognizes
only the no-`--all-features` form. The mismatch is latent while the command remains in non-strict
Markdown. Moving the documented release command into `.github`, `scripts`, or `release` would fail the
boundary and reference gates.

At baseline commit `24c7d0e087bc171867df22785a37de9404b006ba`, Cargo metadata reports zero declared
features for all eight workspace members. The exact locked all-features Clippy command passes on current
stable. Thus the two commands are behaviorally equivalent today, but they express different future
contracts.

## Decision

Preserve DC-35's stronger release contract. Make this the canonical current-stable Clippy command for
CI and contributor guidance:

```text
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

Add that one exact ordinary-Cargo argument vector to the DC-45 governed-procedure classifier. Retain
the previously accepted no-all-features locked and historical unlocked vectors for additive grammar
stability; repository search found no second current governed consumer of the locked legacy form. The
classifier remains a structural allowlist, not command authority: the new vector must emit neither
policy nor publication `Invocation`.

Update stable CI, `README.md`, and `docs/src/contributing/development.md` to the canonical form.
`docs/src/reference/release-compatibility.md` already contains the selected locked command and must
remain unchanged except for a cross-reference if review requires one. Completed DC-35 and DC-46 records
remain historical authority and are not rewritten to pretend they selected the same intermediate
command.

The exact Rust 1.85.0 job remains unchanged and has no Clippy step. `--all-features` is a current-stable
quality contract, not an MSRV-language contract.

## Policy Boundary

The classifier change applies repository-wide to governed shell and YAML under `.github`, `scripts`,
and `release`. It is not path-specific to CI. Existing assignment, `env`, and `command` prefix
normalization remains unchanged.

Implementation may change only:

- `.github/workflows/ci.yml`;
- `README.md`;
- `docs/src/contributing/development.md`;
- `tools/release-policy/src/command_scan/procedure.rs`;
- existing command-scan test modules; and
- this RFC plus roadmap, milestone, RFC-index, and implementation-status bookkeeping.

It must not change `command_scan/prefix.rs`, YAML extraction, policy or publication invocation
recognition, reference authority, command inventories, semantic evaluators, oracle data, Python files,
dependencies, manifests, `Cargo.lock`, product source, signer authority, release state, or package
boundaries.

## Required Tests

Add table-driven tests proving:

1. the exact canonical vector passes strict shell and YAML scanning;
2. it emits no policy or publication invocation in either mode;
3. existing assignment, `env`, and `command` prefixes retain their reviewed behavior;
4. the canonical vector without `--all-features` remains the accepted locked legacy positive case;
5. missing `--locked` while retaining `--all-features`, duplicate flags, extra flags, reordered
   `--all-features`/`--locked`, and placement of either flag after the `--` separator fail closed;
6. toolchain-prefixed, dynamic-command, dynamic-argument, opaque-shell, and project-local-wrapper forms
   fail closed; and
7. all previously accepted ordinary-Cargo vectors remain accepted.

The full release-policy test suite, authoritative 154-case Rust policy check, and valid empty-error
boundary/reference reports are mandatory implementation evidence.

Because the classifier retains legacy forms, implementation review must directly verify that the
stable CI line and all three public command surfaces (`README.md`, contributor development guidance,
and release compatibility guidance) literally match the canonical command shown in the Decision
section. Boundary/reference success alone does not prove canonical selection.

## Implementation And Evidence Sequence

1. Obtain architect design acceptance and move DC-47 to `rfcs/accepted/`.
2. Prepare the bounded uncommitted implementation and run:

   ```text
   cargo fmt --all -- --check
   RUSTFLAGS="-D warnings" cargo check --workspace --all-targets --locked
   canonical stable Clippy command from the Decision section
   cargo test --workspace --locked
   cargo build --workspace --locked
   cargo +1.85.0 check --workspace --all-targets --locked
   cargo +1.85.0 test --workspace --locked
   cargo +1.85.0 build --workspace --locked
   inventory-selected authoritative Rust release-policy check
   cargo run --locked -p prikk-release-policy -- boundary-check --format json
   cargo run --locked -p prikk-release-policy -- reference-check --format json
   mdbook build docs
   cargo audit --no-fetch
   git diff --check
   ```

3. Obtain separate architect implementation acceptance.
4. The project owner creates one isolated implementation/status commit.
5. Reproduce the commit from a clean no-hardlink checkout and deterministic extracted archive. Run the
   canonical stable Clippy command and these authority gates in both:

   ```text
   cargo test --locked -p prikk-release-policy
   inventory-selected authoritative Rust release-policy check
   cargo run --locked -p prikk-release-policy -- boundary-check --format json
   cargo run --locked -p prikk-release-policy -- reference-check --format json
   ```

   Bind both environments to the immutable commit/tree, verify clean tracked state, compare product
   package listings after normalizing only Cargo's expected `.cargo_vcs_info.json`, verify frozen
   identities, and obtain separate post-commit evidence acceptance.

Architect implementation review v1 accepted the bounded candidate on 2026-07-21. The owner commit and
the post-commit evidence sequence remain pending; DC-47 is not yet complete.

The frozen identities include root `Cargo.toml`, every workspace package manifest, `Cargo.lock`, both
command inventories, the oracle manifest, and `release-signers.toml`.

## Required Follow-Up

DC-48 will retire the historical unlocked and locked no-all-features Clippy productions after DC-47's
implementation commit and post-commit evidence are accepted. At that trigger, stable CI is the sole
current governed Clippy consumer and uses the canonical vector, so the subtraction can restore
classifier-enforced canonical selection. DC-48 requires its own design, implementation, and policy
review; it is targeted before the 0.19.0 release candidate and is not part of DC-47 scope.

## Failure And Rollback

Any failure of the canonical Clippy command, scanner adversarial matrix, authority gate, Rust 1.85
contract, package boundary, or frozen identity blocks acceptance. Do not weaken DC-35 documentation,
add generic Cargo grammar, or modify semantic authority to make a gate pass.

Rollback is the isolated parent commit. Reverting DC-47 restores the accepted no-all-features CI vector
and the known documentation/classifier mismatch; that state remains buildable but cannot open the
0.19.0 release candidate.

## Non-Goals

- No new Cargo feature, feature policy, feature-combination matrix, or dependency change.
- No Rust 1.85 Clippy requirement or MSRV change.
- No generic Cargo allowlist, configurable command inventory, parser refactor, or wrapper expansion.
- No release-policy authority transition, Python retirement, signer change, release candidate, tag,
  publication, release, production-readiness, or public-preview claim.

## Completion Gate

DC-47 is complete only when the canonical all-features stable Clippy command is consistent across CI,
public contributor/release guidance, and the governed classifier; implementation and post-commit
reviews accept the exact-vector grammar and evidence; all frozen identities and product boundaries are
preserved; and durable status records the immutable completion commit. Completion removes only this
pre-0.19.0 release-candidate blocker.
