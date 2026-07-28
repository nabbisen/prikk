# DC-53 Repository-Wide AUTHOR Trust Verification - Design Brief

**This is a design brief, not an implementation handoff.** DC-53's own RFC requires a companion design
document with vectors *before* implementation, so writing implementation instructions now would
contradict it. This specifies what the **design stage** must produce. An implementation handoff follows
once that design is accepted.

**Authored by** the architect (function-designer role).
**Stage gate:** proposed, **post-M2, unscheduled**. Design work may not begin until the owner schedules
it. Sequenced after DC-43 because both touch trust posture and public claims.
**Assigned to:** architect for design; developers for implementation after design acceptance.

## The gap, stated precisely

`prikk verify` checks **publication (MAINTAINER)** signatures against repository-local trust policy. It
does **not** verify **AUTHOR** signatures repository-wide. AUTHOR signatures on Patch objects are
validated structurally — role, algorithm, 64-byte Ed25519 shape — without being checked against any trust
store.

So a repository can contain Patches signed by keys no policy ever admitted, and `verify` reports success.

Two things to hold at once: the documentation is **honest** about this today
(`docs/src/reference/trust-threat-model.md` and the DC-24 caveat blocks state it plainly), so this is a
disclosed limitation, not a false claim. And it is nonetheless the largest remaining gap between the
product's central claim — signed, verifiable history — and what `verify` actually enforces, because
AUTHOR identity is most of that history.

This is a **capability** gap, not an evidence gap. DC-41 could not close it and correctly did not try.

## Decisions the design must make and record

1. **Trust source for AUTHOR keys.** Repository-local trust store mirroring DC-11's maintainer model,
   TOFU with pinning, or an explicit policy file. External design §13.4 offers TOFU for local repos and
   explicit stores for enterprise — that needs **ratifying**, not inheriting.
2. **Verification scope.** Every reachable Patch, or only those reachable from published refs. This
   determines cost on long histories and changes what a failure means.
3. **Failure semantics.** Is an unknown AUTHOR key structural corruption, a publication-trust error (as
   MAINTAINER failures are today), or a warning? Does it change `verify`'s exit status? Today's separation
   of *structural corruption* from *publication-trust failure* is deliberate and should be extended
   consciously rather than collapsed.
4. **Migration.** Existing repositories contain Patches signed before any AUTHOR trust store existed.
   Grandfathered, quarantined, or requiring an explicit trust-store bootstrap — decided **without
   rewriting any persisted byte**.
5. **Key-lifecycle interaction.** Rotation, revocation, and expiration are unimplemented and unscheduled.
   Define behaviour when a key legitimately changes, or defer explicitly with the consequence recorded.
   Do not silently assume keys are permanent.

## Why this needs a companion design document with vectors

It changes what `verify` **accepts**. That is identity-adjacent: a repository that verifies today could
stop verifying, or vice versa. The DC-40 precedent applies — a companion FDD with literal vectors, so the
contract is reviewable before code exists rather than inferred from an implementation.

## Constraints inherited from accepted work

- **No change to signature preimage bytes, canonical encoding, or object identity.** DC-39 and DC-40 own
  those and they are frozen.
- **No key rotation, revocation, expiration, thresholds above one, hardware signing, or remote trust
  distribution** — separate unscheduled area, explicitly out of scope here.
- **No public "publication-grade trust" claim** as a side effect of this increment. Closing the
  verification gap is necessary for such a claim, not sufficient — key lifecycle remains open.

## What the design stage must deliver before an implementation handoff exists

- The five decisions above, each recorded explicitly with its rationale.
- A companion design document defining the verification contract with literal vectors.
- A defined, evidenced outcome for repositories that predate any AUTHOR trust store.
- The public trust documentation updated to describe the new behaviour **after** it exists — not to
  anticipate it.

## Standing boundaries

No release-lane action. No identity-bearing byte change. Production, public-preview, and format-stability
claims remain separate decisions that this increment does not grant.
