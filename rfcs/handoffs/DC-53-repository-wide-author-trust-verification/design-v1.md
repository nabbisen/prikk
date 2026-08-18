# DC-53 Repository-Wide AUTHOR Trust Verification — design v1

**RFC:** `rfcs/proposed/DC-53-REPOSITORY-WIDE-AUTHOR-TRUST-VERIFICATION.md`
**Brief:** `design-brief-v1.md` — this document is what that brief asked the design stage to produce.

**Scheduling.** The RFC and brief both say *"sequenced after DC-43."* **The project owner scheduled DC-53
first on 2026-08-18**, as item 1 of `EXECUTION-ORDER.md`'s recommended order, on the grounds that it is the
cheapest of the five badge criteria and the only one where the README promises what the code does not do.
**That ruling supersedes the inherited sequencing**, which was written when DC-43 was expected sooner and
before DC-43 became blocked on a release-lane event. Recorded here so the documents do not contradict each
other.

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
| **Stage 1 — validity** | Is this patch actually signed by the key it names? | **No trust source, no policy, no adoption step.** A valid signature is valid regardless of trust |
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

| Condition | Class | Exit status |
|---|---|---|
| Signature does not verify against its own claimed key | **Authorship-integrity failure** | **Fails.** This is forgery or corruption, not a trust opinion |
| `key_id` seen for the first time | Pin recorded | Passes |
| `key_id` seen before with a **different** public key | **Authorship-integrity failure** | **Fails.** This is impersonation |

**The first row is the important one.** A signature that does not verify is not "untrusted" — it is
evidence the object is not what it claims. Reporting that as a trust warning would repeat the mistake of
treating a broken guarantee as a policy preference.

### D4 — Migration: no grandfathering for validity; pins bootstrap naturally

**Stage 1 grandfathers nothing.** A signature that does not verify was never valid; admitting it because it
is old would make the check meaningless on exactly the repositories that predate it. **Run it against real
repositories and report what happens** — if prikk's own authoring path has always signed correctly, nothing
fails, and that is evidence rather than assumption. If something does fail, that is a finding and it stops
the stage.

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

**Stage 1 — cryptographic validity.** D2, D3's first row, D4's first half, and vectors 1–5. No trust store,
no pinning, no policy. Closes the integrity gap.

**Stage 2 — pinning.** D1, D3's remaining rows, D4's second half, D5's documentation, vector 6.

**Report before implementing each stage**, per this project's standing shape.

## 6. What this increment does not grant

Unchanged from the brief, restated because they will be assumed otherwise:

- **No public "publication-grade trust" claim.** Closing this gap is necessary for such a claim, not
  sufficient — key lifecycle remains open.
- **No key rotation, revocation, expiration, thresholds, hardware signing, or remote trust distribution.**
- **No release-lane action.**
- **No identity-bearing byte change.**
