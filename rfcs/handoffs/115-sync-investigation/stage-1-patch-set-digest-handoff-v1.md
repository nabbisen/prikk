# RFC 115 Stage 1 — the patch-set digest: implementation handoff

**Design:** `rfcs/handoffs/115-sync-investigation/design-v1.md` **§5 (D4)** and **§7**. Read both; this
handoff implements them and adds nothing architectural.

**Scope: one value and its documentation.** No artifact format, no exchange path, no accept logic.
Stage 1 is deliberately the smallest piece of RFC 115, chosen because it is independently useful — it
answers *"are these two repositories the same?"* today, with no transport — and because it exercises
RFC 114's documentation obligation on a surface where a mistake is cheap.

## 1. What to build

**A patch-set digest**: a canonical value over the set of patch ids reachable from a ref.

```
preimage := DOMAIN ("PRIKK-PATCH-SET-DIGEST-v1")
         || count (u64 BE)
         || each patch id, sorted ascending, deduplicated, 32 bytes each
digest   := sha256(preimage)
```

**Every element of that is decided, not a suggestion.** In particular:

- **The count is hashed even when zero.** An empty set must not collide with a degenerate one.
  `state_root.rs` already does this (`:70-78`) and it is the detail that would otherwise be found later
  by whoever compares two empty repositories.
- **Sorted and deduplicated.** Reuse `canonical.rs`'s existing `ObjectId` ordering — the same comparator
  `parent_block_ids` and `required_attestation_ids` already use. **Do not introduce a second ordering.**
- **SHA-256**, matching prikk's frozen algorithm surface (RFC 114 §2). Not Blake3.

**Shape it like `state_root.rs`, not like `id.rs`.** This is a comparison value, not a storable object:
a dedicated newtype, **not** an `ObjectId`, and no `(object_type, schema_version)` pair. `MerkleRoot` is
the precedent to follow.

## 2. Which patches are in the set

**Every patch reachable from the named ref**, via the existing machinery — `merge_evidence.rs`'s
`ancestors_inclusive` walk and each block's `patch_ids`, which is what `export_bundle` already does
(`bundle.rs:151-322`).

**Report before implementing if the reachable set is ambiguous for any ref kind** — a closed ref, a tag,
a received `remotes/` pointer. **Do not pick an interpretation silently**: the digest's whole value is
that two parties compute the same thing, so an ambiguity here is a correctness defect, not a detail.

## 3. It is identity-bearing, and that has consequences today

**Two prikk versions must produce identical bytes over the same patch set**, or the comparison means
nothing across an upgrade. So:

1. **Document the preimage in `docs/src/reference/release-compatibility.md`**, in the frozen list added
   by RFC 114 — **in this increment, not later.** RFC 114 exists because a promise nobody wrote down
   cannot be broken, only discovered absent.
2. **Add a frozen literal vector**, DC-40/RFC 114 Gate A style: a fixed set of patch ids and its expected
   digest, as committed literals. **Compute once, hardcode, delete the computation** — a vector derived
   at test time from the code under test asserts only that the code agrees with itself.
3. **Include the empty-set case as its own vector.** It is the case most likely to be broken by a
   refactor and least likely to be noticed.

## 4. Surface

**A library function is the deliverable.** A CLI surface is optional and, if added, must print the digest
and nothing else — no interpretation, no "in sync"/"out of sync" verdict. **Two equal digests mean the
two repositories hold the same patches; they do not mean the repositories are otherwise identical**
(design §7: block lineage, state roots and publication trust are local and deliberately not comparable).
**If the CLI implies more than that, it is wrong.**

## 5. Tests

- The frozen vectors of §3, including empty.
- **A negative control**: perturb the preimage construction — drop the count, change the domain, reverse
  the ordering — and confirm the vectors fail. Report the failing output; a vector nobody has seen fail
  is not evidence.
- **Two repositories holding the same patches produce equal digests**, and this must hold **with
  different block structure on each side** — that is the property the digest exists for, and a test that
  builds both sides identically does not prove it.
- **A repository with one extra patch produces a different digest.**

## 6. Out of scope

- Any exchange artifact, transport, or accept path.
- The recognition claim object (Stage 2) and the artifact (Stage 3).
- Comparing anything other than patch sets — no block, lineage or state-root comparison.

## 7. Reporting

Report before pushing, with the negative control's output and the full gate set **run against the exact
commit after the last edit**. If §2's reachable-set question turns out to have no single obvious answer,
**stop and report rather than choosing** — that is an architecture decision and it is mine.
