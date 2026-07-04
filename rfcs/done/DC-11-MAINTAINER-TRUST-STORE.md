# RFC (done) — DC-11 Publication Signing and Minimal Trust Store

**Status.** Implemented (v0.4.0)
**Target release.** v0.4.0.
**Tracks.** Replacing the local seal MAINTAINER placeholder with real role-bound Ed25519
MAINTAINER signatures, and adding the minimal trust-store/policy surface needed for publication
verification to fail closed.
**Touches.** `seal`, signature verification, repository layout, trust-store read/write helpers,
`verify`/`doctor` diagnostics, CLI signer input, docs, and release notes.
**Companion FDD updates.** `../handoffs/DC-11-maintainer-trust-store/fdd-02-update.md`,
`../handoffs/DC-11-maintainer-trust-store/fdd-04-update.md`.

## Context

v0.3.0 made every AUTHOR-role Patch signature produced by production commands a real role-bound
Ed25519 signature. The remaining signing exception is publication: `prikk seal --allow-no-audit`
currently signs Block, RefState, and inline RefUpdate envelopes with the reserved key id
`dev-placeholder-maintainer` and deterministic SHA-256 marker bytes. That keeps seal useful as a local
durability scaffold, but it is not publication-grade repository trust.

DC-11 is sequenced after the v0.3.0/DC-10 release. Its implementation gate must stay closed until this
RFC and the companion FDD updates are re-reviewed with the errata below folded in.

The external design already reserves `.prikk/trust/keys/` and `.prikk/trust/policy.toml`, and assigns
`prikk-crypto` responsibility for Ed25519 signatures, role/domain separation, trust-store primitives,
and TOFU policy. DC-11 is the first focused design pass for that surface.

## Design goals

1. Production publication objects must carry real role-bound Ed25519 MAINTAINER signatures.
2. `seal` must not publish with a development placeholder or deterministic signature marker.
3. Verification must have a repository-local trust decision for MAINTAINER keys and fail closed when
   trust or policy is missing, ambiguous, malformed, or contradictory.
4. The first trust store must be small enough to review and test without implementing full PKI,
   remote sync, hardware signing, audit plugins, or key lifecycle automation.
5. Existing object identity must not drift: signatures remain outside object identity and sign the
   unsigned object ID plus object type, signer role, and key id.

## Proposed design

### Maintainer signer boundary

Add a `MaintainerSigner` boundary analogous to `AuthorSigner`.

- The signer exposes a non-empty key id and detached signature bytes.
- The production `Ed25519MaintainerSigner` uses a caller-supplied 32-byte Ed25519 seed.
- `seal` signs Block, RefState, and RefUpdate with
  `Signature::signed_bytes(Ed25519, object_type, object_id, Maintainer, key_id)`.
- The current dev salts are removed; they are not part of the canonical signature preimage.
- `RefUpdatePayload.author_key_id` is populated with the real MAINTAINER key id. This required payload
  field is identity-bearing, so new RefUpdate object IDs will differ from placeholder-era RefUpdates.
- `created_at` remains advisory and must not be used as a trust or freshness control.

`TagPayload.author_key_id` has the same identity-bearing shape, but tag publication/signing is outside
DC-11. The same real-key rule must be applied when tag signing is designed.

CLI key input for this increment:

```text
PRIKK_MAINTAINER_KEY_ID
PRIKK_MAINTAINER_SEED
```

This mirrors the AUTHOR environment input and is intentionally still key input, not key management.

### Minimal trust store

Initialize and open a repository-local trust store:

```text
.prikk/
  trust/
    keys/
      maintainer/
        <key-id>.pub
    policy.toml
```

For DC-11, trusted key files contain exactly one lowercase hex Ed25519 public key. The file name is
the key id. Key ids must use the existing non-empty signature key-id constraints plus a path-safe
subset for file storage; unsafe key ids are rejected before filesystem access.

The minimal policy is:

```toml
[maintainer]
required = 1
keys = ["<key-id>"]
```

Rules:

- `required = 1` is the only supported threshold in DC-11.
- The `keys` list is authoritative for trusted MAINTAINER key ids.
- Each listed key id must have exactly one matching public-key file.
- Duplicate key ids, malformed public keys, missing files, unknown roles, unsupported thresholds, and
  empty key sets fail closed.
- A publication object is trusted only when it carries at least one valid Ed25519 signature with
  `SignerRole::Maintainer`, a policy-listed key id, and a public key whose signature verifies.

### Trust bootstrap

This RFC intentionally chooses explicit local trust configuration over implicit TOFU for v0.4.0.

DC-11 includes a small helper:

```text
prikk trust maintainer add --key-id <key-id> --public-key <64-hex>
```

The helper is deliberately scoped to the DC-11 policy shape: one MAINTAINER key, `required = 1`, and an
atomic write of both the key file and policy file with directory fsync. It does not import secret keys,
rotate keys, revoke keys, or manage multiple maintainers. `seal` still requires an already-valid trust
policy before it can publish publication objects. This avoids silently trusting the first observed
publisher and gives tests a real trust-writing code path rather than hand-crafted files.

### Strict policy parsing

DC-11 uses a hand-written strict parser for the fixed policy shape instead of adding a TOML dependency.
The accepted grammar is only:

```toml
[maintainer]
required = 1
keys = ["<key-id>"]
```

Extra sections, unsupported keys, duplicate keys, duplicate fields, comments inside values, alternate
thresholds, non-string key entries, multiple keys, and malformed whitespace-sensitive constructs are
rejected. This keeps the dependency surface unchanged and makes fail-closed behavior reviewable. A full
TOML parser can be reconsidered when policy shape grows beyond this fixed grammar.

### Publication-time trust binding

Before any publication object write, `seal` must check a three-way binding:

1. `PRIKK_MAINTAINER_KEY_ID` appears in `policy.keys`;
2. exactly one `.prikk/trust/keys/maintainer/<key-id>.pub` file exists for that key id;
3. `Ed25519KeyPair::from_seed(PRIKK_MAINTAINER_SEED).public_key_bytes()` equals the trusted `.pub`
   file bytes.

Any mismatch fails closed before Block, RefState, RefUpdate, ref-log, ref-pointer, or WAL mutation.
The trust read and binding check must preserve the existing seal critical-section discipline: validate
trust before object writes, then continue through the locked publish sequence.

### Publication verification

`verify` should distinguish structural repository integrity from publication trust.

This is the first cryptographic signature-verification surface in repository-wide `verify`; current
verification is structural only. DC-11 adds trusted MAINTAINER verification for publication objects but
does not add AUTHOR Patch signature verification. AUTHOR verification remains a separate gap to close in
a later verification/policy increment.

Required behavior:

- Block, RefState, and RefUpdate publication envelopes are checked for trusted MAINTAINER signatures.
- RefUpdate log verification must validate the inline RefUpdate envelope signature against the trust
  store before treating it as publication evidence.
- RefState chain verification must validate each RefState object signature.
- Block verification must validate each reached Block signature.
- On missing or invalid trust configuration, verification reports a distinct publication-trust status or
  issue code rather than conflating the result with structural corruption.
- Verification uses the current local trust policy. There is no "trusted at seal time" model in DC-11;
  rotation, revocation windows, and historical trust semantics are left for RFC-025.

`doctor` may explain trust failures, but must not auto-create trust or repair signatures.

## Rejected alternatives

### Keep the MAINTAINER placeholder but narrow the release claim

Rejected. That preserves the publication-grade trust gap and blocks the next project-level signing
claim.

### Implicit TOFU during `seal`

Rejected for this increment. TOFU is allowed by the external design, but implicit trust on first seal is
easy to mistake for policy enforcement. Explicit local trust configuration is more reviewable.

### Trust AUTHOR keys as MAINTAINER keys

Rejected. AUTHOR and MAINTAINER are separate roles in the role-bound signature preimage. Reusing key
material is a caller decision, but trust policy must bind the key id to the MAINTAINER role.

### Implement full key lifecycle now

Rejected. Rotation, revocation, expiration, hardware signing, remote policy distribution, and
multi-maintainer thresholds belong to later RFC-025 work. DC-11 should close the local publication
placeholder first.

### Add a general TOML parser now

Rejected for this increment. The accepted policy shape is intentionally tiny, and a strict parser avoids
new dependency review until the policy language grows.

## Compatibility and identity rules

- Object IDs for Block and RefState payloads remain unchanged by signature replacement.
- RefUpdate payload identity changes for new publications because `author_key_id` must become the real
  MAINTAINER key id instead of `dev-placeholder-maintainer`.
- Existing v0.3.0 repositories sealed with `dev-placeholder-maintainer` are pre-publication-trust
  artifacts. DC-11 makes a clean break: publication-trust verification reports those histories as
  untrusted unless a later migration or compatibility mode is explicitly designed. This is more
  disruptive than DC-10's rollback-classification break, so v0.4.0 release notes must call it out.
- Production code must not carry `dev-placeholder-maintainer` as trusted publication identity.
- Test-only fixtures may keep placeholder signatures when explicitly named as legacy or scaffold
  fixtures.

## Implementation plan

1. Review this RFC and the companion FDD-02/FDD-04 updates before opening implementation.
2. Add repository layout paths for `.prikk/trust/`, `.prikk/trust/keys/maintainer/`, and
   `.prikk/trust/policy.toml`.
3. Add strict fixed-shape policy parser/validator with deterministic failure modes and no secret
   handling.
4. Add `prikk trust maintainer add --key-id <key-id> --public-key <64-hex>` for the single-key
   `required = 1` policy, using atomic writes and directory fsync.
5. Add `MaintainerSigner` / `Ed25519MaintainerSigner` and replace `seal`'s deterministic marker
   signatures.
6. Wire `seal` to `PRIKK_MAINTAINER_KEY_ID` / `PRIKK_MAINTAINER_SEED`.
7. Verify the three-way signer/policy/public-key binding before publication, so `seal` fails before
   writing objects if trust is not configured or the signer does not match the trusted key.
8. Add trusted MAINTAINER signature checks to repository verification for reached Blocks, RefStates,
   and inline RefUpdates.
9. Add distinct publication-trust status/issue codes separate from structural corruption.
10. Update `doctor` diagnostics to report trust-store and publication-signature failures without
   mutating trust.
11. Update docs, README, status, changelog, and release scope, including the pre-DC-11 legacy-history
    consequence.
12. Cut v0.4.0 only after local gates and release checks pass.

## Test gates

Required tests:

- `seal` refuses to run without MAINTAINER key input;
- `seal` refuses to run when the signer key id/public key is not trusted by local policy;
- `seal` refuses to run when the signer seed derives a public key different from the trusted `.pub`;
- `seal` writes real Ed25519 MAINTAINER signatures on Block, RefState, and RefUpdate;
- `seal` writes the real MAINTAINER key id into `RefUpdatePayload.author_key_id`;
- MAINTAINER signatures verify against the trusted public key;
- verification fails if object id, object type, signer role, key id, signature bytes, or trusted public
  key changes;
- verification rejects the old `dev-placeholder-maintainer` production path;
- trust policy rejects duplicate keys, missing key files, malformed public keys, unsupported threshold,
  unsafe key ids, extra sections, unsupported fields, and multi-key policies;
- repository initialization creates the trust-store directory scaffold;
- `prikk trust maintainer add` writes the key file and policy through the production trust path;
- `verify` reports publication trust failures separately from structural corruption;
- `doctor` reports trust failures and does not repair them.

Standard gates:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Out of scope

- key rotation, revocation, expiration, and migration;
- multi-maintainer thresholds beyond `required = 1`;
- hardware-backed signing and external secret stores;
- remote trust distribution or sync trust;
- audit plugin execution and attestation publication policy;
- rollback authorization policy;
- repository-format stability guarantees.

## Review rulings folded in

- Include the minimal `prikk trust maintainer add` helper.
- Report publication-trust failures as a distinct non-healthy trust status/issue code, not as structural
  corruption.
- Make a clean break for pre-DC-11 placeholder-sealed histories: no production legacy trust recognition.
