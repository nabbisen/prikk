# RFC 117 stage 2a — the tag carries its patch count: implementation handoff

**Design:** `rfcs/handoffs/117-tag-sync/design-v1.md` **§9 (T7). Read it in full — §9.4 in particular,
which is the part most likely to be implemented as a security weakness by accident.**
**Base:** current `main` (`3d9c6c1` + the T7 design commit).
**Follows:** stage 1 (`babf54b`) and stage 2 (`3d9c6c1`). **Precedes stage 3, deliberately.**

**Why now:** stage 2 measured resolution at **O(N²)** — 64.6 ms at 500 blocks, 902 ms at 2000,
~37 minutes extrapolated at 100,000. Stage 3 is the last increment before tags are written in earnest,
and a field added before the first real producer is cheap.

---

## 1. The field

```
TagPayload {
    name, target_block_id, message, created_at, author_key_id, patch_set_digest,  // 1-6
    patch_count: u64,                                                             // field 7, NEW, REQUIRED
}
```

The number of **distinct patch ids in the closure the digest covers** — i.e. the same count
`patch_set_digest_preimage` already hashes (`DOMAIN ‖ count (u64 BE) ‖ sorted ids`).

- `field_u64(7, ..)` / the matching decode arm, `created_at`'s own shape.
- **Required.** A missing field 7 is a decode error, as field 6 is.
- `validate_format2_schema` stays **`Tag => &[1]`**. Still no second schema.

## 2. Resolution prunes by size before hashing

In `resolve_patch_set_digest`, take the target count alongside the target digest. During stage 2's
existing single pass, **compare each candidate's closure `.len()` to the target count first, and hash
only on a match.**

The size is free — the pass already builds the set. **Do not add a second traversal, do not
materialise all closures to filter afterwards**, and do not disturb the move/clone-on-fan-out scheme
stage 2 built; this is a comparison inserted before an existing hash call, not a restructuring.

**The signature changes.** `resolve_patch_set_digest(layout, digest)` becomes something that also takes
the count — take the whole `TagPayload`, or digest-plus-count explicitly. Your choice of shape; say
which you took and why.

## 3. §9.4 — the part to get right, and the reason to state it in the code

**A wrong `patch_count` can never produce a wrong resolution.** It can only cause the right candidate to
be skipped (→ `NotHeld`) or extra candidates to be hashed (→ slower). **The digest still decides.**

**Put that in the module doc.** "We filter on an attacker-supplied integer" reads as a weakness until
the reader sees the integer cannot *admit* anything — it can only narrow. This is D6 §11.6's
tried-not-trusted framing, one object over, and it should be recognisable as such.

**Required test, not optional:** a tag whose `patch_count` is wrong for its digest **never resolves to a
block**. Both directions: too small and too large. That is the property that makes the pruning safe, and
it is the one a reader will want to see asserted.

## 4. The frozen vectors — again

- **`rfc114_vector_11` moves a second time.** Regenerate deliberately; **add to the existing
  authorization comment rather than replacing it**, so the record shows the vector moved twice, when,
  and under which ruling each time. A vector that has moved twice with only one note reads like it moved
  once.
- **`empty_tag|5|1` must NOT move.** Generated from literal `b""`. **If it moves, stop and escalate.**
- Report both, explicitly, as in stage 1.

## 5. The measurement — this increment must show its own effect

Re-run stage 2's own `row6` benchmark shape at **500 and 2000 blocks**, before and after, and report all
four numbers.

**The claim to test is that the curve changed shape**, not merely that it got faster. Stage 2's was 14×
for 4× the blocks. If the new numbers are still superlinear at that ratio, the pruning is not working as
§9.2 predicts and **I want to know rather than have a faster O(N²) reported as a fix.**

## 6. Out of scope

- **The artifact section, receive path, local tag creation** (T3, T4). Stage 3.
- **A persisted digest index.** Still excluded; T7 is the cheaper answer to the same problem.
- **Any change to the digest itself or its preimage.** The count is exposed, not introduced.
- **Consolidating the four ref-tip resolution copies.** Still recorded, still not this — and still do not
  add a fifth.

## 7. What to report

1. **Vector 11's old and new values, and `empty_tag` unchanged** (§4). Both, explicitly, and confirm the
   authorization comment now records **two** moves.
2. **The four benchmark numbers** (§5), and whether the curve's shape changed.
3. Control output for: the field being required; pruning actually skipping size-mismatched candidates;
   and §3's wrong-count-never-resolves test, both directions.
4. **Which signature shape you took for `resolve_patch_set_digest`** (§2) and why.
5. The **full gate set against the exact commit, after the last edit**: `cargo fmt --all -- --check`;
   `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
   `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`; `git diff --check`;
   `cargo audit --no-fetch`; release-policy `check` / `boundary-check` / `reference-check`.
   Cross-target clippy pair only if this diff contains `#[cfg(target_os)]`.
6. Test counts before and after, per crate.
7. Anything here that turned out to be wrong. **Say so plainly.**

**Stop and escalate, do not guess**, if: `empty_tag` moves; the new numbers are still superlinear (§5);
or pruning by size turns out to require materialising closures the current pass does not already hold.
