# RFC 116 N3 — the recognition claim carries its block's parents: implementation handoff

**Design:** `rfcs/handoffs/116-sync-negotiation/design-v1.md` **§3 (N3). Read it in full — including
§3.2 on why this must land before any claim producer exists.**
**RFC:** `rfcs/accepted/116-sync-negotiation-and-transport.md` (ACCEPTED 2026-08-20).
**Base:** current `main`. **Must land before RFC 116's negotiation artifacts, and before anything that
constructs a `RecognitionClaimPayload`.**

**This is the same shape as the D6 amendment (`e4ad639`), for the same reason, on the same object.**
Read that handoff and its review if you have not — the technique, the hazards, and the "zero frozen
bytes move" argument all carry over.

---

## 1. Why now, and why it is free exactly once more

**Verified while designing:** there is still **no production path that constructs a
`RecognitionClaimPayload`.** The only non-test construction site in the workspace remains
`prikk-object/src/vectors.rs:151`, the Gate A snapshot generator. Stage 4 *consumes* claims; it does not
create them. No release has ever written one.

So this is again a free amendment in `schema_version` **1** — no version 2 — and **zero frozen bytes
move**, because `repeated_object_id` over an empty list writes nothing and the frozen vector
`recognition_claim_populated` describes a block with no parents.

**The first real producer is RFC 116's sender side**, two increments away. After that, this same change
becomes a schema 2 with two contracts to carry forever. **This is the last time the window is open.**

## 2. The problem this solves

A claim carries `block_id` and `patch_ids` and **nothing about its block's parents**, while
`seal_from_accepted_claim` seals **one claim per call**. So a delta spanning two blocks leaves the
receiver holding two claims with **no way to derive which to seal first** — and the wrong order either
fails to apply or builds a different history.

RFC 115's tests never hit this because they are single-block. Real sync hits it immediately.

## 3. The change

Add field **3** to `RecognitionClaimPayload`:

```
parent_block_ids: Vec<ObjectId>   // the block's own parent_block_ids, VERBATIM
```

**Verbatim — not sorted, not deduplicated**, exactly as D6 made `patch_ids` verbatim. `BlockPayload`
imposes no order or uniqueness invariant on `parent_block_ids`, and the claim mirrors the block.
**May be empty** — a root block has no parents, and an empty list is the correct, common case, not a
degenerate one. **Do not add a non-empty guard**; `patch_ids`' non-empty guard stays as it is.

### 3.1 Sites

| File | Change |
|---|---|
| `prikk-object/src/payload/recognition_claim.rs:62-63` | `encode_canonical`: add `writer.repeated_object_id(3, &self.parent_block_ids)?` **after** tag 2 |
| `…:78-88` | `decode_canonical`: accept tag `3`; tag 3 currently falls into the unknown-tag refusal |
| `…` struct + docs | new field, documented as the block's parents verbatim, and why |
| `…` | bound the declared parent count the same way `RECOGNITION_CLAIM_MAX_PATCH_IDS` bounds tag 2 — a per-push check, refused the instant the limit-plus-one entry would be read |

**Keep:** the `patch_ids` non-empty guard, the unknown-tag refusal for tags **other than** 1-3, and the
existing count bound.

## 4. The consistency check must say *which* field disagreed

`check_recognition_claim_consistency` now compares **both** fields against a held block, each by exact
sequence equality.

**But `Contradicted { claimed, actual }` currently names only `patch_ids`.** If a parent mismatch is
reported through those fields, the diagnostic says "your patches disagree" when the patches are fine —
**a misleading diagnostic of exactly the class Stage 4's divergence-vs-corruption ruling exists to
prevent.** A wrong explanation is worse than a vague one, because it sends the reader somewhere false.

**Required: `Contradicted` names the disagreeing field.** Either two variants or a field discriminator —
your choice of shape, but the outcome must let a caller distinguish "the claim lies about which patches"
from "the claim lies about which parents" without parsing a string. Update the doc comments on
`Consistent` and `Contradicted`, which currently describe `patch_ids` alone.

**The correctness argument is unchanged and still holds for the new field:** a claim names a `block_id`;
blocks are content-addressed; the same block id means the same canonical payload, therefore the same
`parent_block_ids` sequence. **So sequence equality on parents can only detect a lie, never manufacture
one** — the same reasoning D6's amendment carried into the module doc. Extend that doc rather than
duplicating the argument.

## 5. Gate A

- **The existing vector `recognition_claim_populated` must not move.** Its block has no parents, and an
  empty repeated field writes no bytes. `snapshot.txt` must show **only additions**.
- **Add a vector for a claim *with* parents** — otherwise the new field ships frozen by nothing. A new
  row is fine; an existing row changing is a **stop-work finding**, per `snapshot.rs`'s own header.
  `PRIKK_REGEN=1` is not the tool for this increment.
- `rfc114_vector_13_...` must still pass untouched.

## 6. Out of scope

- **The topological sort over claims.** RFC 116 stage 2; this only makes it possible.
- **The negotiation artifacts** `PSYNCSU1` / `PSYNCHV1`. Stage 2.
- **Any sender side / claim producer.** Stage 3, and deliberately after this.
- **Stage 4's `seal_from_accepted_claim` signature.** It still takes one claim; ordering *across* claims
  is stage 2's problem, not a change to that function here.
- **`patch_ids`' own contract.** Untouched by this.

## 7. Tests and controls

Every behavioural claim needs a control that was **observed failing**:

| Property | Control |
|---|---|
| Parents round-trip verbatim, order preserved, duplicates preserved | Sort them in `encode_canonical` → round-trip test fails |
| An empty parent list round-trips and is *not* an error | Add a non-empty guard → the root-block test fails |
| A claim whose parents disagree with a held block is `Contradicted` **naming the parent field** | Compare only `patch_ids` → the parent-mismatch test fails |
| A patch mismatch still reports as a *patch* mismatch | Swap the field discriminator → the patch-mismatch test fails |
| An over-limit declared parent count is rejected before allocating | Remove the per-push bound → refused too late |
| Identity is untouched | `snapshot.txt` shows **only** additions; vector 13 passes |

Rows 3 and 4 are the pair that matters — either alone passes with the discriminator unimplemented.

## 8. What to report

1. **Whether any pre-existing `snapshot.txt` row changed.** It must not. If one did: **stop, escalate,
   do not regenerate.**
2. Control output for each row of §7 — actual failure text, and the single line mutated.
3. The **full gate set against the exact commit, after the last edit**: `cargo fmt --all -- --check`;
   `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
   `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`; `git diff --check`;
   `cargo audit --no-fetch`; release-policy `check` / `boundary-check` / `reference-check`.
   Cross-target clippy pair only if this diff contains `#[cfg(target_os)]` — check this diff.
4. Test counts before and after, per crate.
5. **Confirmation that no claim producer was introduced** — this increment must not create one, or it
   closes the window it exists to use.
6. Anything here that turned out to be wrong. **Say so plainly.**

**Stop and escalate, do not guess**, if: any existing `snapshot.txt` row moves; vector 13 fails; the
canonical writer rejects tag 3 following repeated tag 2; or §4's field-discriminator shape conflicts
with how `RecognitionClaimConsistency` is consumed in `accept.rs` or `seal_from_accepted.rs`.
