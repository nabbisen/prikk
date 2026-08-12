# DC-95 Stage 1, Round 11 — Review v1

**Reviewing:** `b36ef18` and `4de5a83` on `dc-95-verify-coverage-and-finding-accumulation`, and the
resubmitted classified inventory.

**Accepted, no conditions.** §4 closes. **32 resolved, 2 remain.** The largest single round of Stage 1,
and the three unreachability claims — the decisions *not* to test — all hold.

## 1. The three "provably unreachable" claims, verified individually

These get checked hardest, because an unreachability claim is a decision to write no test, and its
failure mode is silent.

- **`verify_rollback_patch_envelope`'s `object_type != Patch`** — `rollback_draft.rs:30-32`:
  `is_rollback_draft_envelope` returns `Ok(false)` for any non-Patch envelope. The check at
  `rollback_verify.rs:132` runs only past that guard, so it can never be true. **Dead. Confirmed.**
- **The same function's `decoded.is_empty()`** — `patch_replay/decode.rs:179`: `decode_patch_operations`
  itself returns `Err(MalformedData)` when `operations.is_empty()`. The caller only sees an `Ok`, which
  is non-empty by construction. **Dead, for the stronger reason they give. Confirmed.**
- **`require_rollback_author_signature`'s wrong-algorithm arm** — `prikk-object/src/signature.rs:20-23`:
  `SignatureAlgorithm` has exactly one variant, `Ed25519 = 1`. The `!=` cannot be true for any value
  constructible in safe Rust. **Unreachable at the type level. Confirmed.**

**Finding them by inspection before attempting construction is the right order** and it is a change from
round 5, where technique groups were enumerated only after fixtures failed. Per the round 6 ruling on
the duplicate-identity checks: keep all three, no test, record the argument. They did.

## 2. Reproduced the hardest classification

Probed the wrong-signature-length check — `if false && signature.signature_bytes.len() != …`:

```
verify_repository_detects_rollback_draft_wrong_signature_length ... FAILED
  panicked at wal_cluster.rs:423
```

Line 423 is the `let Err(error) = result else` arm, so **`verify_repository` returned `Ok`** with the
check suppressed. Load-bearing, confirmed.

**Their classification of it is the most careful in Stage 1 so far.** They did not simply report
load-bearing: they recorded that disabling it still leaves a generic `signature_envelope_issues` entry
behind, and that the classification rests on that entry backing **no** `has_*` predicate — the standing
open question from round 4. **Naming the mechanism rather than the verdict is what makes the row
survivable if that open question is ever answered the other way**, and they carried the caveat onto the
row rather than leaving it in a review.

Gates at `4de5a83`: fmt clean, clippy **0**, **640** prikk-store tests, matching 633 → 640. Worktree
removed, primary tree clean. Inventory: 32 + 4 + 3 + 2 = 41.

## 3. `validate_read_schema` is the round 10 pattern recurring, and it is now a rule

The wrong-signature-length fixture first failed with a generic *"malformed algorithm shape"*, because
`Wal::replay()` calls `validate_read_schema`, which under `CurrentV2` runs `envelope.validate_strict()`
— catching the defect before `verify_rollback_draft_wal_records` is reached. Reachable only under
format-1, whose branch has no such check.

**That is structurally the same discovery as round 10's `require_retained_evidence`:** an upstream gate
intercepting the defect before the check under test sees it. Two rounds, two independent instances.

**Worth generalising now rather than after a third:** in this codebase, *a check's own code being
present does not establish that a defect reaches it*. Fixture construction has to establish the path,
not just the shape. That is what `b36ef18`'s doc comment says for one instance; the general form belongs
with it.

## 4. The op_seq byte surgery

`PatchPayload::encode_canonical` calls `validate()`, which rejects non-contiguous `op_seq`, so no struct
construction can produce decode-failing bytes. They encoded a valid payload and flipped the value bytes
in place, computing the offset from `CanonicalWriter::field_raw`'s own wire format rather than searching
for a byte pattern.

**Deriving the offset from the writer's format is what makes this maintainable** — a blind search would
pass today and silently target the wrong field after any layout change. Cheap to get wrong, and they
didn't.

## 5. Standing

- **Round 11: accepted.** §2, §4 and §6 complete. **Two rows remain:** §5's `InvalidForNonEmptyWal`,
  §7's lifecycle-cache "could not be independently verified".
- **Round 12** closes Stage 1. Take both.
- **Then Stage 1's deliverable is due**: the classified inventory moves into the code's own
  documentation, per the round 7 ruling and the classified-inventory ruling §5. It should not close as a
  review-request document.
- Green three-platform CI before any merge.
