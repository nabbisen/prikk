# RFC 116 stage 3 — the sender side: implementation handoff

**Design:** `rfcs/handoffs/116-sync-negotiation/design-v1.md` §1.2 (as amended), §4 (N4), §5 (N5).
**RFC:** `rfcs/accepted/116-sync-negotiation-and-transport.md` (ACCEPTED, both rulings).
**Base:** current `main` (`10553f8`). **Completes the negotiation loop.**

**This increment introduces the first `RecognitionClaimPayload` producer in the project's history, and
by doing so permanently closes the free-schema-amendment window.** Two amendments have already used
that window — D6 (the claim carries block order) and N3 (the claim carries block parents). A
stress round found no third missing field. **After this merges, any further claim change costs a second
schema version with two contracts carried forever** (RFC 114). Treat §3 accordingly.

**No network. No new dependency.** `prikk-store` stays bytes-in, bytes-out.

---

## 1. What to build

One function: given a ref name and a have-list received from the other side, produce the `PEXCH001`
that closes their gap.

```
build_sync_artifact(layout, ref_name, have_list_bytes, signer) -> (report, Vec<u8>)
```

**One ref, one artifact** (design §1.2's amendment). Do not accept a list of refs, and do not build a
union — that was the error the stress round caught.

Steps:

1. Decode the have-list; **its own digest/list self-consistency check applies** (stage 2, `PSYNCHV1`).
2. `compute_sync_delta` for this ref — stage 2's function, unchanged.
3. Find the blocks that contain the delta's patches (§2).
4. Build, sign and persist one claim per such block (§3).
5. `export_exchange_artifact(layout, delta_patch_ids, claim_ids)` — **existing signature, unchanged.**
   It already derives the blob closure from patch operations and gathers author key material scoped to
   the exported patches.

## 2. Finding the blocks — derive it, do not build an index

There is **no patch→block reverse index**, and this increment should not add one. Walk the ref tip's
ancestry (`merge_evidence::ancestors_inclusive`, the same walk Stage 1's
`patch_ids_reachable_from_block` and bundle export both use) and map each block's `patch_ids` to its
block. **Do not write a second traversal** — Stage 1 deliberately reused that walk so the digest could
not drift from what export ships, and the same reasoning applies here.

A block qualifies when **any** of its `patch_ids` is in the delta.

## 3. The claim — the ruling that matters most

**Build the claim from the block, verbatim, in full.**

```
RecognitionClaimPayload {
    block_id:         <the block's own id>,
    patch_ids:        <the block's own patch_ids,        VERBATIM AND COMPLETE>,
    parent_block_ids: <the block's own parent_block_ids, VERBATIM AND COMPLETE>,
}
```

**Do not trim `patch_ids` to the delta.** This will be tempting — the receiver already holds some of
them, so sending the full list looks wasteful. It is not optional:

- **A trimmed claim is a false statement about the block.** The claim asserts *"block B contains these
  patches"*. Trimmed, it asserts something untrue.
- **It would break the receiver's own lie-detector.** `check_recognition_claim_consistency` compares a
  claim against a block the receiver holds by **exact sequence equality** (D6, N3). A trimmed claim
  about a block the receiver has would read `Contradicted` — the receiver would correctly conclude the
  sender lied.
- **D7 already handles the overlap on the receiving side.** Patches the receiver has sealed are
  skipped; absence is the only refusal. **That is why D7 exists**, and trimming here would be solving
  on the sending side a problem already solved correctly on the receiving one.

**No new disclosure:** the extra ids are ones the receiver named in its own have-list.

Sign with `maintainer_signature` (the production path, not `test_support`'s dummy). Persist the claim
objects, then pass their ids to `export_exchange_artifact`.

**The signer must be locally trusted:** call `verify_signer_trusted` before signing. Signing a claim is
not sealing and confers nothing on the receiver — but a repository should not emit signed assertions
under a key it does not itself adopt. Secure by default; state the reason in the module doc so it is
not later removed as redundant.

## 4. Absence is not a refusal — the carry-forward from stage 2's review

Stage 2's review §7 recorded this: *"an asymmetric ref set is never refused"* is currently pinned by a
passing test but by **no control**, because stage 2 has no refusal path to mutate. **Stage 3 is the
first code that acts on a delta, and where an accidental refusal on an absent ref would first do
damage.**

Required behaviour, each with its own test:

- A ref the **sender** does not hold → an artifact carrying an empty delta, **not** a refusal.
- A have-list naming a ref the **receiver** does not hold (empty list) → the full reachable set as the
  delta, **not** a refusal.
- An empty delta (already in sync) → **report it as such**; do not build a pointless artifact and do
  not error.

## 5. Security properties (N5), as tests with controls

Each needs a test **and** an observed-failing control.

| # | Property | Control |
|---|---|---|
| 1 | The artifact carries **exactly** the delta — no more | Send the full reachable set instead → the artifact contains patches the receiver already has |
| 2 | Claims carry the block's full, verbatim `patch_ids` and `parent_block_ids` | Trim either to the delta → a receiver holding that block reads `Contradicted` |
| 3 | An untrusted signer cannot produce an artifact | Drop `verify_signer_trusted` → an unadopted key signs claims |
| 4 | A ref absent on either side is not refused (§4) | Introduce a refusal on the absent-ref path → the §4 tests fail |
| 5 | Building an artifact adopts no key and changes no trust | Adopted-maintainer set byte-identical across a build |
| 6 | Round trip: build → accept → seal lands the delta | End to end, asserting the delta's patches are reachable from the receiver's ref tip afterwards |

**Row 6 is the one that proves the loop closes.** Everything before it proves pieces; row 6 proves that
two repositories can be made the same. Assert reachability from the ref tip, **not** that accept
returned `Ok`.

**Row 2 is the one most likely to be "optimised" away later.** Its control must show a `Contradicted`,
not merely a differing byte count, so the reason is on the record.

## 6. Out of scope

- **Any transport, protocol, socket, or new dependency.** RFC 116 ruling 2.
- **A patch→block index** (§2).
- **Tag sync** — stage 2 ruled tags out of the summary; unchanged here.
- **Set reconciliation** — revisit on measurement.
- **Changing `export_exchange_artifact`'s signature.** If you believe it must change, **escalate** —
  that would mean my §1 step 5 is wrong.
- **CLI wiring**, unless trivial; say in your report if you think the boundary should move.

## 7. What to report

1. Control output for each row of §5 — actual failure text, and the single line mutated.
2. **For row 6:** the end-to-end assertion, including what you read back from the receiver's ref tip.
3. The **full gate set against the exact commit, after the last edit**: `cargo fmt --all -- --check`;
   `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
   `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`; `git diff --check`;
   `cargo audit --no-fetch`; release-policy `check` / `boundary-check` / `reference-check`.
   Cross-target clippy pair only if this diff contains `#[cfg(target_os)]`.
4. Test counts before and after, per crate. **`snapshot.txt` must not change** — this defines no schema.
5. **Explicitly: confirm the claim payload you construct matches the block verbatim in both fields**,
   and say how you verified it. This is the increment that closes the schema window; if the first
   producer writes a shape the schema did not intend, we live with it.
6. Anything here that turned out to be wrong. **Say so plainly.**

**Stop and escalate, do not guess**, if: §3's full-verbatim rule makes an artifact implausibly large in
a realistic case; `export_exchange_artifact` cannot take persisted claim ids as §1 assumes; or the
end-to-end round trip in row 6 cannot be built — **that last one would mean the loop does not actually
close, which is this increment's entire purpose.**
