# RFC (proposed) - DC-53 Repository-Wide AUTHOR Trust Verification

**Status.** Proposed; design review required. This is a **feature** increment, not a corrective or
assurance one, and it is the largest remaining item from the original architect review's trust findings.
**Target milestone.** Unscheduled — post-M2 at the earliest. Sequenced after DC-43 (release security
controls) because both touch trust posture and public claims.
**Schedule position.** Not startable until scheduled by the project owner. Recorded now so the gap is
owned rather than resurfacing as a discovery.
**Tracks.** The AUTHOR-trust portion of architect review N4, which DC-41 explicitly did not deliver
(DC-41 covers evidence gaps; this is a capability gap).
**Touches.** `prikk-store` verification, trust policy, and public trust documentation. Identity-bearing
behaviour change — requires a companion design document before implementation.

## Problem

`prikk verify` checks publication (MAINTAINER) signatures against repository-local trust policy, but does
**not** verify AUTHOR signatures repository-wide. AUTHOR signatures on Patch objects are validated
structurally — role, algorithm, 64-byte Ed25519 shape — without being checked against any trust store.

Consequently a repository can contain Patches signed by keys that no policy ever admitted, and `verify`
reports success. The current documentation is honest about this (`docs/src/reference/trust-threat-model.md`
and the DC-24 caveat blocks state it plainly), so this is a disclosed limitation rather than a false
claim. But it is a limitation that any production or public-preview readiness claim would have to close,
because "signed, verifiable history" is a central product claim and AUTHOR identity is most of that
history.

## Design sketch (requirements level; detailed design deferred to design review)

The increment must decide and record:

1. **Trust source for AUTHOR keys.** Repository-local trust store (mirroring DC-11's maintainer model),
   TOFU with pinning, or an explicit policy file. The external design's §13.4 offers TOFU for local repos
   and explicit stores for enterprise; that choice needs ratifying rather than inheriting.
2. **Verification scope.** Every reachable Patch, or only those reachable from published refs. The
   distinction matters for cost on long histories and for what a failure means.
3. **Failure semantics.** Whether an unknown AUTHOR key is a structural corruption error, a
   publication-trust error (as MAINTAINER failures are today), or a warning — and whether it blocks
   `verify` exit status.
4. **Migration.** Existing repositories contain Patches signed before any AUTHOR trust store existed.
   The increment must define whether those are grandfathered, quarantined, or require an explicit
   trust-store bootstrap — without rewriting any persisted byte.
5. **Interaction with key lifecycle.** Rotation, revocation, and expiration remain unimplemented. The
   design must state how it behaves when a key legitimately changes, or explicitly defer that with a
   recorded consequence.

Because this changes what `verify` accepts, it is identity-adjacent and requires a companion design
document with vectors before implementation, following the DC-40 precedent.

## Non-goals

- No key rotation, revocation, expiration, threshold, or hardware-signing implementation — those remain
  a separate unscheduled area.
- No change to signature preimage bytes, canonical encoding, or object identity (DC-39/DC-40 own those
  and are frozen).
- No remote trust distribution.
- No public "publication-grade trust" claim as a side effect of this increment.

## Acceptance criteria

The trust source, verification scope, failure semantics, migration posture, and key-lifecycle interaction
are each recorded as an explicit decision; a companion design document defines the verification contract
with vectors; existing repositories have a defined and evidenced outcome; and the public trust
documentation is updated to match the new behaviour rather than to anticipate it.
