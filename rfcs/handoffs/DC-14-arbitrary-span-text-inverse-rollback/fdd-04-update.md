# DC-14 FDD-04 Update - Rollback Draft Text Inverse Security Notes

Status: Accepted for v0.7.0 implementation after design re-review v1
Related RFC: `../../done/DC-14-ARBITRARY-SPAN-TEXT-INVERSE-ROLLBACK.md`
Target FDD: FDD-04 Threat Model

## Purpose

DC-14 exposes deterministic arbitrary-span `EditText` inverse payloads through rollback preview and
rollback draft surfaces. It also confirms the DC-10 rollback-draft marker replacement while keeping
rollback authorization, rollback refs, worktree rollback mutation, and AUTHOR trust-store enforcement
out of scope.

## Required FDD-04 Body Updates

### Rollback Draft Authority

Rollback-draft authority for DC-14 production logic is:

- Patch payload purpose is `PatchPurpose::RollbackDraft`;
- rollback-draft verification rules pass;
- the Patch envelope carries a real role-bound Ed25519 AUTHOR signature record.

Reserved AUTHOR key ids, placeholder signatures, marker signatures, or hash markers are not
rollback-draft authority. Production verification must not accept an old marker-key shortcut as a valid
DC-14 rollback draft.

### AUTHOR Signature Boundary

DC-14 does not add AUTHOR trust-store enforcement or rollback authorization. Without a repository-local
AUTHOR public-key authority or another supplied public-key source, rollback-draft verification must not
claim policy verification or full cryptographic trust validation of arbitrary historical AUTHOR
signatures.

The DC-14 verification boundary must still reject:

- missing signature records;
- wrong signer role;
- wrong signature algorithm;
- malformed signature records, including Ed25519 signature payloads whose length is not 64 bytes;
- placeholder or marker signatures;
- payload purpose mismatch.

### Text Inverse Integrity

Rollback-draft verification must compare the derived inverse `PatchPayload` canonical bytes. It must
not compare only operation summaries, replacement strings, or semantic final text. Generated inverse
`EditText` records must have absent presentation hints, recomputed anchors, recomputed duplicate index,
and recomputed `span_id` from post-forward text.

## Required Security Tests

- normal-purpose Patch with byte-identical inverse operations is rejected;
- rollback-draft Patch without `PatchPurpose::RollbackDraft` is rejected;
- rollback-draft Patch using old marker-key authority is rejected or treated only as a legacy/internal
  artifact, not as a valid DC-14 draft;
- generated rollback drafts use the real AUTHOR signing path;
- missing, wrong-role, wrong-algorithm, malformed Ed25519 length, placeholder, and marker signatures are
  rejected;
- stale inverse anchors, stale inverse `span_id`, and non-absent generated inverse presentation hints
  are rejected during rollback-draft verification.
