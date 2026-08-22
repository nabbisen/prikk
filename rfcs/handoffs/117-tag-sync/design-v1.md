# RFC 117 — tag sync: design v1

**RFC:** `rfcs/accepted/117-tag-sync.md` (ACCEPTED 2026-08-22, adopting recommendation (a)).
**Independence:** author-reviewed, the standing ceiling.

**AMENDED 2026-08-22, same day, by an owner ruling that lifts the constraint this design was written
under:** *"No project has been created in production in the world yet. Breaking change is accepted."*

**So: no schema 2. `TagPayload` is amended in place at `schema_version` 1, and v1 tags break.** One
contract instead of two, forever. **Get it right once — there is no second version to correct it in,
and this is the last moment the ruling makes a break free.**

**The one thing that is not free, and must be done knowingly:**
`rfc114_vector_11_tag_schema_1_identity_and_signature` hardcodes a populated `TagPayload`'s canonical
bytes, object id and signature preimage, and **all three move.** Normally a moved identity vector is a
stop-work finding; here it is authorized. **Regenerate deliberately and record the ruling beside the new
values.** `empty_tag|5|1` does **not** move — it is generated from literal `b""`.

---

## 1. T1 — `TagPayload` schema 2 carries the patch set

```
TagPayload v2 {
    name, target_block_id, message, created_at, author_key_id,   // fields 1-5, unchanged
    patch_set_digest: PatchSetDigest,                            // field 6, NEW, required
}
```

`patch_set_digest` is `compute_patch_set_digest_from_block(target_block_id)` — the digest of the patch
closure of the block the tag names. **Two repositories holding the same patches produce the same value
by construction**, which is the property that makes a tag portable.

**Both fields are kept, not one.** `target_block_id` remains because a tag must still name something
locally resolvable and because v1 compatibility depends on the field's meaning not moving.
`patch_set_digest` is what travels. **A tag is therefore a local pointer plus a global identity**, and
the design should say so in exactly those words wherever it is documented.

**Required, not `Option`.** A tag without a digest would be the old tag with extra steps, and
optionality here buys nothing but a second failure mode. The ruling that permits breaking v1 is what
makes "required" affordable — take it.

## 2. T2 — resolution, and the ambiguity that must be refused

Given a digest, the receiver finds the local block whose patch closure matches.

**Two distinct blocks in one repository can share a patch set** — the same patches sealed in a different
order give a different state root, therefore a different block id, therefore two blocks with one digest.
So resolution can return **zero, one, or several**.

**Ruled:**

- **Zero** → the receiver does not hold that patch set. Report it; **not an error.** This is the
  ordinary "you have not synced that far yet" case.
- **One** → resolved.
- **More than one** → **refuse, naming every candidate.** Do not prefer the ref tip, do not prefer the
  newest, do not pick. Ambiguity about which block a release names is exactly the thing a release must
  not guess at, and this project refuses ambiguity everywhere else (`refuse_if_order_ambiguous`,
  `order_claims_for_sealing`'s duplicate-block refusal).

**Resolution must not walk every block in the repository per lookup.** Reuse the ancestry walk already
shared by Stage 1's digest and bundle export (`merge_evidence::ancestors_inclusive`) — do not add a
third traversal, and do not build a digest index in this increment.

## 3. T3 — how a tag travels

**The signed Tag object travels in the exchange artifact**, as a new section alongside patches, blobs,
author key material and claims. `PEXCH001` is **representational** (RFC 114 §3), so adding a section is
a format revision, not a frozen-surface change — but it is still a format revision and must be
versioned as one.

**Why the object and not just `name → digest`:** an unsigned name/digest pair is unattributable — anyone
could assert `v1.0` is any patch set. The signed Tag object carries its author and is verifiable on the
same terms a recognition claim is. §5 of RFC 117 rejected the manifest for exactly this reason; do not
reintroduce it as an optimisation.

**A received tag is reported, never gating** — the same rule D3 sets for claims. An unadopted signer's
tag is `Unverifiable`, visible, and does not refuse the exchange.

## 4. T4 — the receiver's tag is the receiver's own act

**Ruled: sync does not mint tags. The receiver creates its own tag, signed by its own key, as an
explicit act.**

This mirrors D5 exactly: accept writes objects, **sealing is a separate explicit local act**. A tag is a
signed assertion about your own repository, so it must carry your signature, not be conjured on your
behalf from someone else's assertion.

Practically: a received Tag object is stored and reportable; a separate command resolves its digest
against local blocks (§2) and creates a **local** v2 tag naming the local block and the same digest.
**The sender's tag and the receiver's tag are different objects with the same global identity** — which
is the same relationship their blocks already have.

## 5. T5 — v1 tags break, and that is the ruling

`validate_format2_schema` keeps `Tag => &[1]`. **There is no second accepted schema**, so
`TagPayload`'s decoder gains a required field and **every Tag object written before this change stops
decoding.**

**In scope of the break, stated so nobody discovers it:**

- Tags written by `0.22.1` or any dev build become unreadable; `verify` will report them.
- **Any bundle carrying a v1 tag becomes unimportable** — DC-78's tag-bundle path included.
- Test fixtures constructing `TagPayload` all need the new field.

**There is no migration and none should be written.** The owner's ruling is that no production data
exists; writing a migration for data that does not exist would be inventing an obligation RFC 114 does
not impose and then carrying it forever. **A repository holding v1 tags re-creates them** — the digest
is computable from the block the old tag named, so the information is not lost, but the new tag is a
**new signed assertion under the re-creator's own key**, not a rewrite of the old object.

## 6. T6 — security properties, as refusals

1. **Resolution ambiguity refuses** (§2). Never picked, never defaulted.
2. **A received tag confers nothing.** No key adoption, no local ref movement, no automatic tag creation.
3. **Signature outcome is reported, never gating** (§3), matching claims.
4. **A tag whose digest matches no local block is not an error** (§2) — asymmetry is ordinary.
5. **Declared counts and sizes bounded before decoding**, DC-86's standard, for the artifact's new section.
6. **A refused exchange records no tag**, on the same terms §8.1 sets for claims and key material.

## 7. Deliberately excluded

- **Tag *deletion* or *movement* across repositories.** prikk tags are create-once and immutable
  locally; making removal travel is a separate question and a larger one.
- **Discovery of what tags a counterpart has** without a full exchange — that is remote-tracking's
  territory (RFC 116 §6's open item), not this.
- **A digest→block index.** §2 forbids it here; revisit on measurement.
- **Changing `prikk tag`'s local behaviour** beyond writing v2 going forward.

## 8. Staging

1. **`TagPayload` field 6 at schema 1 + the deliberately regenerated `rfc114_vector_11`** — the frozen
   surface, alone, so it gets its own review. **`empty_tag|5|1` must not move; vector 11 must, and the
   report must show both.** `validate_format2_schema` stays `Tag => &[1]`.
2. **Local resolution** (§2), including the ambiguity refusal.
3. **The artifact section and the receive path** (§3), plus the explicit local tag creation (§4).

**Report before implementing each**, and RFC 115 §5.1's discipline — threat model, adversarial fixtures,
a two-repository harness, a negative control per refusal — applies to each stage.

## 9. T7 — the tag carries its patch count, so resolution can prune before hashing

**Added 2026-08-22, on the owner's ruling ("Take it now"), after stage 2 measured what T2 actually
costs.** Finding recorded in `.git-exclude/reviewed/RFC-117-stage-2-digest-resolution-review-v1.md` §3.

### 9.1 The measurement, not the estimate

`resolve_patch_set_digest` is **O(N²)**, reproduced independently by the architect: **64.6 ms at 500
blocks, 902 ms at 2000** — 14× for 4× the blocks. Extrapolated, ~22 s at 10,000 blocks and **~37
minutes at 100,000**, to resolve a single tag.

**The traversal is not the problem and was fixed correctly in stage 2.** The cost is irreducible under
the shape T1 left: **every candidate's closure must be hashed in full**, because a `PatchSetDigest`
carries no information about the closure it summarizes, so nothing can be skipped.

### 9.2 The ruling

**`TagPayload` gains field 7, `patch_count: u64`** — the number of distinct patch ids in the closure
the digest covers.

Resolution then **filters candidates by closure size before hashing any of them**. The size is free:
stage 2's pass already builds each closure set, and `.len()` costs nothing. **In a linear history the
closure sizes are 1, 2, 3, … N — all distinct — so exactly one candidate is ever hashed.** O(N²)
collapses to O(N log N). A branchy history prunes less completely and still enormously.

### 9.3 This is not new information — it is information the digest hashes away

`patch_set_digest_preimage` is `DOMAIN ‖ count (u64 BE) ‖ sorted ids`. **The count is already part of
what is hashed.** Field 7 does not introduce a new fact about the patch set; it exposes a fact the
digest already commits to, so it can be compared in one integer comparison instead of a full hash.

That also settles the obvious worry: **a tag whose count disagrees with its digest is self-inconsistent
and will simply fail to resolve** — the count narrows, the digest decides.

### 9.4 The count is a hint that prunes, never an authority

**A wrong `patch_count` can never produce a wrong resolution.** It can only cause the right candidate to
be skipped (→ `NotHeld`) or extra candidates to be hashed (→ slower). **The digest still has to match.**

This is D6 §11.6's framing again, one object over: the cheap field is *tried*, the expensive one
*decides*. State it in the module doc, because "we filter on an attacker-supplied integer" reads as a
weakness until the reader sees that the integer cannot admit anything.

### 9.5 The cost, stated plainly

**`rfc114_vector_11` moves a second time**, and `TagPayload` is touched twice in three increments. Both
are affordable only because of the standing owner ruling that prikk has no production users and breaking
changes are accepted. **`empty_tag|5|1` must still not move.**

**Why now rather than later:** stage 3 is the last increment before tags are written in earnest. This is
the same sequencing that put N3 ahead of RFC 116's sender side — a field added before the first real
producer is cheap; after it, it is a second contract forever.

### 9.6 The pattern, named because it is now three for three

D6 (block order), N3 (block parents), T7 (patch count): **each object was specified against the
narrowest use that was in front of us, and each widening of use has cost exactly one more field.** Every
one was found by designing the *next* stage, never by reviewing the last. **When specifying an object,
ask what the stage after next will need of it** — that question has been worth a field three times.

### 9.7 Staging, amended

Insert before the old stage 3:

- **2a. `TagPayload` field 7 + resolution prunes by size + `rfc114_vector_11` regenerated again.**
- **3. The artifact section and the receive path** (T3, T4), unchanged, now last.
