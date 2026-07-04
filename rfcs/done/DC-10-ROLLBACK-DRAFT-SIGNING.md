# RFC (done) — DC-10 Rollback Draft Identity and AUTHOR Signing

**Status.** Implemented (v0.3.0)
**Target release.** v0.3.0.
**Tracks.** Removing the rollback-draft fake AUTHOR-signature marker so every AUTHOR-signature-bearing
Patch produced by production commands is signed with real role-bound Ed25519 key material.
**Touches.** `prikk-object` Patch payload schema, rollback draft append/verify/classification,
CLI signer plumbing, docs, release notes, and signature/identity regression tests.
**Companion FDD updates.** `../handoffs/DC-10-rollback-draft-signing/fdd-03-update.md`,
`../handoffs/DC-10-rollback-draft-signing/fdd-04-update.md`.

## Context

v0.2.0 made node-addressed `prikk commit` patches carry real role-bound Ed25519 AUTHOR signatures.
One exception remains: `prikk rollback-draft --append-inverse` writes an inverse Patch envelope with
the key id `dev-placeholder-rollback-author`. That value is not only a fake signature; it is also the
marker used by active rollback verification and sealed rollback history classification.

This blocks a clean project-level claim that AUTHOR Patch signatures produced by production commands
are real Ed25519 signatures. Replacing the marker with a real key without adding a separate rollback
identity field would erase rollback-draft classification.

## Design goal

Separate rollback-draft identity from AUTHOR signatures:

1. rollback-draft Patch objects must be identifiable without inspecting signer key ids;
2. rollback-draft Patches must carry real role-bound Ed25519 AUTHOR signatures;
3. normal Patch identity bytes and existing PATCH-framing anchors must not drift;
4. rollback-draft verification and sealed rollback classification must keep working before rollback
   refs, authorization policy, or audit plugins exist.

## Proposed design

Add a non-advisory Patch payload discriminator named `PatchPurpose` at `PatchPayload` canonical
field tag **5**.

```text
PatchPurpose:
  Normal = 1
  RollbackDraft = 2
```

Rules:

- `PatchPurpose::Normal` is the default when the field is absent.
- Canonical encoding omits the field for `Normal`, preserving existing normal Patch identity bytes.
- Canonical encoding writes the field only for non-default purposes such as `RollbackDraft`.
- Canonical decoding rejects an explicitly encoded tag-5 `Normal` value as non-canonical. There must
  be exactly one canonical byte representation for a normal Patch.
- `PatchPurpose` is not advisory intent. It is identity-bearing payload metadata used by verification,
  history classification, and future policy checks.
- The existing `intent` field remains display/advisory-only and must not be used for rollback identity.

`rollback-draft` construction changes:

1. derive and validate the supported inverse Patch payload as today;
2. set `PatchPurpose::RollbackDraft` on that payload;
3. build the unsigned Patch object ID from that canonical payload;
4. sign through the same `AuthorSigner` boundary used by worktree `commit`;
5. append the signed Patch envelope to the active WAL.

`rollback-draft` identification changes:

- `is_rollback_draft_envelope` must decode the Patch payload and check `PatchPurpose::RollbackDraft`.
- It must no longer inspect AUTHOR key ids for rollback identity.
- The old `dev-placeholder-rollback-author` marker becomes invalid for newly authored rollback drafts.

CLI signer input:

- `prikk rollback-draft --append-inverse` uses `PRIKK_AUTHOR_KEY_ID` and `PRIKK_AUTHOR_SEED`, matching
  `prikk commit`.
- This is still minimal key input, not a trust store.
- Repository-wide trust verification remains out of scope until the publication-grade trust phase.
- `rollback-draft-verify` reports the real AUTHOR key id carried by the draft signature for operator
  review.

## Rejected alternatives

### Keep using the AUTHOR key id as marker

Rejected. It preserves the current exception and keeps rollback identity coupled to fake or reserved
signer names.

### Use `intent`

Rejected. Requirements and external design treat intent as advisory only. Rollback classification and
future policy are not advisory display hints.

### Use a WAL record kind only

Rejected for this increment. It would identify active rollback drafts but lose the marker after seal,
unless seal copied the information elsewhere. Sealed rollback history currently classifies persisted
Patch objects, so the identity must survive object persistence.

### Create a new object type instead of a Patch

Rejected for now. Rollback draft replay, inverse comparison, seal, and history already operate over
Patch payloads. A new object type would expand storage/reachability rules more than needed.

## Compatibility and identity rules

- Existing normal Patch objects remain decodable as `PatchPurpose::Normal`.
- Existing normal Patch canonical bytes must remain unchanged.
- An explicit encoded default `PatchPurpose::Normal` is rejected as non-canonical, even though absent
  purpose decodes as `Normal`.
- The two PATCH-framing anchors remain frozen:
  - empty-PATCH `510ab866...5157`
  - populated `24031b48...c854`
- A representative `PatchPurpose::RollbackDraft` Patch receives its own frozen canonical vector and
  ObjectId before implementation is considered complete.
- Newly authored rollback drafts receive new Patch IDs because `PatchPurpose::RollbackDraft` is part
  of their canonical payload.
- Previously sealed rollback-marked Patches using `dev-placeholder-rollback-author` are pre-stability
  development artifacts. v0.3 makes a clean break: production classification does not recognize the
  old marker, so old sealed rollback Blocks may stop being reported as rollback Blocks by `log` and
  `verify`.
- Any old-marker compatibility fixture must be clearly test-only. Production code must not carry
  `dev-placeholder-rollback-author` as a rollback identity rule.
- Sealed rollback classification now decodes candidate Patch payloads to read `PatchPurpose` instead
  of doing the previous envelope/key-id check. This is acceptable because `verify` is already the
  deep validation path, but it is a real behavior/cost change.

## Implementation plan

1. Update FDD-03 to register `PatchPayload` tag 5 as `PatchPurpose`, including omit-default and
   reject-explicit-default canonical rules.
2. Update FDD-04 to close the rollback-draft fake AUTHOR marker residual and record
   `PatchPurpose` as identity-bearing designed metadata. This is a threat-model update, not a
   post-hoc documentation note.
3. Re-review the amended RFC and FDD updates before opening the implementation gate.
4. Add `PatchPurpose` to `prikk-object::PatchPayload`, with decode support for absent-as-`Normal` and
   rejection of explicit default `Normal`.
5. Pin vectors proving normal Patch identity is unchanged and a representative rollback-draft purpose
   Patch has a frozen canonical byte/ObjectId vector.
6. Change rollback draft append to accept an `AuthorSigner` and set `PatchPurpose::RollbackDraft`.
7. Change CLI rollback-draft to build the AUTHOR signer from env vars.
8. Change rollback verification and sealed history classification to use `PatchPurpose`, not key id.
9. Remove production reliance on `dev-placeholder-rollback-author`.
10. Confirm rollback-draft keeps the DEC-008 active-WAL guard+append invariant: the active-session
    lock is held across empty-WAL check, partial-tail check, and append.
11. Update docs, `README.md`, `ROADMAP.md`, `CHANGELOG.md`, and implementation status.
12. Cut v0.3.0 only after local gates and release checks pass.

## Test gates

Required tests:

- normal Patch golden anchors unchanged;
- `PatchPurpose::Normal` absent field decodes as normal;
- explicitly encoded tag-5 `PatchPurpose::Normal` is rejected as non-canonical;
- rollback-draft Patch payload encodes `PatchPurpose::RollbackDraft`;
- a representative `PatchPurpose::RollbackDraft` canonical byte/ObjectId vector is frozen;
- rollback-draft append produces a real Ed25519 AUTHOR signature through an injected signer;
- signature verification fails if object id, role, or key id changes;
- active rollback-draft verify classifies by payload purpose, not key id;
- sealed rollback history classification still counts rollback Blocks/Patches;
- sealed rollback history no longer classifies pre-DC-10 fake-key-id rollback artifacts in production
  logic;
- rollback-draft append holds one active-session lock across WAL guard and append;
- `grep -rn "dev-placeholder-rollback-author\b" crates/ --include=*.rs` is empty, except any test-only
  backward-compatibility fixture explicitly named as such.

Standard gates:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Out of scope

- publication-grade MAINTAINER signing;
- trust-store enforcement, key revocation, key rotation, and signature policy;
- rollback-specific refs and rollback authorization;
- audit/plugin approval;
- worktree rollback mutation;
- arbitrary-span text rollback;
- commutation, confluence, conflict witnesses, and remote sync.

## Open questions before implementation

None. Architect review rulings:

- tag number: `PatchPayload` tag 5;
- legacy marker compatibility: clean break; pre-stability artifact only;
- `rollback-draft-verify` should report the real AUTHOR key id.
