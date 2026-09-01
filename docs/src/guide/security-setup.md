# Security and Signing Setup

This guide describes the current operator setup for Prikk signing and repository-local maintainer
trust. For the full security model, see the [trust and threat model](../reference/trust-threat-model.md).
For verification diagnostics after setup, see
[integrity and recovery diagnostics](../reference/integrity-recovery.md). For the physical trust-store
paths, see [repository layout and authority](../reference/repository-layout.md).

## Core Caveats

- Prikk is early implementation software and is not a production Git replacement.
- Current key input is environment-variable based and intentionally minimal.
- Seeds are secret key material. Prikk does not store, encrypt, rotate, revoke, expire, back up, or
  generate private keys.
- Prikk currently has no key-generation command and no command that derives a public key from a seed.
- Operators must obtain matched Ed25519 seed and public-key material with external tooling.
- Maintainer trust is repository-local, held as a set of adopted MAINTAINER keys with `required = 1`
  (any one adopted key's signature suffices), and enforces trust-on-first-use per key id.
- AUTHOR signatures are real Ed25519 signatures, but Prikk does not currently enforce a
  repository-wide AUTHOR trust policy.
- MAINTAINER key revocation exists (`prikk trust maintainer remove`); there is no key rotation, hardware
  signing, remote trust, sync trust, hosted identity, multi-maintainer threshold policy, or stable
  migration policy yet.

## Current Signing Roles

Prikk currently uses role-bound Ed25519 signatures.

AUTHOR signing is used for Patch envelopes produced by commit and rollback-draft authoring paths. The
AUTHOR signature identifies the key used by the authoring path, but it is not checked against a
repository-wide AUTHOR trust store.

MAINTAINER signing is used for publication objects. Seal signs Block, RefState, and RefUpdate
envelopes with the configured MAINTAINER signer and verifies that signer against the repository-local
maintainer trust policy before publishing.

The signature preimage binds the signature algorithm, object type, object id, signer role, and key id.

## Current Key Inputs

The CLI reads AUTHOR key material from:

- `PRIKK_AUTHOR_KEY_ID`
- `PRIKK_AUTHOR_SEED`

The CLI reads MAINTAINER key material from:

- `PRIKK_MAINTAINER_KEY_ID`
- `PRIKK_MAINTAINER_SEED`

Each seed value is a caller-supplied 32-byte Ed25519 secret seed encoded as 64 hex characters. Missing
variables, empty key ids, wrong-length seed hex, and non-hex seed bytes fail closed before signing.

Prikk does not currently derive the MAINTAINER public key from `PRIKK_MAINTAINER_SEED`. The operator
must provide the matching public key separately when configuring repository-local maintainer trust.

## Maintainer Trust Store Setup

The current commands for repository-local MAINTAINER trust are:

```text
prikk trust maintainer add --key-id ID --public-key HEX
prikk trust maintainer remove --key-id ID
```

`ID` must match the MAINTAINER key id used by `PRIKK_MAINTAINER_KEY_ID`. `HEX` must be the lowercase
64-hex-character Ed25519 public key that matches `PRIKK_MAINTAINER_SEED`.

`add` writes the trusted public key and adds it to the repository's adopted-key set, with `required = 1`
continuing to mean any one adopted key's signature suffices. Adopting a key id already in the set with
the same public key succeeds idempotently; adopting it again with a different public key is refused.
This refusal is Prikk's trust-on-first-use enforcement: the first public key seen for a key id is the
one trusted for that id, permanently, even after removal — `remove` takes a key id out of the adopted
set, but re-adding the same id later with a *different* public key is still refused. There is still no
remote trust distribution.

## Minimal Local Workflow

Use placeholders for seed and key values in documentation, scripts, and notes. Before running the
workflow below, populate the local shell variables `AUTHOR_SECRET_SEED_64_HEX`,
`MAINTAINER_SECRET_SEED_64_HEX`, and `MAINTAINER_PUBLIC_KEY_64_HEX` with key material generated and
handled outside Prikk.

```sh
prikk init ./sample-repo

export PRIKK_AUTHOR_KEY_ID="author-key-id"
export PRIKK_AUTHOR_SEED="$AUTHOR_SECRET_SEED_64_HEX"
export PRIKK_MAINTAINER_KEY_ID="maintainer-key-id"
export PRIKK_MAINTAINER_SEED="$MAINTAINER_SECRET_SEED_64_HEX"

(cd ./sample-repo && prikk trust maintainer add \
  --key-id "$PRIKK_MAINTAINER_KEY_ID" \
  --public-key "$MAINTAINER_PUBLIC_KEY_64_HEX")

echo "hello prikk" > ./sample-repo/readme.txt
(cd ./sample-repo && prikk commit -m "genesis")
(cd ./sample-repo && prikk seal --allow-no-audit)
(cd ./sample-repo && prikk verify)
```

The MAINTAINER seed and public key above must be matched private/public halves of one Ed25519 keypair.
If they do not match, seal fails because the configured signer is not trusted by the repository-local
policy.

## Seed Handling Warnings

Any seed or key values published in Prikk's README, quick start, docs, tests, review packages, or issue
comments are public examples. They are compromised by publication and must never be used for real
signing.

Do not commit real seeds, paste them into issues, store them in shell history, print them in CI logs,
or put them in release artifacts. Prikk does not currently provide a secret-storage boundary; the
operator owns secret generation, storage, backup, rotation, and destruction outside Prikk.

## Failure and Diagnostic Hints

Missing `PRIKK_AUTHOR_KEY_ID` or `PRIKK_AUTHOR_SEED` prevents commands that need AUTHOR signing from
creating signed Patch envelopes.

Missing `PRIKK_MAINTAINER_KEY_ID` or `PRIKK_MAINTAINER_SEED` prevents seal from creating signed
publication objects.

Malformed seed hex is rejected before signing. Empty key ids and unsafe key ids are rejected by shared
signature validation.

An untrusted MAINTAINER signer prevents seal from publishing. A repository with publication objects
that do not verify against the local trust policy reports publication-trust issues through `verify` and
`doctor`.

The current CLI wording is human diagnostic output, not a stable machine-readable key-management
contract.

## Deferred Work

Still deferred: key-generation commands, public-key derivation commands, local secret storage,
keychain integration, passphrase handling, key rotation, key expiration, compromise recovery, hardware
signing, multi-maintainer thresholds, repository-wide AUTHOR trust policy (including AUTHOR-identity
revocation — only MAINTAINER key revocation is supported), remote trust, hosted identity, JSON
key-management output, stable trust-policy migration, stable repository-format migration, and
production readiness.

## Claim-to-Source Anchors

| Claim | Source anchors |
|---|---|
| AUTHOR and MAINTAINER production signing use real Ed25519 signatures. | [`author_signing.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/author_signing.rs), [`maintainer_signing.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/maintainer_signing.rs), [DC-10](https://github.com/prikk-vcs/prikk/blob/main/rfcs/done/DC-10-ROLLBACK-DRAFT-SIGNING.md), [DC-11](https://github.com/prikk-vcs/prikk/blob/main/rfcs/done/DC-11-MAINTAINER-TRUST-STORE.md) |
| Signature preimages bind algorithm, object type, object id, signer role, and key id. | [`signature.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-object/src/signature.rs), [`author_signing.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/author_signing.rs), [`maintainer_signing.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/maintainer_signing.rs) |
| The CLI reads AUTHOR and MAINTAINER key material from environment variables and expects 64-hex secret seeds. | [`main.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-cli/src/main.rs), [`author_signing.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/author_signing.rs), [`maintainer_signing.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/maintainer_signing.rs) |
| Prikk currently exposes `trust maintainer add` but no key-generation or public-key-derivation command. | [`main.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-cli/src/main.rs), [`help.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-cli/src/output/help.rs), [DC-30](https://github.com/prikk-vcs/prikk/blob/main/rfcs/done/DC-30-KEY-MANAGEMENT-SIGNING-SETUP-GUIDE.md) |
| The maintainer trust store is repository-local and fixed to one MAINTAINER key with `required = 1`. | [`trust.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/trust.rs), [DC-11](https://github.com/prikk-vcs/prikk/blob/main/rfcs/done/DC-11-MAINTAINER-TRUST-STORE.md), [trust and threat model](../reference/trust-threat-model.md) |
| Seal verifies the configured MAINTAINER signer against local trust before publication. | [`seal.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-cli/src/seal.rs), [`trust.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/trust.rs) |
| Verify checks publication trust for Block, RefState, and RefUpdate objects against local MAINTAINER trust. | [`verify.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/verify.rs), [`trust.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/trust.rs), [integrity and recovery diagnostics](../reference/integrity-recovery.md) |
| Current AUTHOR signatures are not checked against a repository-wide AUTHOR trust policy. | [`verify.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/verify.rs), [`rollback_verify.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/rollback_verify.rs), [trust and threat model](../reference/trust-threat-model.md) |

## Provenance

This guide implements DC-30. It is documentation-only and does not change signing, trust, CLI, object
schema, repository format, verification, seal, or repository behavior.
