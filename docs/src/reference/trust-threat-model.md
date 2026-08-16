# Trust and Threat Model

This page is the authoritative current-state reference for Prikk's trust and threat model. It
describes the released implementation through 0.16.0 and is grounded in the code, released RFCs, and
implementation status records listed in the anchor table at the foot of the page.

## Core Caveats

- Prikk is early implementation software and is not a production Git replacement.
- `.prikk/` is Prikk's native repository format and is not Git-compatible storage.
- Ref pointers are mutable, not roots of trust.
- Maintainer trust is repository-local with the current minimal `required = 1` policy.
- `verify` is not a global trust proof.
- MAINTAINER key revocation exists (`prikk trust maintainer remove`); there is no key rotation, hardware
  signing, remote trust, sync trust, or stable migration policy yet, and no AUTHOR-identity revocation.
- Durability and recovery claims are supported by current unit and integration tests, not by a
  completed crash-matrix or fuzzing campaign.
- Repository *mutation* is exercised by project gates on Linux, macOS, and Windows (DC-87 Stage 2).
  Windows' anchoring guarantee is weaker than Linux/macOS in one stated way — see
  [platform support](./platform-support.md) for the exact gap and which of the nine durability
  guarantees are held, weaker, or documented no-ops there. Read-only commands are CI-gated on macOS
  and Windows too — see [platform support](./platform-support.md).

Changes that alter trust, threat, verification, signature, key-management, durability,
platform-support, or production-readiness claims require architect review or accepted RFC/DC coverage.
The local persistence and crash-recovery boundary is covered by the
[durability and crash recovery](./durability-recovery.md) reference. The current `verify` / `doctor`
diagnostic catalog is covered by the
[integrity and recovery diagnostics](./integrity-recovery.md) reference. Current operator setup for
environment key input and repository-local maintainer trust is covered by the
[security and signing setup](../guide/security-setup.md) guide. Physical trust-store paths and other
`.prikk/` authority boundaries are covered by the
[repository layout and authority](./repository-layout.md) reference. Repository path validation and
worktree write-safety limits are covered by the
[path and worktree safety](./path-safety.md) reference.

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
Ed25519 secret seeds encoded as 64 hex characters. Prikk does not provide local secret storage, key
generation, or public-key derivation. For the current setup workflow and seed-handling warnings, see
the [security and signing setup](../guide/security-setup.md) guide.

The local maintainer trust store supports a set of repository-local adopted MAINTAINER keys, with
`required = 1` continuing to mean any one adopted key's signature suffices. `prikk trust maintainer add`
adds a new key id to the set, or idempotently confirms an already-adopted id's matching key; it refuses
to replace an adopted id's key with a different one. This refusal is a trust-on-first-use rule: the
first public key seen for a key id is the one trusted for that id, permanently, until an operator
removes it out-of-band. There is no remote trust distribution.

## What Seal Checks

Seal requires `--allow-no-audit`, a valid local branch ref, a non-empty active WAL, valid active ref
metadata matching the requested ref, and no trailing partial WAL bytes. It verifies that the configured
MAINTAINER signer matches the repository-local trust policy before publication. It then persists Patch
objects, signs and writes the Block and RefState, durably appends the ref pointer as the commit point, appends
exactly one signed RefUpdate, confirms pointer/log agreement, and clears active state. Signer-backed
retry is also the only authority that may finish an exact interrupted publication.

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
| Ed25519 is the only current signing and verification algorithm. | [`prikk-crypto`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-crypto/src/lib.rs), [`signature.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-object/src/signature.rs) |
| Signature preimages bind algorithm, object type, object id, signer role, and key id. | [`signature.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-object/src/signature.rs), [`author_signing.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/author_signing.rs), [`maintainer_signing.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/maintainer_signing.rs) |
| AUTHOR signing is real Ed25519 on Patch envelopes, not a placeholder. | [`author_signing.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/author_signing.rs), [`node_authoring.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/worktree_patch/node_authoring.rs), [DC-10](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-10-ROLLBACK-DRAFT-SIGNING.md) |
| AUTHOR key material comes from environment variables and is not persisted by Prikk. | [`main.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-cli/src/main.rs), [`author_signing.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/author_signing.rs), [implementation status](https://github.com/nabbisen/prikk/blob/main/rfcs/IMPLEMENTATION-STATUS.md) |
| MAINTAINER publication signing is real Ed25519 and role-bound. | [`maintainer_signing.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/maintainer_signing.rs), [`seal.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-cli/src/seal.rs), [DC-11](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-11-MAINTAINER-TRUST-STORE.md) |
| Maintainer trust is repository-local, held as a set of adopted keys, with `required = 1` meaning any one adopted key's signature suffices. | [`trust.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/trust.rs), [`layout.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/layout.rs), [DC-11 FDD-04 handoff](https://github.com/nabbisen/prikk/blob/main/rfcs/handoffs/DC-11-maintainer-trust-store/fdd-04-update.md) |
| Seal validates the maintainer signer against local trust before publication. | [`seal.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-cli/src/seal.rs), [`trust.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/trust.rs) |
| Verify checks publication trust for Block, RefState, and RefUpdate envelopes. | [`verify.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/verify.rs), [`trust.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/trust.rs) |
| Verify does not enforce repository-wide AUTHOR trust. | [`verify.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/verify.rs), [`rollback_verify.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/rollback_verify.rs), [implementation status](https://github.com/nabbisen/prikk/blob/main/rfcs/IMPLEMENTATION-STATUS.md) |
| Rollback-draft verification is structural and semantic for the supported subset only. | [`rollback_verify.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/rollback_verify.rs), [DC-14](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-14-ARBITRARY-SPAN-TEXT-INVERSE-ROLLBACK.md), [DC-14 FDD-04 handoff](https://github.com/nabbisen/prikk/blob/main/rfcs/handoffs/DC-14-arbitrary-span-text-inverse-rollback/fdd-04-update.md) |
| Active WAL metadata integrity is part of verification and doctor diagnostics. | [`verify.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/verify.rs), [`doctor.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/doctor.rs), [DC-15](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-15-ACTIVE-SESSION-INTEGRITY-HARDENING.md) |
| Durability and platform claims remain limited by current test evidence. | [DC-24 baseline recap](https://github.com/nabbisen/prikk/blob/main/rfcs/handoffs/DC-24-data-model-trust-threat-docs/baseline-recap.md), [DC-24](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-24-DATA-MODEL-TRUST-THREAT-DOCS.md) |

## Provenance

This reference consolidates released records through DC-23 and DC-24. It supersedes stale
v0.2.0-era notes that described MAINTAINER signing as deferred; the current released code signs
publication objects with real MAINTAINER Ed25519 signatures and verifies them against local trust.
DC-26 moved this current-state reference from `rfcs/fdds/` into the published book without changing
code, schema, trust, or CLI behavior.
