# FDD-04 - Current Trust and Threat Model Reference

Status: Current-state reference created by accepted DC-24
Scope: Released implementation through 0.16.0 plus accepted DC-24 documentation rules

## Numbering and Scope

`FDD-04` preserves continuity with the existing threat-model trace. This file consolidates the current
released trust and threat claims so public docs can link to a stable reference. It does not create or
complete FDD-01, FDD-02, FDD-03, or FDD-05; those remain unconsolidated or deferred unless later RFCs
create them. `FDD-00-DATA-MODEL.md` is the companion data-model reference.

## Core Caveats

- Prikk is early implementation software and is not a production Git replacement.
- `.prikk/` is Prikk's native repository format and is not Git-compatible storage.
- Ref files are pointers, not roots of trust.
- Maintainer trust is repository-local with the current minimal `required = 1` policy.
- `verify` is not a global trust proof.
- There is no key rotation, revocation, hardware signing, remote trust, sync trust, or stable migration
  policy yet.
- Durability and recovery claims are supported by current unit and integration tests, not by a
  completed crash-matrix or fuzzing campaign.
- Linux is the only platform exercised by the current project gates; cross-platform fsync and path
  semantics remain design targets.

## Trust Roots and Roles

Current signing uses role-bound Ed25519 signatures. The signature preimage binds the algorithm, object
type, object id, signer role, and key id. Signer roles include AUTHOR and MAINTAINER. Ed25519 signing
and strict verification live in `prikk-crypto`; trust stores, key persistence, rotation, revocation,
and policy are outside that crate.

AUTHOR signatures identify the key used by the authoring path for Patch envelopes. Production commit
and rollback-draft authoring use real Ed25519 AUTHOR signatures. Prikk does not currently implement a
repository-wide AUTHOR trust store, AUTHOR revocation, AUTHOR rotation, or AUTHOR identity policy.

MAINTAINER signatures identify publication objects. Seal uses real role-bound Ed25519 MAINTAINER
signatures for Block, RefState, and RefUpdate envelopes and verifies the signer against the local
maintainer trust policy before publishing.

## Key Input and Local Trust Store

Current key input is intentionally minimal. The CLI reads AUTHOR key material from
`PRIKK_AUTHOR_KEY_ID` and `PRIKK_AUTHOR_SEED`, and MAINTAINER key material from
`PRIKK_MAINTAINER_KEY_ID` and `PRIKK_MAINTAINER_SEED`. The seed values are caller-provided 32-byte
Ed25519 secret seeds encoded as 64 hex characters. Prikk does not provide local secret storage.

The local maintainer trust store supports a single repository-local trusted MAINTAINER key with
`required = 1`. `prikk trust maintainer add` writes the trusted public key and fixed-shape policy.
The parser deliberately rejects broader policy shapes. There is no implicit trust-on-first-use rule.

## What Seal Checks

Seal requires `--allow-no-audit`, a valid local branch ref, a non-empty active WAL, valid active ref
metadata matching the requested ref, and no trailing partial WAL bytes. It verifies that the configured
MAINTAINER signer matches the repository-local trust policy before publication. It then persists Patch
objects, signs and writes the Block and RefState, appends a signed RefUpdate, promotes the ref pointer,
and clears active state.

Current seal does not run audit plugins, evaluate attestation policy, perform semantic merge, publish
multi-parent merge Blocks, or provide remote trust distribution.

## What Verify Checks

`prikk verify` is read-only. It checks persisted object placement and identity, envelope decoding,
Block references, ref pointer/log consistency, active WAL records, active WAL metadata health,
rollback-draft structure for active and sealed rollback-marked Patches, and publication trust for
Block, RefState, and RefUpdate envelopes against the repository-local maintainer trust policy.

`verify` does not prove that a repository is globally trustworthy. It does not enforce repository-wide
AUTHOR trust, historical PKI semantics, revocation, rotation, threshold policy beyond `required = 1`,
remote policy, hosted identity, or complete crash-proof durability.

## Rollback-Draft Boundary

Rollback drafts are Patch objects whose payload purpose is `PatchPurpose::RollbackDraft`. Active
rollback-draft verification requires exactly one active WAL record, rejects trailing partial WAL bytes,
requires a rollback-draft Patch purpose, requires an AUTHOR Ed25519 signature, rejects the legacy
placeholder marker key id, requires 64-byte signature payloads, and compares the active payload with
the inverse Patch derived from the current ref.

This is structural and semantic validation for the supported rollback subset. It is not rollback
authorization, does not publish rollback refs, and does not enforce repository-wide AUTHOR trust.

## Threat Boundaries

Current protections target local repository corruption, malformed persisted data, wrong object
placement, ref pointer/log drift, active-WAL ownership drift, unsigned or untrusted publication
objects, and legacy rollback marker signatures. Diagnostics should avoid raw text spans, replacement
text, blob bytes, absolute host paths, `.prikk` private paths, signer secrets, key material, and
arbitrary object debug dumps.

Current non-goals include global identity trust, remote trust, hosted forge semantics, key lifecycle
management, hardware signing, multi-maintainer thresholds, production audit policy, plugin execution,
and stable repository-format migration.

## Claim-to-Source Anchors

| Claim | Source anchors |
|---|---|
| Ed25519 is the only current signing and verification algorithm. | `crates/prikk-crypto/src/lib.rs`; `crates/prikk-object/src/signature.rs` |
| Signature preimages bind algorithm, object type, object id, signer role, and key id. | `crates/prikk-object/src/signature.rs`; `crates/prikk-store/src/author_signing.rs`; `crates/prikk-store/src/maintainer_signing.rs` |
| AUTHOR signing is real Ed25519 on Patch envelopes, not a placeholder. | `crates/prikk-store/src/author_signing.rs`; `crates/prikk-store/src/worktree_patch/node_authoring.rs`; `rfcs/done/DC-10-ROLLBACK-DRAFT-SIGNING.md` |
| AUTHOR key material comes from environment variables and is not persisted by Prikk. | `crates/prikk-cli/src/main.rs`; `crates/prikk-store/src/author_signing.rs`; `rfcs/IMPLEMENTATION-STATUS.md` |
| MAINTAINER publication signing is real Ed25519 and role-bound. | `crates/prikk-store/src/maintainer_signing.rs`; `crates/prikk-cli/src/seal.rs`; `rfcs/done/DC-11-MAINTAINER-TRUST-STORE.md` |
| Maintainer trust is repository-local and limited to one key with `required = 1`. | `crates/prikk-store/src/trust.rs`; `crates/prikk-store/src/layout.rs`; `rfcs/handoffs/DC-11-maintainer-trust-store/fdd-04-update.md` |
| Seal validates the maintainer signer against local trust before publication. | `crates/prikk-cli/src/seal.rs`; `crates/prikk-store/src/trust.rs` |
| Verify checks publication trust for Block, RefState, and RefUpdate envelopes. | `crates/prikk-store/src/verify.rs`; `crates/prikk-store/src/trust.rs` |
| Verify does not enforce repository-wide AUTHOR trust. | `crates/prikk-store/src/verify.rs`; `crates/prikk-store/src/rollback_verify.rs`; `rfcs/IMPLEMENTATION-STATUS.md` |
| Rollback-draft verification is structural/semantic for the supported subset only. | `crates/prikk-store/src/rollback_verify.rs`; `rfcs/done/DC-14-ARBITRARY-SPAN-TEXT-INVERSE-ROLLBACK.md`; `rfcs/handoffs/DC-14-arbitrary-span-text-inverse-rollback/fdd-04-update.md` |
| Active WAL metadata integrity is part of verification and doctor diagnostics. | `crates/prikk-store/src/verify.rs`; `crates/prikk-store/src/doctor.rs`; `rfcs/done/DC-15-ACTIVE-SESSION-INTEGRITY-HARDENING.md` |
| Durability and platform claims remain limited by current test evidence. | `rfcs/handoffs/DC-24-data-model-trust-threat-docs/baseline-recap.md`; `rfcs/accepted/DC-24-DATA-MODEL-TRUST-THREAT-DOCS.md` |

## Provenance

This reference consolidates released records through DC-23 and accepted DC-24. It supersedes stale
v0.2.0-era notes that described MAINTAINER signing as deferred; the current released code signs
publication objects with real MAINTAINER Ed25519 signatures and verifies them against local trust.
