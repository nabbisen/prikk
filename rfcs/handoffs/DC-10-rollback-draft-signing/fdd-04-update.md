# DC-10 FDD-04 Update — Rollback Draft Signing Threat Model

Status: Reviewed DC-10 update  
Related RFC: `../../proposed/DC-10-ROLLBACK-DRAFT-SIGNING.md`  
Target FDD: FDD-04 Threat Model

## Purpose

DC-10 changes rollback-draft identity and signing. It closes the v0.2.0 residual where rollback
drafts used `dev-placeholder-rollback-author` as both a fake AUTHOR signature and a rollback marker.

## Required FDD-04 Body Updates

### Section 5.1 — Object Identity Fork

Add `PatchPurpose` to the object-identity controls:

- `PatchPurpose` is identity-bearing Patch payload metadata.
- `PatchPurpose::Normal` is canonical only by omission; an explicitly encoded default is rejected.
- `PatchPurpose::RollbackDraft` is encoded at PatchPayload tag 5 and survives WAL-to-object
  persistence, allowing active and sealed rollback classification without inspecting signer key ids.

Threat addressed:

- two byte representations of the same logical normal Patch are rejected by the explicit-default
  canonical rule;
- rollback identity is part of designed payload identity, not a convention over a key-id string.

### Section 5.9 — Signature Replay and Role Confusion

Add rollback-draft signing control:

- rollback-draft Patches are signed through the same role-bound Ed25519 AUTHOR preimage as worktree
  commit Patches;
- rollback-draft identity is not encoded in `Signature.key_id`;
- verification/classification must not treat reserved AUTHOR key IDs as rollback identity;
- `Signature.created_at` remains advisory only.

Residual closed:

- the v0.2.0 rollback-draft fake AUTHOR marker is removed from production identity logic.

### Section 5.11 — Worktree/Authoring and Rollback Draft Residuals

If FDD-04 v1.4 contains the worktree-authoring residual table, update the rollback-draft residual:

| Residual | v0.2.0 State | DC-10 State |
|---|---|---|
| Rollback-draft AUTHOR marker is not a real signature | Internal, non-publishable scaffold using `dev-placeholder-rollback-author` | Closed for new rollback drafts: identity is `PatchPurpose::RollbackDraft`; AUTHOR signature is real Ed25519 |

Add a compatibility note:

- pre-DC-10 rollback-marked Patches are pre-stability development artifacts;
- production code does not carry legacy recognition for `dev-placeholder-rollback-author`;
- old sealed rollback Blocks may stop being classified as rollback Blocks by `log`/`verify`.

## Required Security Tests

- rollback-draft AUTHOR signature verifies with the supplied Ed25519 public key;
- the signature fails if object id, signer role, or key id changes;
- rollback classification is independent of AUTHOR key id;
- `dev-placeholder-rollback-author` does not appear in production Rust code;
- rollback-draft append holds one active-session lock across empty-WAL/partial-tail checks and WAL append.
