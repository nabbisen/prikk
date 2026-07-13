# RFC (done) - DC-30 Key Management and Signing Setup Guide

**Status.** Released in 0.17.4.
**Target release.** 0.17.4.
**Tracks.** TASK-09 key management and signing setup.
**Touches.** mdBook guide documentation, README links, trust/threat cross-links, roadmap/status docs.
**Companion handoff.** None. This is a current-state operator guide and does not create a gating FDD.

## Context

DC-24 added the current trust and threat model. DC-26 moved current-state references into the
published mdBook. DC-28 documented durability/recovery, and DC-29 documented verify/doctor integrity
diagnostics. The remaining Tier-1 documentation gap is operational: users can see that AUTHOR and
MAINTAINER signatures exist, but there is no dedicated guide for setting up the current key input and
repository-local maintainer trust store safely.

The README quick start currently shows environment variables and `trust maintainer add`, but it cannot
carry the full caveat set without becoming too long. DC-30 should turn that quick-start fragment into a
reviewed operator guide while keeping the trust/threat reference as the authoritative model.

DC-30 closes that documentation gap without changing signing, trust, CLI, repository format, key
storage, verification, seal, or publish behavior.

## Problem

1. **The signing setup path is easy to copy without understanding.** The README demonstrates
   `PRIKK_AUTHOR_*`, `PRIKK_MAINTAINER_*`, and `trust maintainer add`, but does not explain their
   roles, ordering, or security limits.
2. **Environment seeds are sensitive key material.** Current seed input is intentionally minimal and
   caller-supplied; users need plain warnings that seeds are not stored by Prikk and must not be
   checked into repositories, shells, CI logs, docs, or releases.
3. **MAINTAINER trust is real but narrow.** Seal signs Block, RefState, and RefUpdate envelopes with
   real role-bound Ed25519 MAINTAINER signatures and checks the signer against repository-local trust,
   but the policy is deliberately fixed at one trusted key with `required = 1`.
4. **AUTHOR trust is easy to overread.** Commit and rollback-draft production paths use real
   role-bound AUTHOR Ed25519 signatures, but there is no repository-wide AUTHOR trust store or AUTHOR
   identity policy.
5. **Key lifecycle features remain absent.** Rotation, revocation, expiration, hardware signing,
   remote trust, hosted identity, and multi-maintainer threshold policy are not implemented and must
   not be implied by the guide.
6. **The current CLI consumes key material but does not generate it.** Prikk has no key-generation
   command and no command that derives a public key from a seed. Operators must already have a matched
   Ed25519 secret seed and public key before configuring the local trust store.

## Design Goals

1. Add a current-state operator guide at `docs/src/guide/security-setup.md`.
2. Explain the current roles: AUTHOR signs Patch envelopes; MAINTAINER signs publication objects.
3. Explain the current key-input mechanism: `PRIKK_AUTHOR_KEY_ID`, `PRIKK_AUTHOR_SEED`,
   `PRIKK_MAINTAINER_KEY_ID`, and `PRIKK_MAINTAINER_SEED`.
4. State the exact current seed shape: 32-byte Ed25519 secret seed encoded as 64 hex characters.
5. Explain how to add the repository-local trusted MAINTAINER public key with
   `prikk trust maintainer add --key-id ID --public-key HEX`.
6. Explain that the trust policy is repository-local, fixed-shape, and limited to one trusted
   MAINTAINER key with `required = 1`.
7. Explain that Prikk currently ships no key-generation or public-key-derivation command: the operator
   must generate the Ed25519 seed and derive its corresponding public key with external tooling, and
   `PRIKK_MAINTAINER_SEED` plus `--public-key` must be matched private/public halves of one Ed25519
   keypair.
8. Explain the minimum operator sequence for current local use: set AUTHOR env, set MAINTAINER env,
   add the maintainer public key to the repository trust store, commit, seal, verify.
9. Include honest caveats about sensitive seed handling, public sample/test seeds, missing key
   lifecycle, missing AUTHOR trust policy, missing local secret storage, missing remote trust, and
   early implementation status.
10. Cross-link the guide from README and `docs/src/reference/trust-threat-model.md`.
11. Include visible claim-to-source anchors tying operational claims to released records and current
    code.

## Non-goals

DC-30 does not add:

- code, schema, CLI behavior, repository behavior, or release semantics;
- a new key-generation command;
- local secret storage or keychain integration;
- passphrase handling;
- hardware signing;
- key rotation, revocation, expiration, compromise recovery, or key history;
- multi-maintainer policy, thresholds beyond `required = 1`, or remote/hosted trust distribution;
- repository-wide AUTHOR trust enforcement;
- automatic trust-on-first-use;
- JSON/machine-readable key-management output;
- a stable repository-format or key-policy migration guarantee;
- a new current-state FDD under `rfcs/fdds/`.

## Proposed Documentation Shape

Create:

```text
docs/src/guide/security-setup.md
```

Add it under the mdBook `# Guide` section near the other user-facing workflow pages:

```md
- [Security and Signing Setup](guide/security-setup.md)
```

The page should be a practical operator guide, not a new trust-model reference and not a future design.
It should cross-link:

- `docs/src/reference/trust-threat-model.md` for trust scope;
- `docs/src/reference/integrity-recovery.md` for verify/doctor publication-trust diagnostics;
- `README.md` or the README quick start for the compact command flow when useful.

### Boundary With Trust/Threat Reference

The trust/threat reference remains the authority for security model claims. DC-30 owns operational
setup wording: which variables are read, how the maintainer trust store is initialized, what sequence
an operator runs today, and which foot-guns must be avoided.

The DC-30 guide may summarize trust boundaries for safety, but it must link to the trust/threat
reference for the full model and must not duplicate the entire threat model.

### Required Sections

The implemented page should contain at least:

1. **Core Caveats.** Early implementation, no production replacement, no secret storage, seeds are key
   material, no key-generation or public-key-derivation command, no key lifecycle, no remote trust, no
   AUTHOR trust store.
2. **Current Signing Roles.** AUTHOR signs Patch envelopes; MAINTAINER signs Block, RefState, and
   RefUpdate publication envelopes.
3. **Current Key Inputs.** Exact environment variables, required value shape, and fail-closed behavior
   when variables are missing or malformed.
4. **Maintainer Trust Store Setup.** `trust maintainer add` command shape, key-id/public-key
   requirements, the requirement that the configured MAINTAINER seed and public key belong to the same
   Ed25519 keypair, repository-local policy file outcome at a conceptual level, and `required = 1`.
5. **Minimal Local Workflow.** A concise command flow from initialized repo through commit/seal/verify,
   using placeholders rather than real-looking secret values.
6. **Seed Handling Warnings.** Do not commit seeds, paste real seeds into issues, store them in shell
   history, put them in CI logs, or reuse sample/test seeds for real signing. Any seed or key values
   published in Prikk's README, quick start, docs, or tests are public examples and must never be used
   for real signing.
7. **Failure and Diagnostic Hints.** Missing env vars, empty key ids, malformed seed hex, untrusted
   maintainer signer, and publication-trust verification failures should point to current CLI behavior
   without promising stable wording.
8. **Deferred Work.** Key generation, local secret storage, key rotation/revocation/expiration,
   hardware signing, multi-maintainer thresholds, AUTHOR trust policy, remote trust, hosted identity,
   stable migration.
9. **Claim-to-Source Anchors.** Code and released-record anchors for every security-significant claim.

## Required Current-State Claims

The guide must state:

- AUTHOR and MAINTAINER production signing use real Ed25519 signatures, not placeholders.
- The signature preimage is role-bound and includes algorithm, object type, object id, signer role,
  and key id.
- `PRIKK_AUTHOR_SEED` and `PRIKK_MAINTAINER_SEED` are caller-supplied 32-byte secret seeds encoded as
  64 hex characters.
- Prikk does not currently store local private keys or manage secrets.
- Prikk does not currently provide a key-generation command or a public-key-derivation command.
- Operators must generate Ed25519 seeds and derive the matching public keys with external tooling.
- `prikk trust maintainer add --key-id ID --public-key HEX` configures the repository-local
  MAINTAINER trust store.
- The `PRIKK_MAINTAINER_SEED` value and the `--public-key` value supplied to
  `trust maintainer add` must be the matched private/public halves of one Ed25519 keypair.
- The trust policy is currently one MAINTAINER key with `required = 1`.
- Seal verifies the configured MAINTAINER signer against local trust before publication.
- Verify checks publication trust for Block, RefState, and RefUpdate objects against the local
  MAINTAINER trust store.
- Current AUTHOR signatures are not checked against a repository-wide AUTHOR trust policy.

## Forbidden Claims

The guide must not claim:

- that Prikk stores, generates, encrypts, rotates, revokes, expires, or backs up private keys;
- that Prikk can derive a maintainer public key from `PRIKK_MAINTAINER_SEED`;
- that env-var seed handling is production-grade key management;
- that sample/test seeds, README quick-start seeds, documentation seeds, or published example keys are
  safe for real signing;
- that MAINTAINER trust is global or remote;
- that AUTHOR signatures imply repository-wide AUTHOR identity trust;
- that multiple maintainers or threshold policies are supported beyond `required = 1`;
- that `verify` proves global trust;
- that the current CLI output is a stable machine-readable key-management contract;
- that repository format or trust-policy migration is stable.

## Source Audit List

Implementation should audit at least:

- `crates/prikk-cli/src/main.rs` for env-var loading, seed decoding, trust command dispatch, and
  command failure wording;
- `crates/prikk-cli/src/output/help.rs` for visible command help and the absence of key-generation or
  public-key-derivation commands;
- `crates/prikk-store/src/author_signing.rs` for AUTHOR signing behavior;
- `crates/prikk-store/src/maintainer_signing.rs` for MAINTAINER signing behavior;
- `crates/prikk-store/src/trust.rs` for maintainer trust-store and fixed-shape policy behavior;
- `crates/prikk-cli/src/seal.rs` for seal-time trust verification and publication signing;
- `crates/prikk-store/src/verify.rs` and `crates/prikk-store/src/trust.rs` for publication-trust
  verification behavior;
- `crates/prikk-object/src/signature.rs` for signature role/key-id/preimage validation;
- `docs/src/reference/trust-threat-model.md`;
- `docs/src/reference/integrity-recovery.md`;
- `README.md`;
- `rfcs/done/DC-10-ROLLBACK-DRAFT-SIGNING.md`;
- `rfcs/done/DC-11-MAINTAINER-TRUST-STORE.md`;
- `rfcs/done/DC-24-DATA-MODEL-TRUST-THREAT-DOCS.md`;
- `rfcs/done/DC-29-VERIFY-DOCTOR-INTEGRITY-RECOVERY-REFERENCE.md`;
- `rfcs/IMPLEMENTATION-STATUS.md`.

## Implementation Review Requirements

The implementation review should verify:

- the page is an operator guide, not a new security model or feature design;
- every command shown is accepted by the current CLI;
- placeholders are used for secret seeds rather than real-looking reusable values;
- the guide explains how the maintainer public key relates to the maintainer seed without inventing a
  key-generation command;
- the guide states that current operators must obtain matched Ed25519 seed/public-key material through
  external tooling;
- the guide explicitly names README/docs/test seed and key values as public examples that must never be
  used for real signing;
- the guide clearly separates AUTHOR signing from AUTHOR trust;
- the guide clearly separates repository-local MAINTAINER trust from global/remote trust;
- the guide does not imply key lifecycle features or production-grade secret management;
- README and trust/threat cross-links make the guide discoverable;
- mdBook output builds and generated links resolve.

## Expected Files

Likely implementation files:

- `docs/src/guide/security-setup.md`;
- `docs/src/SUMMARY.md`;
- `docs/src/reference/trust-threat-model.md`;
- `docs/src/reference/integrity-recovery.md` if a short cross-link is useful;
- `README.md`;
- `ROADMAP.md`;
- `rfcs/README.md`;
- `rfcs/IMPLEMENTATION-STATUS.md`.

## Completion Criteria

DC-30 is complete when:

- the design is accepted and moved to `rfcs/accepted/`;
- the guide is implemented and reviewed;
- the guide is reachable in mdBook navigation;
- README and trust/threat reference link to the guide;
- status/roadmap files record the increment;
- review confirms the guide is accurate, caveated, and does not add behavior claims.
