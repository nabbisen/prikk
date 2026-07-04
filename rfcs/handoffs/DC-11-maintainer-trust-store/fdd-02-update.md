# DC-11 FDD-02 Update — Minimal Trust Store Layout

Status: Reviewed DC-11 update  
Related RFC: `../../done/DC-11-MAINTAINER-TRUST-STORE.md`
Target FDD: FDD-02 Storage Transaction Model

## Purpose

DC-11 introduces the first repository-local trust-store files used by publication signing and
verification. The layout must be explicit because trust files become durable local authority, unlike
rebuildable caches.

DC-11 uses explicit local trust configuration rather than implicit TOFU. The implementation includes a
minimal `prikk trust maintainer add` helper so trust material is written through a reviewed production
path.

## Required FDD-02 Body Updates

### Repository Layout

Add required directories:

```text
.prikk/trust/
.prikk/trust/keys/
.prikk/trust/keys/maintainer/
```

Add required file:

```text
.prikk/trust/policy.toml
```

For DC-11, `policy.toml` may be absent only before trust has been configured. Publication commands that
require trust must fail closed when it is absent.

### Trust File Semantics

- `.prikk/trust/keys/maintainer/<key-id>.pub` contains exactly one lowercase hex-encoded 32-byte
  Ed25519 public key.
- `<key-id>` is a storage-safe key id, not an arbitrary path. It must reject separators, traversal,
  empty strings, and ambiguous names.
- `.prikk/trust/policy.toml` records the MAINTAINER policy for DC-11:

```toml
[maintainer]
required = 1
keys = ["<key-id>"]
```

The DC-11 parser is a strict fixed-shape parser, not a general TOML implementation. It accepts only the
single `[maintainer]` section, `required = 1`, and one key id in `keys = ["<key-id>"]`. Extra sections,
unsupported fields, duplicate fields, duplicate keys, malformed public keys, missing key files,
multi-key policies, and unsafe key ids fail closed.

### Transaction Rules

- Trust updates are not auto-repaired by `doctor`.
- `prikk trust maintainer add --key-id <key-id> --public-key <64-hex>` writes the public-key file and
  policy file atomically and fsyncs the containing directories.
- `seal` must read and validate trust configuration before publishing Block, RefState, or RefUpdate
  objects.
- If trust configuration is malformed or incomplete, `seal` fails before object writes.
- Before object writes, `seal` must verify the three-way binding:
  1. signer key id is listed in policy;
  2. exactly one maintainer public-key file exists for that key id;
  3. the public key derived from `PRIKK_MAINTAINER_SEED` equals the trusted public-key file.

## Required Tests

- repository initialization creates the trust directory scaffold;
- trust policy/key reads reject malformed or unsafe key ids;
- missing policy/key material makes publication fail closed;
- signer seed / trusted public-key mismatch makes publication fail closed before object writes;
- `prikk trust maintainer add` writes the DC-11 policy shape atomically;
- no trust file is treated as a rebuildable cache.
