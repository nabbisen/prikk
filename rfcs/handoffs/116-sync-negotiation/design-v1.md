# RFC 116 — negotiation: design v1

**RFC:** `rfcs/accepted/116-sync-negotiation-and-transport.md` (ACCEPTED 2026-08-20, both rulings).
**Builds on:** RFC 115 Stages 1-4 and D6, all merged (`07d8a47`).
**Independence:** author-reviewed, the standing ceiling.

**What this designs:** how two repositories determine what one lacks, using messages that move over any
channel. **No network code. `prikk-store` stays bytes-in, bytes-out** (RFC 116 ruling 2).

---

## 1. N1 — the message set

Three artifacts. Two are new; the third already exists.

| Artifact | Magic | Direction | Contents |
|---|---|---|---|
| **Sync summary** | `PSYNCSU1` | published / fetched | per ref: name, `PatchSetDigest`, patch count |
| **Have-list** | `PSYNCHV1` | receiver → sender | per ref: name, digest, full patch-id list |
| **Exchange artifact** | `PEXCH001` | sender → receiver | *(exists — RFC 115 Stage 3)* |

**All three are representational, not frozen** (RFC 114 §3): they carry objects whose identity is
already frozen and have no identity of their own. Say so in each module doc, and do not treat that as
licence for carelessness.

### 1.1 Why the summary is separate from the have-list

The summary is **32 bytes plus a name per ref**. It answers "are we the same?" without transferring a
patch-id list at all — so the steady-state case, two repositories already in sync, costs a few hundred
bytes rather than 32 × N.

This is §3.1b's *published basis*, made concrete: a repository can publish its summary somewhere
cheap, and anyone can compare against it before deciding whether a real exchange is worth starting.

### 1.2 The flow

1. **B obtains A's `PSYNCSU1`** — by any means. Compares each ref's digest to its own. **All equal →
   in sync, stop.**
2. **For refs that differ, B sends A a `PSYNCHV1`** carrying B's patch-id lists for exactly those refs.
3. **A computes A∖B** per ref and builds a `PEXCH001` carrying those patches, their blobs, the author
   key material for them, and the claims covering them (§3).
4. **B accepts it** (Stage 3), then **seals** (Stage 4).

**One round trip after the summary.** Nothing else is required, and nothing here is a conversation —
each step is a blob that can be written to a file and moved by any means the user already has.

### 1.3 Self-consistency check on a have-list

`PSYNCHV1` carries **both** the digest and the list per ref, and the receiver of that message
**recomputes the digest over the list it was sent and refuses on mismatch.** The redundancy is
deliberate: it costs 32 bytes and turns a truncated or reordered list into a refusal instead of a
silently wrong delta. Reuses `compute_patch_set_digest` unchanged.

---

## 2. N2 — negotiation messages are unsigned, and that is correct

**Ruled: `PSYNCSU1` and `PSYNCHV1` carry no signatures.**

A lying negotiation message can cause exactly two things: the sender sends **more** patches than needed
(wasteful, harmless — the receiver deduplicates by content address), or **fewer** (the receiver stays
behind and its next summary comparison says so). **Neither is a verification failure**, because every
byte that finally arrives is verified by Stage 3's accept path against material it carries, and Stage 4
seals only under the receiver's own key.

Signing them would imply the negotiation is trust-bearing. It is not, and implying otherwise is worse
than leaving it plain.

**The one real property to state, not defend against: negotiation discloses metadata.** A have-list
reveals which patch ids a repository holds — their existence and identity, never their content. For a
private project that is still information, and a user choosing a channel should know it. **Document it;
do not build a countermeasure**, on the same terms D3 states selective omission.

---

## 3. N3 — the inter-claim ordering gap, found while designing this

**A claim carries `block_id` and `patch_ids` and nothing about its block's parents.** Stage 4 seals
**one claim per call** (`seal_from_accepted_claim`). So when a delta spans two blocks, the receiver
holds two claims and **cannot derive which to seal first** — and sealing them in the wrong order either
fails to apply or produces a different history.

This does not arise in RFC 115's own tests because they exercise single-block cases. **It arises the
first time real sync sends more than one block's worth of patches, which is the normal case.**

### 3.1 The ruling

**The claim carries the block's `parent_block_ids` verbatim, by the same faithful-projection principle
D6 established for `patch_ids`.** Inter-claim order then follows from a topological sort over the batch:
a claim's block is sealed after any claim in the same batch whose block is one of its parents.

Why this rather than "the artifact's claim sequence is authoritative":

- **The order becomes signed.** It lives in the claim payload, therefore in the object id, therefore in
  the signature preimage — exactly the argument D6 §11.5 made for patch order, at the level above.
  An unsigned artifact field would be a second, weaker source of truth for the same kind of fact.
- **It is derivable rather than asserted.** A topological sort has a defined answer or a defined
  failure; a sequence has neither.
- **It is more of the projection the claim already is**, not a new concept.

**The order remains a hint that must be *tried*, never a fact that is *trusted*** (D6 §11.6, unchanged).
A hostile parent graph can only produce a seal that fails or a different valid application; it cannot
forge a state.

### 3.2 The window is open now and closes the same way D6's did

**Verified while writing this: there is still no production path that constructs a
`RecognitionClaimPayload`** — the only non-test construction site remains
`prikk-object/src/vectors.rs:151`, the Gate A snapshot generator. Stage 4 *consumes* claims; it does not
create them. **No release has ever written one.**

So this is again a free amendment in `schema_version` 1, and again **zero frozen bytes move**: a claim
whose block has no parents encodes identically, because `repeated_object_id` over an empty list writes
nothing — and the frozen vector `recognition_claim_populated` has no parents. **Add a second vector
covering a claim *with* parents**; no existing row moves.

**This is the second time this window has paid for itself.** The first producer will be whatever builds
the `PEXCH001` sender side. **Amend before that exists**, or this becomes a schema 2 with two contracts
forever — exactly D6's situation, repeating because the same surface is still unfinished.

---

## 4. N4 — which patches, which claims

**Ruled: the delta is computed per ref, and the artifact carries the union.**

For each ref where the digests differ, A computes `reachable(A.ref) ∖ B.list`. The union across refs is
the patch set to send. `patch_ids_reachable_from_block` already produces the operand and is exported.

**Claims to include: every stored claim whose `patch_ids` intersect the delta.** A claim covering
patches partly held and partly sent is still needed — Stage 4 refuses a partially-sealed claim, so
sending it is what lets the receiver notice, rather than discovering the gap later with less context.

**Blobs and author key material** follow Stage 3's existing export rules unchanged.

---

## 5. N5 — security properties this design must hold

Stated as refusals, in the shape RFC 115 §8 established:

1. **A negotiation message never changes state.** Reading a summary or a have-list writes nothing,
   records nothing, and adopts nothing.
2. **Declared counts and sizes are bounded before decoding**, to DC-86's standard: *a declared count
   over the limit must not cost more than reading one integer to reject.*
3. **A have-list whose digest disagrees with its own list is refused** (§1.3).
4. **A malformed or hostile negotiation message can cause at most a wrong delta, never a wrong
   acceptance.** Everything that arrives still goes through Stage 3's accept unchanged.
5. **Nothing here listens on a network, opens a socket, or adds a runtime dependency.** The workspace's
   third-party runtime surface stays at five crates.
6. **A ref present in one repository and absent in the other is not an error.** Ref sets differ
   legitimately; report the asymmetry, refuse nothing.

---

## 6. Deliberately excluded

- **Any protocol or network code** (RFC 116 ruling 2).
- **Set reconciliation** — Bloom/IBLT. RFC 116 §3(iv): revisit on measurement, not before.
- **Discovery and remote identity.** DC-78 §D4 left these out; this does not reopen them.
- **The repository-complete artifact** — still wanted by RFC 114 §5.2 and paikuli, still not on
  criterion 1's path. Its own RFC.
- **Confidentiality.** RFC 116's stated forgone property: the user's channel supplies it or nothing does.

## 7. Staging

1. **The claim's `parent_block_ids` amendment** (N3) — small, and **must land before any claim
   producer exists**.
2. **The two negotiation artifacts and the delta computation** (N1, N4).
3. **The sender side that builds a `PEXCH001` from a have-list** — the first claim producer, and
   therefore strictly after 1.

**Report before implementing each**, and RFC 115 §5.1's discipline applies to each stage, not once at
the end.
