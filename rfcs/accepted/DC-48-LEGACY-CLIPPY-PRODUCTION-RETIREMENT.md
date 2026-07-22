# DC-48 - Legacy Clippy Production Retirement

**Status.** Architect implementation review v1 accepted the bounded candidate on 2026-07-22; owner
commit and post-commit evidence review pending; DC-48 is not yet complete.
**Target milestone.** M2 - required before the 0.19.0 release candidate.
**Tracks.** DC-47 required follow-up and post-commit-review N1/N2.
**Touches.** Governed ordinary-Cargo procedure grammar and command-scan tests only, plus RFC/status
bookkeeping.

## Problem

DC-47 made this the canonical current-stable Clippy command across stable CI and current public
contributor/release guidance:

```text
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

The DC-45 default-closed governed-procedure classifier currently accepts three exact Clippy argument
productions:

- **A, historical unlocked:**
  `cargo clippy --workspace --all-targets -- -D warnings`;
- **B, locked without all features:**
  `cargo clippy --workspace --all-targets --locked -- -D warnings`; and
- **C, canonical:**
  `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`.

DC-47 retained A and B temporarily to keep its change additive. That exception allowed alignment to
land without silently withdrawing previously accepted grammar, but it also leaves a policy gap:
replacing stable CI's canonical C with weaker A or B would still pass structural boundary/reference
checks. Human literal-line inspection is the sole current detection control.

At baseline commit `72d88a54297650ffab36f1188257320b725c7f59`, repository search of the governed
`.github`, `scripts`, and `release` roots finds one live Clippy command: canonical C in stable CI. No
governed consumer uses A or B. Historical RFCs and handoffs containing older command examples are not
live governed procedures and must remain preserved as historical records.

## Decision

Remove exactly productions A and B from the `"clippy"` branch of
`tools/release-policy/src/command_scan/procedure.rs`. Retain exact production C as the sole accepted
Clippy procedure:

```text
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

This is a repository-wide policy subtraction for governed shell and YAML procedures under `.github`,
`scripts`, and `release`. After implementation, any governed use of A or B must fail closed. Canonical
C remains structural permission only and must emit neither a policy nor publication `Invocation`.

Do not rewrite stable CI, current public command surfaces, completed DC-35/DC-46/DC-47 records, or old
handoffs. Current live surfaces already use C; historical records must continue to describe the command
that applied at their own lifecycle point.

## Policy Boundary

Implementation may change only:

- `tools/release-policy/src/command_scan/procedure.rs`;
- `tools/release-policy/src/command_scan/tests.rs`, or an existing command-scan test module if review
  requires a source-size split; and
- this RFC plus roadmap, milestone, RFC-index, and implementation-status bookkeeping.

It must not change stable or Rust 1.85 CI, `README.md`, mdBook content, command prefix parsing, YAML
extraction, policy/publication invocation recognition, reference authority, command inventories,
semantic evaluators, oracle data, Python files, dependencies, manifests, `Cargo.lock`, product source,
signer authority, release state, or package boundaries.

The accepted ordinary-Cargo grammar for `fmt`, `test`, `check`, `build`, and `install` remains
unchanged. No replacement Clippy form, generic Cargo argument grammar, path-specific exception,
configurable allowlist, wrapper, or toolchain prefix is introduced.

## Required Tests

Update table-driven command-scan tests to prove in both strict shell and YAML modes:

1. exact canonical C remains accepted and emits no policy or publication invocation, both bare and
   through assignment, bounded `env`, and bounded `command` prefixes;
2. exact historical A fails with exactly `unclassified-procedure-command` and emits no invocation,
   both bare and through those same three bounded prefix classes;
3. exact locked no-all-features B fails with exactly `unclassified-procedure-command` and emits no
   invocation, both bare and through those same three bounded prefix classes;
4. all existing canonical near misses remain rejected, including missing or duplicate flags, extra or
   reordered flags, flags after `--`, toolchain prefixes, dynamic command/argument forms, opaque
   shells, and project-local wrappers;
5. all non-Clippy ordinary-Cargo productions remain accepted without authority;
6. bounded prefix normalization does not mask A/B retirement or change canonical C; and
7. at least one non-Clippy accepted ordinary-Cargo vector is exercised through assignment, `env`, and
   `command` prefixes so prefix coverage does not depend on the Clippy command family alone.

The existing positive occurrences of A and B must be reclassified, not merely deleted. The combined
YAML workflow fixture that currently uses A must use C or another still-accepted exact vector, and A
must appear in the explicit rejection matrix. B must move from the positive locked-workspace matrix to
the explicit rejection matrix.

The full `prikk-release-policy` test suite, authoritative 154-case Rust policy check, and valid
empty-error boundary/reference reports are mandatory implementation evidence.

## Current-Surface Invariant

Implementation and post-commit review must directly verify that each current governed/public surface
contains exactly one literal canonical C and no A/B residue:

- `.github/workflows/ci.yml`;
- `README.md`;
- `docs/src/contributing/development.md`; and
- `docs/src/reference/release-compatibility.md`.

After A/B retirement, boundary/reference checks enforce canonical selection for governed procedures,
but direct inspection remains required to bind unchanged public guidance and the exact reviewed CI
line. Historical RFCs and handoffs are excluded from this current-surface invariant.

## Implementation And Evidence Sequence

1. Obtain architect design acceptance and move DC-48 to `rfcs/accepted/`.
2. Prepare the bounded uncommitted implementation and run:

   ```text
   cargo fmt --all -- --check
   RUSTFLAGS="-D warnings" cargo check --workspace --all-targets --locked
   cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
   cargo test --workspace --locked
   cargo build --workspace --locked
   cargo +1.85.0 check --workspace --all-targets --locked
   cargo +1.85.0 test --workspace --locked
   cargo +1.85.0 build --workspace --locked
   cargo test --locked -p prikk-release-policy
   inventory-selected authoritative Rust release-policy check
   cargo run --locked -p prikk-release-policy -- boundary-check --format json
   cargo run --locked -p prikk-release-policy -- reference-check --format json
   mdbook build docs
   cargo audit --no-fetch
   git diff --check
   ```

3. Obtain separate architect implementation acceptance.
4. The project owner creates one isolated implementation/status commit.
5. Reproduce the immutable commit from a clean no-hardlink checkout and deterministic extracted source
   archive. Run canonical Clippy and these gates independently in both environments:

   ```text
   cargo test --locked -p prikk-release-policy
   inventory-selected authoritative Rust release-policy check
   cargo run --locked -p prikk-release-policy -- boundary-check --format json
   cargo run --locked -p prikk-release-policy -- reference-check --format json
   ```

   Bind both environments to the commit/tree, verify clean tracked state, verify bare and prefixed A/B
   rejection with the exact error and no invocation, verify bare and prefixed C acceptance with no
   invocation, compare product package listings after normalizing only Cargo's expected
   `.cargo_vcs_info.json`, verify frozen identities, and obtain separate post-commit evidence
   acceptance.

Architect implementation review v1 accepted the bounded candidate on 2026-07-22. The owner commit and
post-commit evidence sequence remain pending; DC-48 is not yet complete.

The frozen identities include root `Cargo.toml`, every workspace package manifest, `Cargo.lock`, both
command inventories, the oracle manifest, and `release-signers.toml`.

## Failure And Rollback

Any acceptance of A or B, rejection of C, authority emission from C, regression in another accepted
ordinary-Cargo production or prefix form, authority-gate failure, current-surface drift, package
boundary change, or frozen-identity change blocks acceptance.

Rollback is the isolated parent commit. Reverting DC-48 restores the temporary A/B compatibility gap
and therefore also restores the pre-0.19.0 release-candidate blocker. Do not weaken canonical CI or
public guidance, add broader grammar, or modify semantic authority to make a gate pass.

## Alternatives Rejected

1. **Keep A/B indefinitely and rely on review.** Rejected because a weaker governed CI command would
   pass automated policy while only manual inspection could detect the regression.
2. **Retire only unlocked A.** Rejected because locked B still omits the accepted all-features quality
   contract and preserves the same downgrade path.
3. **Rewrite historical RFCs and handoffs.** Rejected because those records describe prior accepted
   states and are not live governed consumers.
4. **Make Clippy grammar generic or configurable.** Rejected because exact C is the sole current need
   and broader grammar weakens the default-closed boundary.
5. **Require Clippy on Rust 1.85.0.** Rejected because current-stable lint quality remains separate from
   the minimum language/toolchain contract established by DC-46.

## Non-Goals

- No CI, public documentation, product behavior, Cargo feature, dependency, lockfile, or package change.
- No parser, prefix grammar, YAML extraction, inventory, oracle, Python, or semantic-authority change.
- No Rust 1.85 Clippy requirement or MSRV change.
- No DC-45 migration completion, Python retirement, signer change, release candidate, tag,
  publication, release, production-readiness, or public-preview claim.
- No resolution of DC-39, DC-40, or other M1/M2 program work.

## Completion Gate

DC-48 is complete only when C is the sole accepted Clippy procedure; A and B fail closed in strict
shell and YAML modes; non-Clippy procedure and prefix behavior remains covered; all four current
surfaces remain exactly canonical; implementation and post-commit evidence reviews accept the
subtraction; all frozen identities and product boundaries are preserved; and durable status records
the immutable completion commit. Completion removes only the legacy-Clippy-production blocker before
the 0.19.0 release candidate.
