# Amendment — `Patch` schema 2 escalation: ruling

**Supersedes v1's §2 premise and answers its §2 blocker. The deletion proceeds.**
**Ruling by the architect, 2026-08-24.**

**The escalation was right on both counts, and my v1 was wrong on three separate estimates.** Recorded
because the pattern matters more than any one of them.

---

## 1. Three corrections to v1, all mine

1. **`PatchPayload::decode_canonical` does not exist.** I asserted a `RefState`-shaped monolithic decoder
   for `Patch`. There isn't one — decoding is three narrow functions. **Your read is correct: put the
   `schema_version` parameter on `decode_patch_operations`, the real general decoder.**
2. **I called the deletion cheap** on the strength of the `RefState` precedent. **`RefState` is not
   referenced inside format-2 state derivation. `Patch` and `Blob` are.** The precedent did not transfer.
3. **Then I called the format-2 amendment expensive**, and recommended retiring the field in place with
   a note. **The owner's response — "nonsense, the very technical debt" — is correct.** That is the
   "recorded, not rejected" pattern this arc has been deleting: a dead artifact kept with a note
   explaining that it is dead. **I pre-discounted the larger option, which is precisely what this
   project's standing principle forbids.**

## 2. The blocker's answer: `require_schema_one` is a defensive assertion, not a state-root dependency

**Traced it rather than estimating a third time.**

`read_patch_operations` (`lifecycle_cache/replay.rs:552-585`) does exactly four things: read the
envelope, check `object_type`, apply the `require_schema_one` check, then call
`decode_patch_operations(&envelope.canonical_payload)` and **return the operations.**

**Only the operations list reaches state derivation.** And `decode_patch_operations`
(`patch_replay/decode.rs:167`) contains **`2..=4 => {}`** — it skips tags 2, 3 and 4 entirely. **A
schema-2 Patch, identical but for field 2's absence, decodes to the identical operations and therefore
the identical state root.**

**So format-2's Merkle and state-root rules do not depend on Patch envelope schema.** The
`schema_version != 1` check duplicates knowledge that `format.rs`'s `validate_format2_schema` owns.

### The ruling

**Replace the hardcoded `!= 1` with the authoritative admitted-set check** — the same source
`validate_format2_schema` uses — rather than adding schema 2 to a second, hand-maintained list.

**This is strictly better than what is there now**, independently of this increment: it removes a
duplication of the admitted-schema contract. **`Blob` is unaffected** — it is admitted at `&[1]` only
(`format.rs:32`), so generalizing leaves its behaviour byte-identical while making it
correct-by-construction.

**Do not simply widen the check to `<= 2`.** A second hardcoded list is the defect, not the number in it.

**If the admitted set is not reachable from `lifecycle_cache`, report that rather than duplicating it** —
plumbing it is in scope; copying it is not.

## 3. The `accept.rs` ordering wrinkle — keep both, do not reorder

`decode_patch_parent_ids` (schema-blind, `:184`) runs **before** `decode_patch_operations` (`:193`), so
in that one caller the existing refusal shadows the new schema-aware one.

**That is the safe direction and it stays.** The schema-blind check is *stricter* — it refuses a
non-empty field at **any** schema, including schema 1 where the field is legal-but-must-be-empty.
**Keep both. Do not reorder `accept.rs`.** Note the shadowing in a comment so nobody later "simplifies"
the schema-blind check away as redundant — **it is not redundant; it is broader.**

## 4. Everything else in v1 stands

**§2's RFC 114 framing** (schema 1 frozen forever, this adds a pair), **§3 Gate A** (it will fail until
the `(Patch, 2)` vector exists — quote the failure), **§4's work items**, **§5's tests** — with §5 item 1
now the sharpest of them: **a schema-1 patch carrying field 2 must still decode and verify unchanged.**

**Add to §7's report:** which admitted-set source you used (§2), and confirmation that `Blob`'s
behaviour is unchanged.

## 5. What the escalation cost, and why it was worth it

**Nothing.** No code was written against a wrong premise, and the working tree stayed clean. **Three
wrong architect estimates were caught before any of them reached the frozen surface** — by reading the
functions instead of the doc comments, which is the discipline that has now corrected me four times in
this arc.
