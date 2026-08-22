# RFC 117 — Tags across repositories: what a tag names when blocks diverge

**Status.** **PROPOSED, 2026-08-22.** An investigation with a recommendation; **not a design, and
implementation must not start from it.** Written on the owner's direction after criterion 1 closed
(`199a1e4`), which recorded "branches only — tag sync is its own unanswered question" as a stated limit.

**Independence:** author-reviewed, the standing ceiling. Every claim cites the code or record it came from.

---

## 1. The problem, as a team actually meets it

Alice tags `v1.0`. Bob syncs her patches. **Bob has no `v1.0`, and cannot identify which of his blocks
is the release.** He has the code; he cannot name it.

That is not a bug in sync. It follows directly from an accepted ruling: **blocks diverge between
repositories by design** (RFC 115 §2.4-§2.7), because identity lives at the patch level. And
`TagPayload.target_block_id` names a **block** (`payload/tag.rs:14`). A tag therefore names something
that exists in exactly one repository.

**For a VCS whose users ship releases, "v1.0 means different things in different repositories" defeats
the purpose of a release.** This is a *semantic* gap, not a cost gap — which is why it is proposed
ahead of remote-tracking.

## 2. Why prikk is in the harder camp, and who else is

**Git, Mercurial and jj copy tags trivially because commit identity is global** — the same commit has
the same hash everywhere, so a tag is a pointer that transfers unchanged. Mercurial goes further and
version-controls the tag list itself (`.hgtags`), which works for the same underlying reason.

**prikk gave that up deliberately.** So it sits with the patch-theoretic systems:

- **Darcs** makes a tag *a patch* — a special patch depending on every patch present. It therefore
  names a patch **set**, and travels like any other patch.
- **Pijul** derives tags from channel state, which is itself content-derived from the patch set.

**Both answer the same way: when blocks/states are local, a tag must name the patch set.**

## 3. prikk already has the primitive

`PatchSetDigest` (RFC 115 Stage 1) is a canonical 32-byte value over the sorted, deduplicated patch
ids of a closure, and `compute_patch_set_digest_from_block` computes it for any block. **Two
repositories holding the same patches produce the same digest by construction** — that is the property
the whole sync design rests on.

So the identification a tag needs is **already computable today**. The manual workaround proves it:
Alice can tell Bob "v1.0 is the patch set with digest X", and Bob can find his matching block. Nobody
has made that a first-class thing.

## 4. The constraint that shapes every option — and it is not the one that shaped RFC 116

**`TagPayload`'s schema window is CLOSED.** Verified: `0.22.1` ships the `tag` command and writes
`TagPayload` with `schema_version` 1 (`git show 0.22.1:crates/prikk-cli/src/tag.rs`). A released binary
has written this pair, so RFC 114's promise binds it.

**This is the opposite of the situation D6 and N3 exploited twice**, where no release had ever written a
`RecognitionClaim` and amendment was free. Here, any field addition is a genuine **schema 2, with two
contracts to decode forever**, plus its own Gate A vector. That cost is real and must be paid
deliberately rather than discovered.

## 5. Options

**(a) `TagPayload` schema 2 adds `patch_set_digest`.** The tag names both its local block *and* the
patch set, so it is self-contained, signed and attributable. On sync it travels; the receiver resolves
the digest against its own blocks and **creates its own local tag** — exactly as it creates its own
blocks. v1 tags stay decodable forever and can be upgraded locally by computing the digest from the
block they already name.
*Cost: a real schema 2, permanently.*

**(b) The tag becomes a patch** (Darcs' answer). Travels automatically with no new sync machinery, but
makes tags part of sealed history and changes the patch model — a larger semantic change than (a), and
`PatchPayload` is equally frozen.

**(c) A tag manifest outside the object model** — sync ships `name → digest` pairs; the receiver
resolves locally. **No schema change at all.** But it is a second identity mechanism alongside the
signed Tag object, and an unsigned manifest is unattributable: anyone could assert `v1.0` is anything.
Signing it means inventing a second signed-assertion type when one already exists.

**(d) Status quo, documented.** Costs nothing; leaves releases unshareable.

## 6. Recommendation — (a), with two details that need deciding in design

**(a)** keeps one signed, attributable tag object; reuses a primitive that already ships; leaves v1
readable forever; and needs no new object type. Its cost is honest and bounded: one schema version, one
Gate A vector, one dual-contract decoder — the same shape `RefState` already carries (schemas 1 and
`REF_STATE_CLOSED_SCHEMA`), so there is precedent in the codebase for exactly this.

Two details a design must settle, named now so they are not discovered later:

1. **Resolution ambiguity.** Two distinct blocks in one repository can share a patch set — same
   patches, different order gives a different state root and therefore a different block, but an
   identical digest. So "find my block matching digest X" may return **more than one**. Consistent with
   every other ruling in this arc, the answer should be **refuse and report, never pick** — but which
   disambiguator (the named ref's tip? the tag's own ref?) is a design decision.
2. **Whether a received tag is created automatically or explicitly.** Sealing is already an explicit
   local act under the receiver's own key (D5). A tag is a signed assertion, so the same reasoning
   suggests the receiver signs its own tag deliberately rather than having sync mint one — but that is
   a UX and trust decision, not an obvious consequence.

## 7. What the owner rules

1. **Is (a) the direction**, accepting a permanent `TagPayload` schema 2 — or is (c)'s no-schema-change
   route preferable despite fragmenting attribution?
2. **Does this outrank remote-tracking** (the O(history)-per-exchange cost)? The architect recommends
   yes — a broken meaning is worse than an expensive operation — but both are RFC-scale and the
   sequencing is the owner's.

## 8. Non-goals

- **Designing the schema.** Not until §7.1 is answered.
- **Remote-tracking, counterpart identity, transport.** Separate, and still open.
- **Changing how tags are created locally.** `prikk tag` keeps working; this concerns what travels.
- **Retroactively changing existing v1 tags.** They stay valid and readable; upgrade is local and optional.
