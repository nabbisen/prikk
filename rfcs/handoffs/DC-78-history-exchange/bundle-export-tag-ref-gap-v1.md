# `bundle export` fails on a tag ref, with a message naming the wrong problem

**Found 2026-08-19** by the dev team while investigating RFC 115 Stage 1's reachable-set question, and
correctly reported rather than fixed in place. **Verified independently by the architect.** Live on
`main`.

## 1. The defect

A `tags/*` ref does not point at a Block. `tag.rs`'s own module doc states the model: *"a tag ref must
point at the tag object, never directly at a block — two hops: ref -> tag object -> block"*, and
`TagPayload.target_block_id` (`payload/tag.rs:14`) is where the Block id lives.

**`export_bundle` performs only the first hop.** `bundle.rs:178` takes
`ref_state_payload.target_object_id` as `tip_block_id` and passes it to `ancestors_inclusive` (`:189`),
which does `read_typed(block_id, ObjectType::Block)` (`merge_evidence.rs:415-420`).

**Given a tag ref, that reads a Tag object id as a Block id** and fails with
`Integrity("missing Block {id}")` — an error naming a missing object when the real problem is an
unresolved indirection. **Nothing in `bundle.rs` mentions `RefKind::Tag` or the two-hop model**, and no
test exercises export against a tag ref.

## 2. Why it matters more than the failure looks

**The failure mode is a misleading diagnosis, not a crash.** A user exporting a tag is told an object is
missing from their repository. It is not missing; it was never a Block. **A message that sends someone
looking for corruption they do not have is worse than a plain refusal** — and this project already ruled
once this week that a refusal must not name a problem the user cannot act on (the `PBNDL001` migration
loop).

## 3. The fix

**Resolve the second hop when the ref is a tag**: read the Tag object, take `target_block_id`, walk from
there.

**Decide and state what a tag bundle contains** — at minimum the Block closure. **Whether the Tag object
itself travels is a real question**, not a detail: a bundle exporting a tag whose Tag object is absent
leaves the receiver with the history but not the tag. **Report your reading before implementing**; if the
Tag object must travel, that changes what the artifact contains.

**If a ref kind cannot be resolved to a Block by any supported path, refuse with a message that says
so** — not one that reports a missing object.

## 4. Scope

- **`export_bundle` only.** Import consumes what export produced and is unaffected.
- **No format change** if the Tag object already falls inside the exported closure; **report before
  changing that** if it does not.
- **Not RFC 115 Stage 1's concern.** Stage 1 adds two-hop resolution for its own digest path *because*
  `export_bundle` lacks it. The two fixes are independent and must not be folded together.

## 5. Tests

1. **Exporting a tag ref succeeds**, and the resulting bundle imports and verifies.
2. **A tag ref and the `heads/*` ref pointing at the same block export the same object closure** — the
   property that says the second hop landed in the right place.
3. **A negative control**: revert the second hop, watch test 1 fail, restore, and report the output.
4. **An unresolvable ref kind refuses with a message naming the ref kind**, not a missing object.

## 6. Reporting

Report before pushing, with §3's Tag-object reading stated explicitly, and the full gate set run against
the fixed commit after the last edit.
