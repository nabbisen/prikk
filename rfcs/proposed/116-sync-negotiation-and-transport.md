# RFC 116 — Sync: negotiation before transport

**Status.** **PROPOSED, 2026-08-20.** An investigation with a recommendation; **not a design, and
implementation must not start from it.** Written on the owner's direction after RFC 115 closed
criterion 1's first gap (Stages 1-4 merged, `07d8a47`). **It asks the owner to rule on one thing:
whether sync's next increment is negotiation-as-artifacts or a network protocol.**

**Independence:** author-reviewed, the standing ceiling. Every claim cites the code or record it came
from.

---

## 1. RFC 115 §3 is stale in a way worth stating precisely

§3 already carries its own warning — *"Superseded as the primary question by §2.1. The four options
below remain open."* Two further things have changed since it was written, and both narrow the problem.

**(a) §3 predates the patch-unit ruling.** Its central recommendation was to *"build the
repository-complete artifact, and do not call it sync"*, resting on three converging needs: RFC 114
§5.2's format-7 carry-forward, paikuli's prikk encoder, and criterion 1. **That reasoning survives for
migration and carry-forward and no longer applies to sync** — what shipped is `PEXCH001`, a
*patch-level* artifact carrying patches, blobs, author key material and recognition claims, and
deliberately not a repository. The repository-complete artifact is still unbuilt and still wanted by
the other two needs; **it is simply no longer on criterion 1's path.**

**(b) §3 predates the interface existing.** It weighed four transport options against a design that had
not been built. The interface is now real, and what it needs from a transport is far more specific than
§3 could have known.

## 2. The gap is negotiation, not transport — and this is the same reframing the owner already made

**`PEXCH001` requires the sender to know which patches to send.** Nothing in RFC 115 answers how it
learns that.

What exists today:

- `compute_patch_set_digest_for_ref` answers **"are we the same?"** — one bit. Equal or not.
- `patch_ids_reachable_from_block` produces the full patch-id list for a ref, and is **already public
  and exported** (`prikk-store/src/lib.rs:154-155`).
- Nothing exchanges either one.

So the local primitives for computing a difference exist, and the missing piece is that **no two
repositories can tell each other what they hold.** A digest tells you that you differ; it cannot tell
you *how*. Without that, "incremental exchange" degrades to "send everything and let the receiver
deduplicate", which §3.1's reason 2 already ruled is not what criterion 1 is for.

**This is the owner's own 2026-08-19 reframing, arriving one layer down:** *"the subject is not network
connection (exchange) but design on data model interface on what to be shared."* It was right about the
artifact and it is right again here. **Negotiation is a data-model question. Transport is a delivery
question. They are separable, and the first one is both harder and more valuable.**

## 3. Negotiation — four shapes, and the recommendation

**(i) Exchange the full patch-id list.** 32 bytes per patch. A 10,000-patch history is 320 KB; 100,000
is 3.2 MB. Exact, no false positives, no new dependency, and it reuses
`patch_ids_reachable_from_block` unchanged.

**(ii) Digest short-circuit first.** Exchange `PatchSetDigest` (32 bytes). Equal → nothing to do, stop.
**This already exists and costs nothing to use.** It makes the common steady-state case — two
repositories already in sync — a 32-byte exchange rather than a 320 KB one.

**(iii) Ref-scoped narrowing.** Digest per ref, then lists only for refs that differ. Bounds the list
exchange to the part of history that actually diverged.

**(iv) Set reconciliation — Bloom filters, IBLT.** Sublinear in history size. **Recommended against, for
now.** prikk's entire third-party runtime surface is five crates (`ed25519-dalek`, `getrandom`,
`rustix`, `sha2`, `windows-sys`); set reconciliation is either a new dependency or a substantial piece
of first-party probabilistic code whose failure mode is **silently omitting a patch**. Sending 32 exact
bytes per patch is honest, and nobody has yet measured a history where it hurts. **Revisit when there is
a measurement, not before.**

**Recommendation: (ii) → (iii) → (i), composed.** Digest short-circuit, then per-ref narrowing, then an
exact list for what differs. Every part reuses something already built and shipped.

## 4. Transport, once negotiation is a set of messages

Negotiation shaped as **messages** rather than as a live conversation has a property worth naming: a
digest, a list, an artifact, and a reply are four blobs. **Four blobs move over anything** — a file
copy, a USB stick, email, a shared drive, HTTP, SSH.

So the recommendation is to **build negotiation first and ship sync over any channel a person already
has, with no network code at all.** Then, if convenience demands it, add a transport that automates
moving those same four blobs.

**This makes the transport choice reversible**, which is the strongest argument available for
sequencing it second. §3.2 hoped for exactly this composition — *"a protocol can then be added later as
a transport over exactly the same artifact and basis"* — and called it unproven for option (b) alone.
**Negotiation-as-messages is what makes it proven rather than hoped-for**, because the messages are the
interface and the channel is an implementation detail beneath them.

Against §3's four options, this is closest to **(c) artifact plus a published basis** — with the
negotiation messages playing the basis role, and now concrete rather than sketched.

## 5. The security surface, which is the real reason to sequence it this way

Everything RFC 115 built is **offline verification of bytes that arrived somehow**. Every refusal it
tests — bounded counts, closure completeness, signature verification, replay inertness, trust never
expanding — assumes the bytes are already local.

**A network endpoint is a different kind of surface**: a listening process, remote-triggered work before
authentication, resource exhaustion, transport authentication and its key management, and a much larger
dependency graph. None of that is covered by RFC 115 §5.1's discipline, which was written for artifacts.

**Recommendation: `prikk-store` stays bytes-in, bytes-out, and prikk itself stays off the network in
this increment.** If a protocol is later wanted, it belongs in its own crate or its own binary, built
against the same messages, so that the verification core never grows a listening socket. This preserves
the property that makes prikk auditable — **the trust-bearing code has no network in it at all** — and
it is the option the owner's standing instruction points at: *"Security is strongly prioritized to
function. Secure by default. Proactive security in mind. We should not be in a hurry especially on it."*

## 6. What is recommended, and what the owner must rule

**Recommended:** negotiation next, as messages, with no network code. Digest short-circuit, per-ref
narrowing, exact patch-id lists. Transport deferred and kept outside the verification core.

**The owner rules on one question:** is sync's next increment **negotiation-as-artifacts** — which
completes criterion 1's mechanism and leaves delivery to whatever channel a user already has — or a
**network protocol**, which is larger, is the biggest security surface this project has taken on, and
which negotiation would have to exist underneath anyway?

**A secondary question, only if the first is answered "negotiation":** whether criterion 1's row can be
met by sync-over-any-channel, or whether the owner reads *"two machines can exchange sealed history"* as
requiring prikk to move the bytes itself. **That is a reading of the criterion, and readings of criteria
are the owner's to confirm** — RFC 111 §8's precedent, and stated here before the work rather than
after.

## 7. Non-goals

- **Designing any protocol.** Not until §6's first question is answered.
- **The repository-complete artifact** — still wanted by RFC 114 §5.2's carry-forward and by paikuli,
  still not on criterion 1's path (§1a). Its own RFC.
- **Set reconciliation** (§3 iv) — revisit on measurement.
- **Discovery, identity of remotes, or remote-tracking semantics.** DC-78 §D4 left these out
  deliberately and this does not reopen them.
