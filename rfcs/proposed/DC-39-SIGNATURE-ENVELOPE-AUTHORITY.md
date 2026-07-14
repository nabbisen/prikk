# RFC (proposed) - DC-39 Signature and Envelope Authority

**Status.** Proposed; DC-34 is accepted upstream authority.
**Target milestone.** M1 - 0.18.0 corrective release.
**Tracks.** Architect review B5, N1, and N2.
**Touches.** Signature vectors, envelope validation/canonical ordering, RefUpdate time semantics,
current-state docs, and compatibility tests.

## Design

Implement and publish DC-34's exact version-1 signature preimage contract. Add a literal golden preimage
vector and deterministic Ed25519 signature vector covering domain, every numeric field, ObjectId, role,
and key-id length/bytes. Record the intentional FDD-03 erratum in tracked authority.

Strict format-2/new-write envelope validation must reject duplicate signature identity tuples and
require one deterministic order defined by key id, signer role, algorithm, and signature bytes.
Advisory `created_at` must not determine canonical ordering. Strict format-aware decoding of a persisted
format-2 envelope with duplicate or non-canonical signatures fails closed. Adding a signature for a new
write must either produce that canonical order or reject the duplicate. Format-1 diagnostic decoding
follows the bounded exception below.

The duplicate tuple is literally `(key_id bytes, signer_role u16, algorithm u16, signature_bytes)`;
`created_at` is excluded because it is advisory. Ordering compares those same fields in that order,
using unsigned lexicographic byte order for key id and signature bytes and numeric order for codes.

Historical compatibility is format-aware:

- released production AUTHOR and MAINTAINER paths emitted one signature with `created_at == 0`;
- format-1 read-only decoding retains the released structural envelope rules and preserves signature
  order/bytes exactly, including forms constructible through the published object/store libraries;
- format-1 verification checks every signature it can validate and reports duplicate/non-canonical
  ordering and non-zero RefUpdate time as legacy non-canonical diagnostics; it never rewrites them;
- format-2 reads and every new write require the strict duplicate/order rule and the DC-34 timestamp
  rule;
- base structural decoding remains distinct from format-aware canonical validation so a format-1
  repository can be diagnosed without silently accepting its envelope for format-2 mutation.

Implementation review must include an inventory of every released CLI/store writer and direct object
construction surface. If that inventory finds a production writer outside these bounds, compatibility
returns to design review rather than being rejected incidentally by decoder tightening.

For current RefUpdate schema version 1, `created_at == 0` is documented and tested as a no-clock
sentinel. Non-zero values are not emitted by production publication until a later RFC defines clock,
retry persistence, ordering, and trust semantics. Documentation must stop describing the field as an
authoritative creation timestamp.

## Required tests

- literal preimage bytes and deterministic signature verify against the pinned public key;
- one-field changes invalidate the vector;
- duplicate signatures and every non-canonical order are rejected on strict format-2/new-write
  add/decode/validate paths;
- format-1 legacy envelopes are preserved and diagnosed without being admitted to format-2 mutation;
- envelope bytes are deterministic for equivalent accepted signature sets;
- production AUTHOR/MAINTAINER and verification paths use the same preimage helper;
- RefUpdate retries preserve the zero sentinel and exact canonical bytes;
- schema-1 non-zero RefUpdate time follows DC-34's format-1 warning/format-2 rejection rule.

## Non-goals

- No new algorithm, key lifecycle, threshold policy, trusted wall clock, signature-domain version, or
  existing-signature migration.
- No change to ObjectId construction; signatures remain outside the ObjectId preimage.

## Acceptance criteria

The byte contract can be implemented independently from prose, envelope bytes are canonical, and docs
make the no-clock policy explicit. Any required compatibility exception must return to design review.
