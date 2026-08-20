# RFC 115 — patch-level exchange: design v1

**RFC:** `rfcs/accepted/115-sync-investigation.md`
**Written by the architect 2026-08-19**, per the design authority the role assigns and after withdrawing
Checkpoint 2 from the dev team, which had asked them to invent architecture decisions.

**Inputs.** The owner–architect discussion recorded in RFC 115 §2 (the reframing, the patch-unit ruling,
the no-arbitration ruling, prior art, production costs, the binding test discipline), and the dev team's
Checkpoint 1 **investigation** — its code-level findings, which are excellent and load-bearing here.
**Its recommendations are not inherited**: each decision below is made on its own reasoning, and where I
reach the same place, I say so and say why.

**Independence:** author-reviewed, the standing ceiling. Every claim cites the code.

## 1. What is being designed

**The shared data-model interface** — what one prikk repository sends another, what claims ride with it,
and what the receiver may do. **Not transport** (RFC 115 §3, four options open); the interface must be
carriable by a file, a file plus a basis, SSH with prikk on both ends, or a protocol, unchanged.

## 2. D1 — patch identity stays content-only; ordering travels in the artifact

**`parent_patch_ids` stays unpopulated. Ordering travels as an explicit manifest in the exchange
artifact, derived by the sender from block lineage it already holds.**

**The mechanism matches Checkpoint 1's recommendation; the reason does not, and the reason is what
makes it a decision.** Their argument was that deriving at export avoids changing local authoring. Mine
is stronger and constrains the future:

**`parent_patch_ids` is inside the canonical payload, so it is inside the object id
(`prikk-object/src/id.rs`).** Populating it would make **patch identity context-bound**: the same change
authored on two different bases would carry two different ids. That destroys the property the owner and
I settled on — **the patch id as the stable, globally citable identifier** (RFC 115 §2.5, §2.7) — which
is the whole basis on which divergent blocks were accepted without arbitration.

**So this is not a deferral of a nice-to-have. Populating the field would contradict a ruling already
made.** If a future increment wants context-bound patch identity, that is a change to what a patch id
*means*, and it needs its own RFC, its own migration reasoning, and RFC 114's frozen-surface analysis.

**The field stays reserved and empty.** The manifest, not the patch, carries order.

**Verified:** every construction site in the workspace sets `Vec::new()`, including both production ones
(`worktree_patch/node_authoring.rs:567`, `patch_inverse.rs:142`) — Checkpoint 1 §0, re-derived here.

## 3. D2 — there is no stored "accepted but unsealed" state; it is derived

**Checkpoint 1 recommended a pending-acceptance index shaped after `received_index.rs`. I am deciding
against it.**

**The state is already derivable from what exists.** `import_bundle` already writes received Patch
objects into the ordinary type-keyed object store (`bundle.rs:392`), where they are readable and
verifiable immediately. **"Accepted but not yet sealed" is exactly: a Patch object present in the store
and not reachable from any sealed Block.** `verify` already walks both sides of that.

**Why derive rather than store, and it is the project's own evidence:**

- **A new durable container is where this project's sharp defects have lived.** The author-key container
  produced two in one week — a refused import leaking one entry, then a multi-key import leaking `1..k-1`
  — both in a container with **no prune, no compaction, no repair**. A pending index would be a third
  such container, with its own crash-ordering, `doctor`, and recovery story.
- **Derived state cannot be corrupted, only recomputed.** An interrupted acceptance leaves a Patch object
  that is either present or absent; both are sound, and neither is a state requiring repair.
- **Nothing needs the sender's identity as a trust input.** DC-78's rule is that the receiver decides;
  authorship is carried by the AUTHOR signature. Provenance beyond that is informational, and
  informational state does not justify a durable index.

**What is given up:** a cheap "when did I accept this, and from whom" record. **If that is wanted later,
it is a report, not a trust input**, and can be added without changing this interface.

## 4. D3 — block recognition is a signed claim object, because a Block cannot stand alone

**Checkpoint 1 reached the same conclusion. The reason recorded here is different, and decisive.**

The attractive-looking alternative is to ship the **Block objects themselves** as evidence — no new type,
no new frozen pair, signature verification already built. **It does not work**, and the reason is
concrete: `validate_v2_lineage` walks a block's parents and fails with
`"format-2 parent Block {id} is missing"` (`block_state.rs:446-447`) when one is absent. **A Block
shipped without its lineage is an `Integrity` error in the receiver's `verify`** — so block-as-evidence
either drags the whole lineage along, which is block-level exchange again, or breaks verification.

**Therefore recognition is its own object: a claim, signed by the sender's maintainer key, asserting that
named patches were sealed into a named block.** It is verifiable by its own signature, needs none of the
block's lineage present, and is **never trust-conferring** — it names a `key_id` the receiver may not
have adopted, and must be reported exactly as `AuthorSignatureVerification::Unverifiable` is: visible,
never gating.

**Not `AttestationPayload`, and not an extension of it.** Its fields — `policy_version`,
`plugin_set_hash`, `results`, `is_reproducible_offline` — are a CI-conformance shape with no meaning
here, and RFC 114's frozen `(object_type, schema_version)` discipline makes a wrong schema expensive
to walk back.

**Consequences accepted deliberately:** this is a new frozen pair the moment it is first written, so it
needs a Gate A identity vector **in the same increment that lands it** (RFC 114 §4). And **selective
omission is unsolvable and must be documented**: a sender can truthfully sign claims about some patches
and stay silent about others. Every byte verifies; the receiver is still not told everything. **That is a
property of any claim system and is stated, not defended against.**

## 5. D4 — the patch-set digest

A canonical value answering *"are these two repositories the same?"* at the level where identity holds.

```
preimage := DOMAIN ("PRIKK-PATCH-SET-DIGEST-v1")
         || count (u64 BE)
         || sorted, deduplicated patch ids, 32 bytes each
digest   := sha256(preimage)
```

**Follows `state_root.rs`'s pattern, not `id.rs`'s** — a comparison value, not a storable object, so a
dedicated newtype rather than an `ObjectId`. **The count is hashed even when zero**, so an empty set is
distinguishable from a degenerate one; `state_root.rs` already does exactly this and the omission would
be found later by whoever compares two empty repositories.

**It is identity-bearing.** Two prikk versions must produce the same bytes over the same set or the check
means nothing across an upgrade. **Its preimage is documented in `release-compatibility.md` the day it
ships**, under RFC 114's frozen list — a promise nobody wrote down cannot be broken, only found absent.

## 6. D5 — what the receiver does, and why DC-78 is preserved

**Accept** — verify the AUTHOR signature against transported key material, write the objects. **Seal** —
a separate, explicit, local act under the receiver's own maintainer key.

**DC-78 is not weakened; it is not even engaged.** Its trust unit is `Block | RefState`
(`verify/objects.rs`), and a raw exchanged patch carries no maintainer seal at all. **The receiver's own
seal is the only seal this material ever gets in this repository** — and `verify_signer_trusted` is
checked before any seal is produced (`merge_execute.rs`), so sealing cannot happen except by a locally
adopted key.

**This is a stronger posture than today's**, and the point is worth keeping: today one adopted key admits
a sender's whole sealed chain; here every accepted patch is individually re-sealed by the receiver's own
act.

## 7. What a third party can verify — the owner's condition

**Globally, from the patch alone:** its content (content-addressed), its authorship (AUTHOR signature over
its own id), and — with transported key material — that the signature verifies. **Continuity, not
first-contact authenticity** (criterion 5's stated limit).

**Globally, across repositories:** the patch id, which is stable everywhere; and the patch-set digest,
which is equal exactly when two repositories hold the same patches.

**Locally, per repository:** block lineage, state roots, and publication trust — **which differ between
repositories by design and are not comparable across them** (RFC 115 §2.4-§2.7).

**Nothing above is weaker than today. What moved is where identity lives**, and every user-facing surface
must say so: **the patch id is what you cite; the block id is local publication detail.**

## 8. Security properties this interface must hold

From RFC 115 §5.1.2 and Checkpoint 1's threat model, as refusals:

1. **A refused exchange leaves nothing behind** — objects are content-addressed and harmless, but **no
   key material, and no claim, may be recorded from an exchange that failed.** Both author-key defects
   this week were exactly this.
2. **Trust never expands on receipt.** No artifact can cause a maintainer key to be adopted.
3. **A recognition claim is reportable, never gating** (D3).
4. **Missing closure refuses the whole exchange** — a referenced blob or parent patch absent is a refusal,
   not a partial apply.
5. **Declared counts and sizes are bounded before decoding**, to DC-86's standard: *a declared count over
   the limit must not cost more than reading one integer to reject.*
6. **A patch whose signature fails against transported material fails; one with no material reads
   `Unverifiable`** — never `Sound`.
7. **Replay is inert.** Re-receiving known patches records nothing new and changes no state.

**The precondition question — resolved 2026-08-19, and the answer removes it as a blocker.**

Checkpoint 1 raised `OperationCondition`'s variants as asserting **the sender's** worktree state, and I
promoted it to a blocking prerequisite. **Investigating it first found a prior fact that settles it:
preconditions are never evaluated anywhere.**

**Verified:** every occurrence of `preconditions` in `prikk-store` is `Vec::new()` — construction only —
and `OperationCondition` does not appear in `prikk-store` or `prikk-cli` at all. **The schema defines
them, the wire format carries them, and no code has ever checked one.**

**This is the third declared-but-unused surface found this week**, after `parent_patch_ids` (D1) and
`Attestation` (RFC 114 Gate A). The pattern is worth naming: **prikk's schema is consistently ahead of
its behaviour**, and any design reading capability from the schema must confirm the behaviour separately.
That mistake has now been made twice — once by me in RFC 115 §2.2, once here.

**Ruled: `accept` does not evaluate preconditions.** They stay inert, exactly as they are for locally
authored patches today.

- **Consistency:** exchange must not be the place a dormant mechanism silently acquires meaning. If
  preconditions should be checked, they should be checked for local patches first, and that is its own
  increment with its own reasoning.
- **The threat shrinks with it.** Checkpoint 1's item 4 assumed preconditions carry force; they do not,
  so a hostile sender's preconditions are as inert as an honest one's. **What remains is only that they
  are attacker-controlled bytes**, which §8's bounds already cover.
- **What is *not* ruled:** whether preconditions should ever be evaluated. That question is now clearer
  for having been asked — a third dormant surface — and belongs in its own record.

## 9. Deliberately excluded

- **Transport** (RFC 115 §3).
- **Populating `parent_patch_ids`** (D1) — a change to identity semantics, needing its own RFC.
- **A pending-acceptance container** (D2).
- **Canonical sealing** (RFC 115 §2.6) — declined with a stated reason; reopening it is the owner's.
- **The `merge_execute` fast-forward gap** — real, separable, improves the product regardless.

## 10. Staging

1. **The digest** (D4) — self-contained, no artifact format involved, immediately useful for "are we the
   same?", and it exercises RFC 114's documentation obligation on a small surface.
2. **The recognition claim object** (D3) — with its Gate A vector in the same increment.
3. **The exchange artifact and accept path** (D1, D2, D5). **No longer gated** — §8's precondition question is resolved.
4. **Sealing what you accepted** — **added 2026-08-20, after Stages 1-3 shipped at `0128c91`.** Not an
   afterthought and not optional: D5 says the receiver's own seal is "a separate, explicit, local act",
   but `seal` builds blocks from the active WAL and an accepted patch was never in the WAL, so no path
   from accepted object to sealed block exists. **Criterion 1 — "two machines can exchange sealed
   history" — cannot be met without it**, because under §2.2's patch-unit ruling the receiver is the one
   who seals. Scheduled ahead of transport by the owner, 2026-08-20. Its own design section is D6 below.

**Report before implementing each**, and §5.1's discipline — threat model, adversarial byte fixtures, a
two-repository harness, a negative control per refusal — applies to each stage, not once at the end.
