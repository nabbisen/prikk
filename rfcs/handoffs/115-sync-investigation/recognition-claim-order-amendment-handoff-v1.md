# RFC 115 — amend the recognition claim to carry block order verbatim: implementation handoff

**Design:** `rfcs/handoffs/115-sync-investigation/design-v1.md` **§11 (D6). Read it in full — it is the
ruling this handoff implements, including why the change is made in `schema_version` 1 rather than a
version 2.**
**Investigation:** `.git-exclude/reviewed/RFC-115-stage-4-ordering-investigation-v1.md`.
**Base:** current `main`. **Precedes RFC 115 Stage 4, and must land before it.**

**This is a small increment with an unusual property: it relaxes an invariant a previous review made a
condition of acceptance, and inverts the meaning of two shipped tests.** Read §4 before touching the
tests, or you will "fix" them back.

---

## 1. Why this is urgent rather than merely desirable

**The window closes at the next release, not the next increment.** Today no release contains the
recognition claim (latest tag `0.22.1` precedes all three RFC 115 merges) and **no production path
constructs one** — the only non-test construction site in the workspace is
`prikk-object/src/vectors.rs:151`, the Gate A snapshot generator. So no user-held claim exists and none
can, and RFC 114's promise — *"every object any prior release wrote"* — is not engaged.

The moment a release ships a claim **producer**, this same change becomes a genuine schema 2 with two
contracts to carry forever. Stage 4 wires up the first producer. **Hence: this increment, then Stage 4.**

## 2. The change

`RecognitionClaimPayload.patch_ids` becomes **the block's `patch_ids` verbatim** — not sorted, not
deduplicated. `Block.patch_ids` has no sorted-or-unique invariant (verified: nothing in
`block_state.rs` or `payload/block.rs` sorts or dedups it); it is a free sequence consumed in order by
`apply_candidate_patches`, and the claim mirrors it exactly.

`schema_version` **stays 1**. There is no version 2.

### 2.1 Sites to change — the complete list

| File:line | Today | After |
|---|---|---|
| `prikk-object/src/payload/recognition_claim.rs:45` | `encode_canonical` refuses unsorted | guard removed |
| `…/recognition_claim.rs:95` | `decode_canonical` refuses unsorted | guard removed |
| `…/recognition_claim.rs:38` | doc: "Sorted, deduplicated, non-empty" | doc: verbatim block order, non-empty |
| `prikk-store/src/recognition_claim.rs:64` | refuses an unsorted claim | **removed — see §4.1** |
| `prikk-store/src/recognition_claim.rs` (`actual.sort_unstable(); actual.dedup();`) | block side normalized before comparing | **removed — comparison becomes exact sequence equality** |

**Keep unchanged:** the non-empty check, the unknown-field-tag rejection, and the
`RECOGNITION_CLAIM_MAX_PATCH_IDS` per-push bound. **Duplicates become permissible**, mirroring
`Block.patch_ids`.

If `is_strictly_sorted` ends up unused in either file, drop the import; do **not** delete the shared
helper — Stage 1's `compute_patch_set_digest` still depends on it and must keep its own refusal.

## 3. The frozen bytes must not move — and they will not

`encode_canonical` **refuses** unsorted input rather than normalizing it, so removing the guard changes
no bytes for any already-sorted payload. The frozen vector `recognition_claim_populated` carries ids
`0x11…, 0x22…, 0x33…` ascending, which remains a valid — and byte-identical — payload under the new
contract.

**Expected outcome: `crates/prikk-object/src/vectors/snapshot.txt` has no diff at all, and
`rfc114_vector_13_recognition_claim_schema_1_identity_and_signature` passes untouched.**

**If either moves, stop and escalate.** A snapshot diff here would mean the change altered identity,
which it must not — and `snapshot.rs`'s own header already says a drift during an
identity-preserving increment is a stop-work finding, never a regeneration trigger. `PRIKK_REGEN=1` is
not the tool for this increment.

## 4. The two shipped tests whose meaning inverts

**Read this section before editing any test. Both of these were correct when written; the ruling they
encoded has changed.**

### 4.1 `claim_with_unsorted_patch_ids_is_refused_not_compared` — delete it

`crates/prikk-store/src/recognition_claim/tests.rs:89`. It exists because review v1 §2 made that
refusal a condition of Stage 2's acceptance. **D6 §11.7 withdraws that condition**: under a
verbatim-order contract, unsorted is the normal, correct case, and the refusal would reject every
truthful claim.

Replace it with the positive property: **a claim carrying a block's patches in the block's own order
round-trips through encode/decode and reads `Consistent`**, including when that order is not sorted.

### 4.2 `claim_matching_a_held_block_is_consistent_regardless_of_the_blocks_own_patch_order` — invert it

Same file. It asserts that a claim listing a block's patches in a *different* order still reads
`Consistent`. **Under D6 that is exactly what must now be refused**, and the reasoning is worth
carrying into the test's own doc comment because it is the correctness argument for the whole change:

> A claim names a `block_id`. Blocks are content-addressed, so **the same block id means the same
> canonical payload, therefore the same `patch_ids` sequence.** An honest claim about a block the
> receiver genuinely holds therefore matches it *in order*, always. A differently-ordered claim about a
> held block cannot arise from honesty — only from a lie or from a lossy claim format. **So sequence
> equality cannot produce a false accusation; it can only detect one.**

Rewrite it to assert `Contradicted` for a permuted claim, and name it for what it now proves — an
order-lie is detectable, which the sorted-set contract structurally could not do.

### 4.3 `prikk-object` payload tests

`recognition_claim_payload_rejects_unsorted_patch_ids_at_encode_and_decode` and
`..._rejects_duplicate_patch_ids_at_decode` assert the withdrawn contract. Replace both with
**order-preserving round-trip** tests: encode a descending sequence and a sequence containing a
duplicate, decode each, and assert the decoded `patch_ids` equals the input **exactly, order
included**. That is the invariant that now matters.

## 5. Check Stage 3's accept path for inherited assumptions

`accept_exchange_artifact` calls `check_recognition_claim_consistency` in Phase C, and a `Contradicted`
outcome refuses the whole exchange. **That is unchanged and correct** — under §4.2's argument, a
contradiction against a held block is still a demonstrated lie.

**But re-read Phase C and its fixtures for anywhere the sorted invariant was assumed rather than
checked** — a fixture that built claims sorted "because they must be" now needs to build them in block
order. Report what you found, including "nothing", so the answer is on the record either way.

## 6. Out of scope

- **Stage 4 itself** — the seal-from-accepted path. Next increment; this one only makes it possible.
- **Any `PEXCH001` format change.** D6 §11.5: the claim is authoritative for order and the artifact's
  sequence carries no meaning, but no format change is required and none should be made here.
- **`parent_patch_ids`** — D6 §11.3 rules it out; patch identity stays content-only.
- **Stage 1's `compute_patch_set_digest` sorted-input refusal** — a different object with a different
  contract. Do not "harmonize" it.

## 7. Tests and controls

Smaller than the last three increments, so the bar is proportionate — but every behavioural claim still
needs a control that was **observed failing**:

| Property | Control |
|---|---|
| A verbatim-order claim round-trips with order preserved | Re-introduce a sort in `encode_canonical` → round-trip test fails |
| A permuted claim about a held block is `Contradicted` | Restore the block-side `sort_unstable()/dedup()` → the inverted §4.2 test passes when it must fail |
| Duplicates are permitted and preserved | Restore the strict-sorted guard → duplicate round-trip fails |
| Identity is untouched | `snapshot.txt` diff is empty and vector 13 passes — assert by running, and say so |

## 8. What to report

1. **Whether `snapshot.txt` changed.** It must not. If it did: stop, escalate, do not regenerate.
2. Control output for each row of §7 — actual failure text, and the single line mutated.
3. What §5 turned up in the accept path, including "nothing".
4. The **full gate set against the exact commit, after the last edit**: `cargo fmt --all -- --check`;
   `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
   `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`; `git diff --check`;
   `cargo audit --no-fetch`; release-policy `check` / `boundary-check` / `reference-check`. Cross-target
   clippy pair only if this diff contains `#[cfg(target_os)]` — check this diff.
5. Test counts before and after, per crate. **Expect a small net change**, not growth: tests are being
   replaced, not accumulated.
6. Anything here that turned out to be wrong. **Say so plainly** — this document withdraws one of my own
   conditions and inverts two tests I previously accepted, so it is exactly the kind of change where a
   further mistake of mine is plausible.

**Stop and escalate, do not guess**, if: `snapshot.txt` moves; vector 13 fails; removing a guard makes
some third caller misbehave in a way §2.1 does not anticipate; or §4.2's argument turns out to be false
because two distinct blocks can somehow share an id.
