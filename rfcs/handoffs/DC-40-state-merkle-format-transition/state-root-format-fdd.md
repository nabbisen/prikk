# DC-40 State-Root and Repository-Format FDD

**Status.** Accepted companion authority inherited from DC-40 after architect re-review on 2026-07-14;
implementation complete at `70c3902` after post-commit evidence acceptance on 2026-07-23. Remains
accepted with its parent until the 0.18.0 release.
**Parent RFC.** `../../done/DC-40-STATE-MERKLE-FORMAT-TRANSITION.md`.

## Scope

This FDD fixes the byte grammar for Block-v2 state roots and the 0.18.0 format-1/format-2 command
boundary. It does not define history-preserving migration, stable repository format, remote exchange,
or a general directory object model.

## Canonical clean state

A clean state is the complete set of live nodes after authoritative replay of a Block's parent state
and ordered Patch operations. Tombstones, Patch ids, Block ids, cache entries, snapshot manifests, and
implicit parent directories are not entries.

Each live entry contains:

- canonical ASCII `RepoPath` bytes under the exact grammar below;
- one non-zero 32-byte NodeId;
- NodeKind code: text `0x0001`, binary `0x0002`, symlink `0x0003`;
- normalized mode as unsigned big-endian `u32`; symlink mode is zero;
- file content as the 32-byte Blob ObjectId, or symlink content as validated UTF-8 target bytes.

Entries are strictly ordered by unsigned lexicographic path bytes. Duplicate paths or NodeIds, invalid
paths/kinds/modes, missing or wrong-kind persistently referenced Blobs, invalid symlink state, and
replay disagreement are integrity failures. For text produced by `EditText`, authoritative replay
reconstructs the resulting bytes and recomputes the canonical text Blob ObjectId; a separately stored
result Blob is not required unless another persisted object references it. Two states are equivalent
only when every listed field is identical. Patch identity does not affect equivalence; NodeId is state
identity and therefore does.

### Format-2 RepoPath grammar

A format-2 path is the exact byte string accepted by the released `prikk-replay::RepoPath` grammar:

- non-empty ASCII, relative, slash-separated bytes;
- no leading `/`, backslash, colon, byte below `0x20`, or byte `0x7f`;
- every slash-delimited component is non-empty, is neither `.` nor `..`, and does not end in space or
  dot;
- the first component is not `.prikk` under ASCII case-insensitive comparison;
- the component base before its first dot is not, under ASCII case-insensitive comparison, `CON`,
  `PRN`, `AUX`, `NUL`, `COM1` through `COM9`, or `LPT1` through `LPT9`;
- across the complete live-entry set, no two paths are byte-equal and no two paths have equal
  `to_ascii_lowercase()` bytes.

Path bytes are stored and hashed exactly as accepted. There is no Unicode normalization, separator
conversion, case normalization, or host-path reinterpretation.

### Format-2 mode and symlink validity

Text and binary files admit exactly two identity-valid modes: regular `0o100644` and executable
`0o100755`. Worktree authoring maps any Unix executable bit to `0o100755`, maps other regular files to
`0o100644`, and defaults to `0o100644` where no executable-bit source exists. Replay of any other file
mode is invalid for a Block-v2 state root. Symlink mode is exactly zero.

A schema-1 symlink target is opaque Rust/Unicode UTF-8 string state. Every byte sequence represented by
the decoded schema-1 `String` is identity-valid, including empty, absolute-looking, traversal-looking,
or control-containing text; no path normalization is applied. This rule preserves existing identity
bytes, does not declare the target safe, and does not authorize symlink materialization. A later static
target-safety RFC must version or constrain new authoring without reinterpreting Block-v2 roots.

## Leaf bytes

Literal domains contain exactly the displayed ASCII bytes and no terminator.

```text
LEAF_DOMAIN = "PRIKK-STATE-LEAF-v2"
NODE_DOMAIN = "PRIKK-STATE-NODE-v2"
ROOT_DOMAIN = "PRIKK-STATE-ROOT-v2"
```

For each entry, construct:

```text
leaf_preimage =
  LEAF_DOMAIN
  || u32be(path_byte_length)
  || path_bytes
  || node_id[32]
  || u16be(node_kind)
  || u32be(normalized_mode)
  || u64be(content_byte_length)
  || content_bytes

leaf_hash = SHA256(leaf_preimage)
```

For text/binary files, `content_byte_length` is 32 and `content_bytes` is the Blob ObjectId. For
symlinks, mode is zero and content is the exact opaque schema-1 UTF-8 target bytes. Length overflow,
unknown codes, zero NodeId, and non-canonical values are rejected before hashing.

## Merkle reduction

Start with leaf hashes in canonical path order. At each level, hash adjacent pairs as:

```text
parent = SHA256(NODE_DOMAIN || left[32] || right[32])
```

When a level has an odd final hash, promote that hash unchanged to the next level; do not duplicate it.
Continue until one hash remains. The final root is:

```text
non_empty_root = SHA256(ROOT_DOMAIN || u64be(entry_count) || top_hash[32])
empty_root     = SHA256(ROOT_DOMAIN || u64be(0))
```

The Block payload carries this final 32-byte value. Golden fixtures must publish complete preimages,
intermediate hashes, and final roots for empty, one-, two-, three-, and nested-path entry sets.

## Schema and format binding

Format 1 accepts only historical envelope schema 1 under released read-only decoding, including Block
schema 1 scaffold semantics. Format 2 applies this exact type allowlist:

| Object type/code | Format-2 envelope schema and authority |
|---|---|
| Patch `0x0001` | Schema 1 allowed as identity-bearing replay input. |
| Block `0x0002` | Schema 2 required; schema 1 rejected. |
| RefState `0x0003` | Schema 1 allowed as identity-bearing ref state. |
| RefUpdate `0x0004` | Schema 1 allowed only inline in the ref log under DC-34/DC-39 rules; not an object-store file. |
| Tag `0x0005` | Schema 1 decodable/storable, but no production tag publication command is authorized. |
| Attestation `0x0006` | Schema 1 decodable/storable; current seal policy does not require or publish it. |
| Blob `0x0007` | Schema 1 allowed as identity-bearing content/snapshot evidence. |
| BlockSummaryCache `0x0008` | Schema 1 cache-only and non-authoritative; rejected from object-store/ref identity positions. |
| RecoveryNote `0x0009` | Rejected/reserved; no accepted format-2 persistence or repair authority. |
| ProjectGenesis `0x000a` | Rejected/reserved; no accepted format-2 persistence path. |

Unknown object types or any schema not listed above are rejected. Format 1 never writes Block schema 2.
A Block-v2 Root has zero parents. A Block-v2 Normal has exactly one Block-v2 parent. Merge, Repair, and
Import kinds are rejected in format 2. Cross-format or v1-to-v2 parent edges are rejected.

Block-v2 canonical payload fields retain the ordered TLV framing and use: tag 1 repeated parent
ObjectId (`0x12`, exactly 32 bytes), tag 2 BlockKind enum-u16 (`0x05`, exactly 2 bytes), tag 3 repeated
Patch ObjectId (`0x12`, exactly 32 bytes), tag 4 state root bytes (`0x11`, exactly 32 bytes), and
optional tag 5 snapshot Blob ObjectId (`0x12`, exactly 32 bytes). Every field is framed as
`u16be(tag) || u8(wire_type) || u64be(value_length) || value`. Parent ids are sorted/unique; Patch ids
remain in semantic order. Root/Normal kind and parent cardinality are validated before state replay.

Replay-derived text content uses the schema-1 Blob identity grammar even when no result Blob file is
persisted. Its ObjectId is:

```text
SHA256(
  "PRIKK-OBJECT-ID-v1"
  || u16be(0x0007)
  || u32be(1)
  || u64be(canonical_blob_payload_length)
  || canonical_blob_payload
)
```

For text bytes `content`, the canonical schema-1 Blob payload is exactly:

```text
u16be(1) || u8(0x05) || u64be(2)              || u16be(0x0001)
u16be(2) || u8(0x11) || u64be(content_length) || content
u16be(3) || u8(0x04) || u64be(8)              || u64be(content_length)
```

Here `0x05` is enum-u16, `0x11` is bytes, `0x04` is u64, and BlobKind Text is `0x0001`.
`content_length` must fit `u64`. Persisted binary or text Blob references must resolve to a schema-1
Blob whose kind/content recompute the referenced id; binary uses BlobKind `0x0002` in the same field-1
grammar.

New `prikk init` writes `.prikk/FORMAT` value `2`. Opening value `1` selects legacy read-only mode;
opening value `2` selects current mode; every other value is unsupported. The selected repository mode
is passed to format-aware envelope validation rather than inferred from an individual object alone.

## State derivation and verification

For each Block v2, verification starts with empty state for Root or the one verified parent state for
Normal, resolves every Patch and referenced Blob, applies operations in canonical sequence through the
authoritative replay engine, constructs the canonical live-entry set, and recomputes the root. Merge,
Repair, Import, zero/multiple-parent Normal, and parent-bearing Root Blocks are rejected before replay.
Verification rejects missing evidence, replay failure, cache-only evidence, or root mismatch. A
lifecycle cache or snapshot may be used only after its content is checked against the same authoritative
replay result.

## Format-1 command compatibility in 0.18.0

| Command/surface | Format-1 behavior |
|---|---|
| Repository open | Succeeds in explicit legacy read-only mode. |
| `log`, `status`, history inspection | Allowed with a legacy-format warning; no repository mutation. |
| `worktree-status` | Allowed with a legacy-format warning; no repository mutation. |
| `verify` | Performs bounded structural/signature checks, reports scaffold roots as unverifiable state commitments, and returns non-zero. |
| `doctor` without repair | Diagnostic only; reports legacy mode and any recoverable 0.17.7 publication state. |
| Doctor repair flags | Refused. |
| Checkout/patch/snapshot plan, inverse plan, rollback preview, merge evidence/plan | Read-only planning allowed with a legacy-format warning. |
| `rollback-draft-verify` | Allowed with a legacy-format warning; verification remains bounded by format-1 scaffold limitations. |
| Checkout/worktree materialization or deletion | Refused because state-root verification is unavailable. |
| `commit`, ordinary `seal`, rollback-draft append, trust mutation | Refused before any write. The sole `seal` exception is DC-34's exact signer-backed one-record-ahead legacy publication completion. |
| `init` against an existing format-1 repository | Refused; init never upgrades in place. |

There is no 0.18.0 history-preserving migration. The documented writable path is a newly initialized
format-2 repository followed by deliberate worktree re-authoring, producing new NodeIds, objects,
signatures, and history. Copying `.prikk/` data or editing `FORMAT` is forbidden.

## Required vectors and compatibility tests

- literal leaf/root vectors for every NodeKind, mode, path depth, empty state, and odd/even leaf count;
- same entries from different Patch ids produce the same root; any NodeId change changes the root;
- every committed field mutation changes the affected leaf/root;
- missing/wrong-kind persistently referenced Blob, incorrect replay-derived text Blob identity, and
  invalid symlink/path/mode fail before hashing;
- Block-v1 parent, mixed schema, format-marker/schema disagreement, and unknown format fail closed;
- Root/Normal cardinality and rejection of Merge/Repair/Import have payload and end-to-end vectors;
- every registered ObjectType follows the explicit format-2 allowlist;
- every RepoPath grammar rule, ASCII case-fold collision, valid/invalid mode, and opaque UTF-8 symlink
  target boundary has a vector;
- every format-1 command row has an end-to-end acceptance/refusal test;
- new format-2 repositories seal and verify Root plus descendant Block-v2 state;
- format-1 bytes, signatures, and scaffold roots are never rewritten or represented as real roots.

## Review gate

Architect review must accept this byte grammar, schema allowlist, parent rule, state-derivation source,
and command matrix before DC-40 implementation. Any byte-level change after acceptance requires an RFC/
FDD amendment and new vectors before coding.
