# DC-53 Repository-Wide AUTHOR Trust Verification — design v1

> **Stage 1 is complete and merged (2026-08-18, `970bc27`). This document is NOT the live instruction
> for Stage 2 — [`design-stage-2-v1.md`](design-stage-2-v1.md) is.**
>
> **It is not superseded either.** §3's decisions **D1-D5** still bind Stage 2, and §5's **v1.2
> amendment** — that author key material does not travel, so Stage 2 must carry transport *and* pinning
> together — is what set Stage 2's scope. Read this for the decisions; work from the Stage 2 design.

**RFC:** `rfcs/proposed/DC-53-REPOSITORY-WIDE-AUTHOR-TRUST-VERIFICATION.md`
**Brief:** `design-brief-v1.md` — this document is what that brief asked the design stage to produce.

**Scheduling.** The RFC and brief both say *"sequenced after DC-43."* **The project owner scheduled DC-53
first on 2026-08-18**, as item 1 of `EXECUTION-ORDER.md`'s recommended order, on the grounds that it is the
cheapest of the five badge criteria and the only one where the README promises what the code does not do.
**That ruling supersedes the inherited sequencing**, which was written when DC-43 was expected sooner and
before DC-43 became blocked on a release-lane event. Recorded here so the documents do not contradict each
other.

**Correction, 2026-08-18 — v1.1.** The dev team's Stage 1 pre-implementation report
(`.git-exclude/review-request/prikk-dc53-stage1-v1.md`, ruled on in
`.git-exclude/reviewed/DC-53-stage-1-report-ruling-v1.md`) established that **Stage 1 as staged in §5 was
not executable**: verifying an Ed25519 signature requires the public key, `Signature` carries only a
`key_id` label (`signature.rs:83`), Ed25519 keys are not recoverable from signatures, and **no `key_id →
public_key` mapping exists for authors anywhere in this codebase** — `trust_index.rs:61`'s
`TrustKeyEntry { key_id, public_key }` is MAINTAINER-only. **This design error was the architect's**: the
increment was split on validity-versus-membership without noticing that validity itself requires key
material, all of which had been placed in Stage 2. **§3's D3 and D4 and §5 are amended below.**

## 1. The finding that reframes this increment

The RFC and brief both describe the gap as: AUTHOR signatures are *"validated structurally — role,
algorithm, 64-byte Ed25519 shape — without being checked against any trust store."*

**That understates it. AUTHOR signatures are not cryptographically verified at all.** Established from the
code, not inferred:

- **`verify/objects.rs:298`** — `trust_verifier.verify(envelope)` is called **only** when
  `matches!(object_type, ObjectType::Block | ObjectType::RefState)`. A `Patch` object takes the `else`
  branch and reaches no verification.
- **`signature_diagnostics.rs:95`** — `classify_signature_envelope`, the only check a Patch's signatures
  receive, tests exactly three conditions: `malformed_shape`, `duplicate`, `noncanonical_order`. **No
  signature is ever verified against a key.**
- **`trust.rs:292`** — `verify_trusted_signature` rejects any signature whose `signer_role` is not
  `Maintainer`, so the existing machinery cannot be pointed at AUTHOR signatures without change.

**Consequence: a Patch carrying 64 arbitrary well-formed bytes as its AUTHOR signature passes `prikk
verify` today.** Not "signed by an untrusted key" — *not signed at all*, and reported as sound.

**This is an integrity gap, not only a trust gap**, and it is the more serious half. The product claim is
*"every change is signed by its author and verifiable by anyone."* Cryptographic validity delivers
**verifiable**; trust-store membership delivers **by someone I accept**. Only the second was scoped.

## 2. The increment splits, and the halves are very different sizes

| | Question answered | Needs |
|---|---|---|
| **Stage 1 — validity** | Is this patch actually signed by the key it names? | **Key material, and nothing else** — no policy, no adoption step, no conflict rejection. A valid signature is valid regardless of trust, but it cannot be checked without the key *(amended v1.1: the original read "no trust source", which made the stage unexecutable — see the correction above)* |
| **Stage 2 — membership** | Is that key one this repository accepts? | All five brief decisions |

**Stage 1 carries most of the claim and almost none of the decisions.** It is also the half that closes an
integrity hole rather than a policy one, and it should ship first and separately.

## 3. The five decisions, made

### D1 — Trust source: TOFU with pinning, not an adopted set

**Ratified, not inherited.** The external design's §13.4 offers TOFU for local repositories; the reasoning
below is prikk-specific.

**Mirroring DC-11/DC-78's maintainer model would be wrong for authors.** Maintainers are few and adopted
deliberately; authors are many, arrive with imported history, and accumulate over a project's life.
Requiring adoption per author would make `bundle import` demand adoption of every historical contributor —
turning a verification feature into an administrative one.

**And admission control already exists elsewhere.** A maintainer seals patches into a block, and that
maintainer signature *is* verified against the trust policy. Trusting the maintainer already covers the
decision to include those patches. So an independent AUTHOR admission list would duplicate a judgement the
maintainer chain already carries.

**What AUTHOR trust adds that nothing else provides is consistency**: the same `key_id` must always carry
the same public key. That is what pinning gives, and it catches the attack that matters — someone
authoring under another author's `key_id` with a different key. DC-78 already enforces TOFU on conflicting
re-add for maintainers, so the concept exists in this codebase and this extends it rather than inventing a
second model.

### D2 — Scope: every reachable Patch

Not "reachable from published refs." A narrower scope creates a class of objects that are present,
readable, and unverified — which is the gap being closed, reintroduced in smaller form.

**Cost must be measured, not assumed.** One Ed25519 verification per patch is O(N) against a `verify` that
is already roughly O(N³) (badge criterion 3), so it should be small in proportion — **but "should be" is
not a measurement.** Report the delta on a real repository before and after.

### D3 — Failure semantics: three outcomes, extending today's separation rather than collapsing it

Today `verify` separates *structural corruption* from *publication-trust failure*. That distinction is
deliberate and this design keeps it, adding a third:

**Amended v1.1 — a fourth outcome.** Because AUTHOR key material is recorded at authoring time (see D4),
a Patch may name a `key_id` for which **no key material exists**. That is neither valid nor invalid, and
the original three rows collapsed it into one of them.

| Condition | Class | Exit status |
|---|---|---|
| Verifies against recorded key material | Sound | Passes |
| **No key material recorded for this `key_id`** | **Unverifiable — reported, never silent** | Passes, and says so |
| Signature does not verify against recorded key material | **Authorship-integrity failure** | **Fails.** This is forgery or corruption, not a trust opinion |
| `key_id` recorded with a **different** public key | **Authorship-integrity failure** | **Fails.** This is impersonation |

**The third row is the important one.** A signature that does not verify is not "untrusted" — it is
evidence the object is not what it claims. Reporting that as a trust warning would repeat the mistake of
treating a broken guarantee as a policy preference.

**The second row must appear in `verify` output.** A repository where most patches are unverifiable must
say so; the difference between "verified" and "nothing objected" is the whole point of this increment.

### D4 — Migration: no grandfathering for validity; pins bootstrap naturally

**Amended v1.1.** "No grandfathering" is retained **for signatures that fail**, and is **not a policy this
design can hold for signatures that cannot be evaluated.** A signature that does not verify was never
valid; admitting it because it is old would make the check meaningless on exactly the repositories that
predate it. But **every Patch authored before this increment has no recorded key material and can never be
verified by any future work** — that is unverifiable in principle, not leniency, and D3's second row is
where it belongs. The original text collapsed the two.

**AUTHOR key material is recorded at authoring time**, by the signer, which is the only party that holds
it. Recording it on first *sight* is impossible: the signature carries no key, so observing one teaches
nothing about the key it names. This is why the mechanism differs from DC-78's maintainer TOFU, where
adoption is itself the act that supplies the key.

**Run Stage 1 against real repositories and report what happens.** If something *fails* — as opposed to
being reported unverifiable — that is a finding and it stops the stage.

**Stage 2 needs no migration step.** Pins are established on first verification, so an existing repository
bootstraps its pin set by being verified once. **No persisted byte is rewritten**, per the RFC's
constraint.

### D5 — Key lifecycle: deferred explicitly, with the consequence stated

Rotation, revocation and expiration remain unimplemented and out of scope.

**The consequence must be recorded rather than discovered: under pinning, a legitimate author key rotation
is indistinguishable from impersonation.** Both present as the same `key_id` with a different public key,
and D3 fails both.

That is the price of shipping pinning before lifecycle, and it is acceptable only because it **fails
closed** — a real rotation produces a loud, specific error rather than silent acceptance. **It must be
documented in `trust-threat-model.md` as a known operational cost**, not left for the first person who
rotates a key to discover.

## 4. The verification contract, and the vectors it needs

DC-40's precedent applies: literal vectors so the contract is reviewable before code exists.

**The design stage must produce**, as committed fixtures:

1. **A known Ed25519 keypair** — literal bytes, not generated at test time.
2. **A known Patch payload** and its **canonical signature preimage bytes**, so the preimage construction
   is pinned independently of the signing code.
3. **A valid AUTHOR signature** over that preimage, verifying.
4. **A mutated signature** — one bit — which must fail.
5. **A signature valid over a different preimage**, which must fail against this one. This is the vector
   that catches a preimage-construction error, and it is the one most likely to be omitted.
6. **A pin-conflict pair**: the same `key_id` with two different public keys.

**Constraint from DC-39/DC-40, unchanged:** no change to signature preimage bytes, canonical encoding, or
object identity. If the preimage this design must verify over turns out not to be constructible from what
is persisted, **stop and report** — that would be a finding about DC-39's surface, not something to work
around here.

## 5. Staging

**Amended v1.1 — restaged.**

**Stage 1 — record and report.**
- **Persist AUTHOR key material at authoring time** (`author_signing.rs`'s path), in a store shaped after
  `trust_index.rs`'s key-material half. **Material only — no admission judgement, no conflict rejection.**
- Widen `verify/objects.rs:298`'s type gate so `Patch` reaches verification. D2 needs nothing further: the
  existing scan already visits every record in every persisted-type container, with no reachability filter.
- D3's first three rows; D4's authoring-time recording; vectors 1–5 (committed).

**Stage 2 — pin and reject.** D1's TOFU conflict semantics, D3's fourth row, D5's documentation, vector 6.

**Added v1.2, 2026-08-18, after Stage 1 merged at `970bc27`. Stage 2's scope is larger than "local
pinning", and badge criterion 5 is not closed by Stage 1.** AUTHOR key material is recorded at authoring
time in the authoring repository, and **it does not travel** — `bundle.rs` references neither
`author_key_container_path` nor `author_key_index`. So a patch received from another party is
**permanently `Unverifiable` on the receiver's side**, which is exactly the case criterion 5 names when it
warns that *"shipping exchange makes this criterion more important, not less, since other people's history
then arrives with authorship unchecked."* Stage 1 delivers *verifiable by its author's own repository*;
criterion 5's claim is *verifiable by anyone*.

**Stage 2 must therefore carry author key material across the exchange path**, not only pin it locally.
The material is then attacker-supplied and proves only self-consistency — which is precisely why pinning
is what gives it meaning: TOFU on first receipt, and a conflicting key for a known `key_id` is
impersonation. **Pinning without transport verifies nothing for a receiver; transport without pinning
verifies nothing at all. Stage 2 needs both, and neither is useful alone.**

Putting the public key in the `Signature` or `PatchPayload` instead is **not available** — DC-39/DC-40
freeze the preimage, canonical encoding and object identity, and the RFC forbids rewriting persisted
bytes. Material must travel *alongside* objects, as its own container, the way it is stored.

**What Stage 1 honestly closes:** every Patch authored after it can be verified, and every Patch before it
is reported unverifiable rather than reported sound. **The integrity gap closes going forward and becomes
visible looking backward** — the most achievable without rewriting persisted bytes, which the RFC forbids.

**Report before implementing each stage**, per this project's standing shape.

## 6. What this increment does not grant

Unchanged from the brief, restated because they will be assumed otherwise:

- **No public "publication-grade trust" claim.** Closing this gap is necessary for such a claim, not
  sufficient — key lifecycle remains open.
- **No key rotation, revocation, expiration, thresholds, hardware signing, or remote trust distribution.**
- **No release-lane action.**
- **No identity-bearing byte change.**
