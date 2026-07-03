# DC-11 FDD-04 Update — Publication Trust Threat Model

Status: Reviewed DC-11 update  
Related RFC: `../../proposed/DC-11-MAINTAINER-TRUST-STORE.md`  
Target FDD: FDD-04 Threat Model

## Purpose

DC-11 closes the local publication-signing placeholder by requiring real role-bound Ed25519
MAINTAINER signatures and repository-local trust policy for publication verification.

## Required FDD-04 Body Updates

### Section 3 — Assets

Add a critical asset:

| Asset | Criticality | Notes |
|---|---|---|
| Repository-local trust policy and MAINTAINER public keys | Critical | Durable local authority for publication trust; not a cache; ambiguity fails closed |

### Section 4 — Trust Boundaries

Add a publication-trust decision boundary:

- Inputs: `.prikk/trust/policy.toml`, `.prikk/trust/keys/maintainer/<key-id>.pub`, publication object
  envelopes, and local signer key input.
- Decision: whether a Block, RefState, or RefUpdate publication signature is trusted for the local
  repository.
- Failure posture: missing, malformed, ambiguous, or contradictory trust data is a publication-trust
  failure, reported separately from structural corruption.

### Signature Replay and Role Confusion

Add controls:

- Block, RefState, and RefUpdate publication signatures use
  `Signature::signed_bytes(Ed25519, object_type, object_id, Maintainer, key_id)`.
- Verification must reject a valid signature made for a different object type, object id, signer role,
  or key id.
- AUTHOR signatures are not publication authority.
- `Signature.created_at` remains advisory only.
- `RefUpdatePayload.author_key_id` must record the real MAINTAINER key id; it must not retain
  `dev-placeholder-maintainer` in production payload identity.

### Trust Store Boundary

Add trust-store controls:

- `.prikk/trust/` is local durable authority, not a cache.
- Trust policy ambiguity fails closed.
- Missing policy, missing public keys, duplicate key ids, malformed public keys, unsafe key ids, and
  unsupported thresholds are trust failures.
- `seal` must check the three-way binding between signer key id, trusted public-key file, and public key
  derived from the signer seed before any object write.
- Repository-wide `verify` adds cryptographic MAINTAINER verification for publication objects. AUTHOR
  Patch signature verification remains a separate gap unless explicitly added by a later increment.
- DC-11 verifies against the current local trust policy; there is no historical "trusted at seal time"
  model until key lifecycle work defines it.
- `doctor` may diagnose trust failures but must not auto-trust keys or repair signatures.

### Placeholder Removal

Residual closed for new publication:

- production publication code must not use `dev-placeholder-maintainer`;
- production verification must not treat `dev-placeholder-maintainer` as trusted publication identity;
- pre-DC-11 placeholder-sealed histories are pre-publication-trust artifacts and receive no production
  legacy trust recognition in DC-11.

## Required Security Tests

- MAINTAINER signatures verify with the trusted Ed25519 public key;
- verification fails when the object id, object type, role, key id, public key, or signature bytes are
  changed;
- `seal` fails closed before publishing when trust configuration is missing or malformed;
- `seal` fails closed before publishing when the signer seed derives a public key different from the
  trusted public-key file;
- `verify` reports publication-trust failures distinctly from structural corruption;
- old placeholder publication signatures are not trusted in production logic.
