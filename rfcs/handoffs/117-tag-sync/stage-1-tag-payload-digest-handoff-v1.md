# RFC 117 stage 1 — `TagPayload` carries the patch-set digest: implementation handoff

**Design:** `rfcs/handoffs/117-tag-sync/design-v1.md` — **T1 and T5, plus the amendment banner at the
top. Read the banner first: it lifts the constraint the rest of the document was written under.**
**RFC:** `rfcs/accepted/117-tag-sync.md` (ACCEPTED 2026-08-22).
**Base:** current `main`.

**This is the frozen surface, alone, so it gets its own review.** It deliberately breaks existing v1
tags, and it deliberately moves a frozen identity vector. Both are authorized by an owner ruling
(*"No project has been created in production in the world yet. Breaking change is accepted."*). Neither
should be done quietly.

---

## 1. The cross-crate structure — ruled, with an exact precedent

`PatchSetDigest` currently lives in `prikk-store` (`patch_set_digest.rs:40`). `TagPayload` lives in
`prikk-object`, which **cannot** depend on `prikk-store`.

**Ruled: move the `PatchSetDigest` *newtype* into `prikk-object`, and leave every *computation* in
`prikk-store`.**

**The precedent is exact and should be followed rather than reasoned from scratch:** `MerkleRoot` is a
32-byte newtype in `prikk-object/src/payload/common.rs`; `compute_state_root` lives in
`prikk-store/src/state_root.rs`; `BlockPayload` carries the value and decodes it with
`read_array::<32>()`. **Do the same.** `compute_patch_set_digest`, `compute_patch_set_digest_from_block`
and `compute_patch_set_digest_for_ref` all need an `ObjectReader` and stay where they are.

**Do not** carry a bare `[u8; 32]` in the payload. This codebase separates 32-byte value kinds by type
on purpose, and a raw array beside `target_block_id` invites exactly the confusion that separation
prevents.

## 2. The field

```
TagPayload {
    name, target_block_id, message, created_at, author_key_id,   // fields 1-5, unchanged
    patch_set_digest: PatchSetDigest,                            // field 6, NEW, REQUIRED
}
```

- Encode with `field_bytes(6, ..)`, decode with `read_array::<32>()` — `MerkleRoot`'s own shape.
- **Required.** A missing field 6 is a **decode error**, not a default. That is the break, and it is
  the point.
- `validate_format2_schema` stays **`Tag => &[1]`**. There is no schema 2 and no dual contract.

## 3. Where the digest comes from

`prikk tag` computes it at creation: `compute_patch_set_digest_from_block(store, target_block_id)`.

**The invariant to state in the module doc and to test:** a tag's `patch_set_digest` is the digest of
the patch closure of the block it names. **Two repositories holding the same patches produce the same
value** — that is the whole reason the field exists.

## 4. The frozen vectors — one moves, one must not

- **`rfc114_vector_11_tag_schema_1_identity_and_signature` MOVES.** It hardcodes the canonical bytes,
  the object id (`22afa858…`) and the signature preimage of a populated `TagPayload`; a required field
  changes all three. **Regenerate deliberately, and put a comment beside the new values recording that
  the move was authorized by RFC 117's owner ruling, with the date.** A future reader must be able to
  tell authorized regeneration from silent identity drift.
- **`empty_tag|5|1` must NOT move.** It is generated from literal `b""` in `vectors.rs`, independent of
  the struct. **If it moves, stop and escalate** — that would mean something changed that this
  increment does not intend.

**Your report must show both facts explicitly**: vector 11's old and new values, and `empty_tag`
unchanged. "The snapshot changed" is not an acceptable summary here.

## 5. Expected fallout — this is the break, not a surprise

Every site that decodes a `TagPayload` now requires field 6. Expect to update fixtures in at least:
`refs/verify/scan.rs`'s tag arm, `bundle.rs` (the DC-78 tag path), `patch_set_digest.rs`'s tag two-hop,
`prikk-cli/src/tag.rs`, and their tests.

**A v1 tag no longer decodes anywhere, and any bundle carrying one is unimportable.** Authorized.
**Write no migration** — the design's T5 rules it out, and inventing one for data that does not exist
would create a permanent obligation RFC 114 does not impose.

**But report the blast radius**: list every file you had to touch. If it is larger than the list above,
I want to know what else depends on this shape.

## 6. Tests and controls

| # | Property | Control |
|---|---|---|
| 1 | Field 6 is required — a payload without it fails to decode | Make it `Option`/defaulted → the missing-field test decodes successfully |
| 2 | A tag's digest equals its block's patch-closure digest | Compute over a different block → the equality test fails |
| 3 | Two repositories with the same patches produce the same tag digest | Derive it from anything block-local (e.g. the block id) → the cross-repository equality test fails |
| 4 | `empty_tag` is unmoved and vector 11 is exactly the regenerated value | Assert by running; show both |

**Row 3 is the one that matters** — it is the property the whole RFC exists for, and the only one that
would still "pass" under a wrong implementation that simply stored *something* 32 bytes long. Build it
as two independently-constructed repositories holding the same patches, not one repository inspected
twice.

## 7. Out of scope

- **Resolution of a digest to a local block** (design T2). Stage 2.
- **The artifact section, the receive path, local tag creation** (T3, T4). Stage 3.
- **Any change to `validate_format2_schema` beyond leaving it alone.**
- **A migration for v1 tags** (§5).

## 8. What to report

1. **Vector 11's old and new values, and `empty_tag` unchanged** (§4). Both, explicitly.
2. The **blast radius** (§5) — every file touched.
3. Control output for each row of §6 — actual failure text, and the single line mutated.
4. **For row 3:** how you constructed two genuinely independent repositories holding the same patches.
5. The **full gate set against the exact commit, after the last edit**: `cargo fmt --all -- --check`;
   `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
   `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`; `git diff --check`;
   `cargo audit --no-fetch`; release-policy `check` / `boundary-check` / `reference-check`.
   Cross-target clippy pair only if this diff contains `#[cfg(target_os)]`.
6. Test counts before and after, per crate.
7. Anything here that turned out to be wrong. **Say so plainly.**

**Stop and escalate, do not guess**, if: `empty_tag` moves; moving `PatchSetDigest` to `prikk-object`
drags anything else across the crate boundary with it; or the blast radius in §5 reaches code this
handoff does not anticipate.
