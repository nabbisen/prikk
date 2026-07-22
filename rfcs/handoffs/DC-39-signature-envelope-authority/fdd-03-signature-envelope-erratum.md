# DC-39 FDD-03 Signature and Envelope Erratum

**Status.** Accepted companion authority inherited from DC-39 after architect design re-review on
2026-07-22; implementation evidence remains pending.
**Parent RFC.** `../../accepted/DC-39-SIGNATURE-ENVELOPE-AUTHORITY.md`.
**Upstream authority.** `../../accepted/DC-34-PUBLICATION-IDENTITY-AUTHORITY.md`.

## Purpose

The historical FDD-03 signature text used `PRIKK-SIGNATURE-v1` and ordered object type before
algorithm. Released Prikk instead uses `prikk.sig.v1` and orders algorithm before object type. DC-34
ratifies the released bytes. This companion records the exact correction, canonical multi-signature
envelope rule, and RefUpdate no-clock rule for implementation reviewers who did not review DC-34.

## Corrected signature preimage

```text
"prikk.sig.v1"
|| u16be(signature_algorithm)
|| u16be(object_type)
|| object_id[32]
|| u16be(signer_role)
|| u16be(key_id_byte_length)
|| key_id_ascii_bytes
```

There is no terminator. The registry codes and key-id grammar are exactly those in DC-34. Any future
change requires a new explicit version/domain and migration design.

## Required golden vector

With the public test-only Ed25519 seed consisting of 32 `0x42` bytes, object type RefUpdate, ObjectId
bytes `00` through `1f`, role MAINTAINER, and key id `maintainer_1`:

```text
public_key =
2152f8d19b791d24453242e15f2eab6cb7cffa7b6a5ed30097960e069881db12

preimage =
7072696b6b2e7369672e763100010004000102030405060708090a0b0c0d0e0f
101112131415161718191a1b1c1d1e1f0002000c6d61696e7461696e65725f31

signature =
102c73afdf34fcd4517b9c479a11c392e629da37cde58b8e882cc9b3ae282619
4c3ab6be87446865ce5cdaef12ffc4ed8dd87b1ec7f87a8d8ae9e02c5f1fb10d
```

## Canonical signature sequence

For each signature define:

```text
K = (key_id bytes, signer_role u16, algorithm u16, signature bytes)
```

The envelope sequence is strictly increasing by `K`, using unsigned lexicographic byte comparison for
both byte fields and unsigned numeric comparison for registry codes. Equal `K` is a duplicate.
Advisory signature `created_at` is excluded from ordering and duplicate identity.

Structural format-1 decode preserves bytes and order for diagnosis. Every new write and every
format-2 read requires strict uniqueness and order. Neither path silently normalizes persisted bytes.
Format-1 duplicate/order findings are warning-level legacy diagnostics and do not make those bytes
canonical. DC-40 owns repository-format selection and consumes DC-39's strict validator.

Ed25519 has exactly 64 signature bytes under strict validation. The structural format-1 decoder keeps
the released non-empty rule: non-empty lengths other than 64 remain readable and receive one
`PRIKK-VERIFY-SIGNATURE-MALFORMED` warning per envelope; zero bytes remain structurally invalid. Strict
new-write and format-2 validation reject every non-64-byte Ed25519 signature before output or
mutation. This syntactic shape check precedes and does not replace public-key authorization or
cryptographic verification.

`ObjectEnvelope::encode_canonical` and `to_canonical_bytes()` are governed new-byte emitters. They
must strictly validate before emitting any envelope field. `add_signature` must reject an invalid
pre-existing vector without mutation before validating and canonically inserting the new signature.

For each format-1 envelope, verification emits at most one issue per signature-envelope code in this
fixed order: malformed, duplicate, non-canonical order. Equal adjacent tuples produce the duplicate
code, not the order code; a separate descending adjacency can produce the order code. Envelope issue
groups follow canonical object type/ObjectId order, WAL sequence, unsigned lexicographic ref-name
bytes, and ref-log record order.

## RefUpdate time correction

Schema-1 `RefUpdatePayload.created_at` is exactly zero for production writes. Zero means no-clock and
is not an event timestamp. Format-1 read-only verification may preserve a historical non-zero value
with `PRIKK-VERIFY-REF-LEGACY-TIMESTAMP`; format-2 reads and every mutation reject non-zero. This field
is distinct from advisory `Signature.created_at`.

## Review gate

This erratum was accepted with DC-39 on 2026-07-22. It is not an independent RFC and does not by
itself accept implementation, authorize format-2 implementation or migration, or make a stable-format
claim.
