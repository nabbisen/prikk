# RFC (accepted) - DC-39 Signature and Envelope Authority

**Status.** Accepted after architect design re-review on 2026-07-22; implementation complete at
`8f565f2` after accepted post-commit evidence review. Awaits the 0.18.0 release lifecycle transition.
DC-34 is accepted upstream authority.
**Target milestone.** M1 - 0.18.0 corrective release.
**Tracks.** Architect review B5, N1, and N2.
**Baseline.** `df80c14932df15a1e8f0f54e3a6f79401efd0003`.
**Touches.** Signature vectors, envelope validation/canonical ordering, RefUpdate time semantics,
current-state docs, and compatibility tests.
**Companion FDD erratum.**
`../handoffs/DC-39-signature-envelope-authority/fdd-03-signature-envelope-erratum.md` records the
byte-level FDD-03 correction and follows this RFC's lifecycle.

## Problem

Accepted DC-34 ratifies the released `prikk.sig.v1` signature preimage instead of the contradictory
older FDD-03 preimage, but the contract has no literal independently reproducible signature vector.
The released envelope API also sorts signatures using advisory `created_at`, accepts duplicate
signatures, and does not distinguish structural legacy decoding from strict canonical acceptance.

Tightening the shared decoder alone would make some format-1 bytes unreadable before they can be
diagnosed. Leaving the current writer behavior unchanged would permit semantically equivalent
signature sets to have different persistent bytes. DC-39 therefore separates structural decoding,
legacy diagnostics, and strict new-write/current-format validation.

## Signature preimage authority

Version 1 uses DC-34's exact bytes, with no terminator or framing beyond the stated fields:

```text
"prikk.sig.v1"
|| u16be(signature_algorithm)
|| u16be(object_type)
|| object_id[32]
|| u16be(signer_role)
|| u16be(key_id_byte_length)
|| key_id_ascii_bytes
```

The immutable codes and key-id grammar are those in DC-34. Signing and verification must construct
this preimage through one shared `prikk-object` helper. No caller may reproduce it manually.

### Golden Ed25519 vector

The following fixture is public test material, never an operational secret:

| Field | Value |
|---|---|
| Ed25519 seed | 32 bytes, each `0x42` |
| Public key | `2152f8d19b791d24453242e15f2eab6cb7cffa7b6a5ed30097960e069881db12` |
| Algorithm | Ed25519, `0x0001` |
| Object type | RefUpdate, `0x0004` |
| ObjectId | bytes `00 01 02 ... 1f` in ascending order |
| Signer role | MAINTAINER, `0x0002` |
| Key id | ASCII `maintainer_1`, 12 bytes |

The exact preimage is 64 bytes:

```text
7072696b6b2e7369672e763100010004000102030405060708090a0b0c0d0e0f
101112131415161718191a1b1c1d1e1f0002000c6d61696e7461696e65725f31
```

The exact deterministic Ed25519 signature is:

```text
102c73afdf34fcd4517b9c479a11c392e629da37cde58b8e882cc9b3ae282619
4c3ab6be87446865ce5cdaef12ffc4ed8dd87b1ec7f87a8d8ae9e02c5f1fb10d
```

The vector must verify from the pinned public key without regenerating the key. A separate fixture may
regenerate it from the public test seed to prove deterministic construction. Mutation tests must alter
each preimage field independently and prove that the unchanged signature fails. Signature
`created_at` is intentionally absent from this preimage and is not covered by that assertion.

## Envelope canonical contract

### Structural validation

Structural validation checks the non-zero envelope schema and each signature's registered algorithm,
registered role, key-id grammar, and non-empty signature bytes. It deliberately does not enforce the
registered algorithm's fixed signature length so released format-1 records with non-empty malformed
Ed25519 lengths remain readable for diagnosis. It preserves signature order and bytes. The base
persistent decoder performs only framing, registry decoding, and this structural validation; it does
not sort, deduplicate, or silently grant current-format validity. A zero-byte signature remains
structurally malformed under the released rule and is not a readable compatibility form.

### Strict validation

Strict validation first performs structural validation, then enforces the registered algorithm's
syntactic signature shape. Ed25519 (`0x0001`) requires exactly 64 signature bytes. This shape check is
separate from public-key authorization and cryptographic verification. Strict validation then
requires all signature tuples to be unique and already in canonical order. For a signature `s`,
define:

```text
K(s) = (s.key_id.as_bytes(), s.signer_role.code(), s.algorithm.code(), s.signature_bytes)
```

Comparison is unsigned lexicographic byte order for key-id and signature bytes, and unsigned numeric
order for the two `u16` registry codes. The signature vector must be strictly increasing by `K`.
Equality of `K` is a duplicate even when the two advisory signature `created_at` values differ.
`created_at` neither orders nor distinguishes signatures.

`ObjectEnvelope::add_signature` must first strictly validate the complete pre-existing envelope. It
then strictly validates the incoming signature's structure and algorithm shape, rejects it when an
existing signature has the same `K`, inserts it, and leaves the complete vector sorted by `K`.
Failure leaves the envelope unchanged. Consequently, successful insertion always establishes a
strictly valid envelope and cannot be used to normalize or partially repair a directly constructed
invalid vector. A strict validator must reject callers that construct the public `signatures` vector
directly in duplicate, non-canonical, or algorithm-malformed form. ObjectId construction remains
unchanged because signatures remain outside its preimage.

### Public canonical serializer

`ObjectEnvelope` is public and implements `CanonicalEncode`; therefore
`ObjectEnvelope::encode_canonical` and the generic `to_canonical_bytes()` path are new-byte emitters
within DC-39's authority. Envelope canonical encoding must call strict validation before writing any
field. A directly constructed duplicate, inverted, or algorithm-malformed signature vector returns an
error and emits no bytes to the supplied canonical writer. The implementation must ensure failed
validation occurs before any partial output is appended.

This rule governs envelope serialization only. Payload types retain their existing canonical encode
contracts, and ObjectId construction continues to hash the unsigned object type, schema, and payload
rather than serialized envelope bytes.

### Format and operation matrix

| Surface | Required behavior |
|---|---|
| Base envelope decode | Structural only; preserves exact signature order and bytes. |
| Format-1 read/verify | Structurally decode; verify every signature with a valid algorithm shape otherwise eligible for verification; report malformed shape, duplicate, and non-canonical order diagnostics; never rewrite. |
| Public canonical envelope serializer | Require strict validation before emitting any envelope bytes; fail without partial output. |
| Any new persistent write | Require strict envelope validation before the first persistent mutation, regardless of repository format. |
| Format-2 read/verify | Require strict validation and fail closed on malformed shape, duplicate, or non-canonical signatures. |
| Format-1 to format-2 transition | No in-place normalization or migration; DC-40's new-repository/re-authoring boundary applies. |

The format-1 diagnostics are stable warning-level, non-canonical legacy findings. They do not by
themselves classify an otherwise structurally readable format-1 envelope as corruption, although
format-1 `verify` still returns non-zero under DC-40 because its state roots are not verifiable:

- `PRIKK-VERIFY-SIGNATURE-MALFORMED` when a structurally readable signature has a byte length that
  does not match its registered algorithm; for Ed25519 this means any non-zero length other than 64;
- `PRIKK-VERIFY-SIGNATURE-DUPLICATE` for any repeated `K` tuple;
- `PRIKK-VERIFY-SIGNATURE-NONCANONICAL-ORDER` when adjacent tuples are not strictly increasing.

A duplicate necessarily also violates strict increase, but verification emits the duplicate code as
the primary envelope finding and does not need to emit both codes for the same equality. These
warnings do not alter bytes or make a format-1 repository writable. DC-40 owns repository-format
selection and must invoke the same strict validator for every format-2 object and inline RefUpdate
read; DC-39 must expose and test that validator without introducing format 2 itself.

Verification emits at most one issue of each signature-envelope code per envelope, regardless of the
number of offending signatures or adjacencies. Within one envelope, issue order is fixed as malformed,
duplicate, then non-canonical order. An equality contributes to duplicate detection and is skipped for
the order issue; a separate descending adjacency still emits the order issue. Across sources, envelope
diagnostics follow the verifier's deterministic encounter order: object files in canonical object-
type then ObjectId order, active WAL records in sequence order, refs in unsigned lexicographic ref-name
byte order, and each ref log in record order. Tests must pin both per-envelope suppression/order and
cross-source encounter order.

## RefUpdate no-clock contract

For `RefUpdatePayload` schema 1, `created_at == 0` is the canonical no-clock sentinel, not an event-time
claim. It is separate from each envelope signature's advisory `created_at` field.

- Every production schema-1 RefUpdate construction and every mutation path requires zero.
- Format-1 read-only verification preserves a structurally valid non-zero payload and reports the
  existing `PRIKK-VERIFY-REF-LEGACY-TIMESTAMP` warning. The value is never trusted as time.
- Format-2 verification and every mutation reject non-zero before publication.
- Retry reconstructs or reuses the same zero-sentinel payload and must produce the same ObjectId,
  preimage, signature, and envelope bytes.

An authoritative clock requires a later versioned RFC covering timestamp source, retry persistence,
ordering, and trust. DC-39 does not add one.

## Write and verification surface inventory

Implementation must audit every released production writer and direct construction boundary at the
reviewed baseline. The known required surfaces are:

| Surface | DC-39 obligation |
|---|---|
| `prikk-object::ObjectEnvelope::add_signature` | Reject an invalid pre-existing vector; validate shape; reject duplicates; insert canonically by `K`; preserve self on error. |
| `ObjectEnvelope::encode_canonical` / `to_canonical_bytes` | Strict validation before any public canonical envelope bytes are emitted. |
| `prikk-store` file-envelope encoder | Strict validation before bytes are produced for persistence. |
| `FileObjectStore::write_object` | Strict rejection before directory creation or immutable publication. |
| `MemoryObjectStore::write_object` | Same public writer contract for tests and early callers. |
| Active WAL append/record encoding | Strict rejection before append. |
| Ref-log append/record encoding | Strict rejection before append, including retry paths. |
| Ref publication | Zero-sentinel validation plus strict RefState/RefUpdate envelopes before mutation. |
| Worktree and rollback-draft AUTHOR creation | Shared preimage helper followed by canonical envelope insertion. |
| Seal and recovery MAINTAINER creation | Shared preimage helper, zero advisory signature time, and canonical envelope insertion. |
| Object, WAL, and ref-log verification reads | Format-aware legacy diagnostics or strict format-2 rejection without normalization. |

Tests and fixture-only direct construction must be inventoried separately. They may deliberately build
invalid envelopes only to exercise rejection or format-1 diagnostics. If implementation discovers a
production persistence sink not covered here, the inventory and design must be amended and reviewed
before that sink is tightened incidentally.

## Required implementation and tests

- Put canonical comparison/duplicate logic in one `prikk-object` authority and reuse it for add and
  strict validation.
- Keep structural and strict APIs visibly distinct; a default-named validator must not ambiguously
  switch legacy semantics.
- Require exactly 64 bytes for strict Ed25519 admission. Pin lengths 0, 1, 63, 64, and 65: zero fails
  structural and strict validation; 1, 63, and 65 remain structurally readable legacy forms with one
  malformed warning but fail strict validation; 64 passes the shape check.
- Prove public canonical envelope encoding rejects direct duplicate, inversion, and each malformed
  length before emitting any bytes.
- Pin the literal preimage, public key, and signature above in tests outside implementation files.
- Prove each preimage field mutation fails verification and malformed key ids/codes fail before use.
- Cover duplicate tuples with equal and unequal advisory signature timestamps.
- Cover every adjacent-order inversion and deterministic bytes for equivalent accepted signature
  insertion orders.
- Prove `add_signature` rejects every invalid pre-existing vector without mutation and that every
  successful result passes strict validation.
- Cover all persistence sinks in the inventory with rejection-before-mutation evidence.
- Preserve and diagnose format-1 duplicate, non-canonical, non-64-byte non-empty Ed25519, and non-zero-
  RefUpdate fixtures byte for byte; prove they are not admitted to strict mutation.
- Pin at-most-one-per-code diagnostic multiplicity and deterministic code/envelope encounter order
  across object files, active WAL, and ref logs.
- Expose strict validation for DC-40 and prove a simulated format-2 read rejects the same fixtures.
- Prove production AUTHOR and MAINTAINER signing and verification use the shared preimage helper.
- Prove RefUpdate retries preserve the zero sentinel and exact canonical bytes.
- Keep Rust implementation files at or below the project's 300-ELOC limit and keep test modules out
  of implementation files.

## Delivery sequence and evidence

1. Satisfied on 2026-07-22: architect re-review accepted this RFC and companion FDD erratum.
2. Implement shared object-layer authority, persistence enforcement, diagnostics, documentation, and
   the complete surface/test inventory as one bounded candidate.
3. Request implementation review with links to DC-34, this RFC, its companion FDD erratum, the DC-39
   design review and re-review, DC-40 and its companion FDD, and any implementation handoff created
   during the work.
4. After owner commit, obtain clean-checkout/archive post-commit evidence for literal vectors,
   format-1 byte preservation, strict rejection, and all persistence sinks.
5. Keep DC-39 accepted but incomplete until that evidence passes. DC-40 implementation then consumes
   the accepted strict validator for repository-format-aware reads.

## Rollback and failure policy

Before commit, rollback is removal of the bounded candidate. After commit but before release, revert
the complete DC-39 implementation and bookkeeping together; do not retain a writer-only or decoder-
only subset. Existing format-1 bytes are never rewritten as rollback or repair.

A mismatch in the literal vector, a newly discovered production preimage grammar, a persistence sink
that cannot reject before mutation, or any required compatibility exception returns DC-39 to design
review. It is not repaired by weakening strict validation or normalizing historical bytes.

## Non-goals

- No new algorithm, role, object type, key lifecycle, threshold policy, trusted wall clock,
  signature-domain version, or existing-signature migration.
- No change to ObjectId construction, payload canonicalization, repository format selection, or
  DC-40 state-root semantics.
- No broad format-2 repository implementation or production/public-preview/stable-format claim.

## Acceptance criteria

Architect design re-review v1 accepted the byte vector, tuple authority, public serializer boundary,
strict Ed25519 shape, deterministic diagnostics, format-1 compatibility, DC-40 integration boundary,
and no-clock policy on 2026-07-22. Completion still requires the implementation, tests, documentation,
implementation review, and post-commit evidence defined above.
