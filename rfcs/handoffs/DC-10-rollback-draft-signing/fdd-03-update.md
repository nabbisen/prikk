# DC-10 FDD-03 Update — PatchPurpose

Status: Reviewed DC-10 update  
Related RFC: `../../proposed/DC-10-ROLLBACK-DRAFT-SIGNING.md`  
Target FDD: FDD-03 Object Schema and Canonical Identity

## Purpose

DC-10 adds an identity-bearing Patch payload discriminator so rollback-draft identity no longer lives
in AUTHOR signature key-id space.

## Required FDD-03 Body Updates

### Section 8 — Repeated Field Classification

No repeated-field entry is added. `PatchPurpose` is a scalar optional/defaulted field.

### Section 9.1 — PatchPayload

Add tag 5:

| Tag | Field | Type | Required | Notes |
|---:|---|---|---|---|
| 5 | `purpose` | `enum_u16` | no | identity-bearing; absent means `Normal`; explicit `Normal` is non-canonical and rejected |

Full `PatchPayload` table after the update:

| Tag | Field | Type | Required | Notes |
|---:|---|---|---|---|
| 1 | `operations` | repeated `Operation` | yes | ordered by `op_seq`; at least one |
| 2 | `parent_patch_ids` | repeated `object_id` | no | sorted |
| 3 | `intent` | `enum_u16` | no | advisory only |
| 4 | `preconditions` | repeated `OperationConditionEntry` | no | sorted by key |
| 5 | `purpose` | `enum_u16` | no | identity-bearing; absent means `Normal`; explicit `Normal` is non-canonical and rejected |

### New Enum — PatchPurpose

| Code | Name | Meaning |
|---:|---|---|
| `0x0001` | `Normal` | ordinary Patch; default when tag 5 is absent |
| `0x0002` | `RollbackDraft` | rollback-draft Patch; survives WAL-to-seal persistence and history classification |

Rules:

1. `PatchPurpose::Normal` is the default only when tag 5 is absent.
2. Encoders must omit tag 5 for `Normal`.
3. Decoders must reject a present tag 5 with value `Normal` as non-canonical.
4. Encoders must emit tag 5 for `RollbackDraft`.
5. Unknown `PatchPurpose` codes are rejected until schema evolution explicitly allows them.
6. `PatchPurpose` is identity-bearing payload metadata. It is not advisory intent.

### Canonical Identity Impact

- Existing normal Patch canonical bytes remain unchanged because tag 5 is absent for `Normal`.
- The existing PATCH-framing anchors remain unchanged:
  - empty-PATCH `510ab866a195347da66cada7fcb724a5ed77c4b85cf57345db169324e55d5157`
  - populated `24031b48ef9b5d1a7bdd31fda720c549a727ba9af774c59c8b5278f6c2bcc854`
- A representative `RollbackDraft` Patch must receive a frozen canonical byte/ObjectId vector.

## Required Tests

- absent tag 5 decodes as `PatchPurpose::Normal`;
- explicit tag 5 = `Normal` fails canonical decode;
- tag 5 = `RollbackDraft` round-trips and changes ObjectId relative to equivalent normal payload;
- frozen RollbackDraft vector remains stable;
- normal Patch anchors remain exactly unchanged.
