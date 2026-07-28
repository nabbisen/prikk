# DC-50 First-Party SHA-256 ROI Decision - Handoff

**Prepared in advance.** Work may **not** begin until `rfcs/proposed/DC-50-…` moves to `rfcs/accepted/`
through design review.
**Authored by** the architect (function-designer role). Review remains independent.
**Size:** small — this produces a **decision record**, not code.
**Touches:** one document. **No code, no dependency, no manifest, no CI.**

## What this is

DC-41 closed the *evidence* half of architect finding N6. DC-50 closes the *ROI* half: should prikk keep
maintaining a first-party SHA-256 implementation, now that its correctness has independent evidence?

This is unusual for a developer handoff in that the deliverable is analysis, not a diff. Resist the pull
to start implementing either outcome — replacing the implementation is explicitly a **separate** RFC that
this decision may authorize but does not perform.

## Evidence already available (do not re-derive)

| Source | Content |
|---|---|
| DC-40 review | 11 state-root vectors reconstructed under Python `hashlib`, all matching |
| DC-41 stage 2 (`d5bd096`) | 11 fixed vectors — 4 canonical published (FIPS 180-2 / RFC 6234) + 7 independently computed — over 10 distinct lengths spanning both padding transitions |
| DC-41 stage 3 (`540d4db`) | 10,000 randomized cases against RustCrypto `sha2`, fixed seed, 957 distinct lengths, zero mismatches |

Total: **10,022 agreeing comparisons against two independent implementations.**

State its limit honestly in the record: a fixed seed means 10,000 *specific* inputs, not 10,000
independent samples of the space. This is strong evidence, not proof.

## The five questions to answer

1. **Correctness assurance** — what DC-41 established and what it did not.
2. **Maintenance cost** — ongoing burden of a hand-written cryptographic primitive. Note the compounding
   factor: every future change to it is an *identity-bearing* change requiring vector re-verification, so
   the cost is not just reading the code once.
3. **Replacement cost and risk** — `sha2 0.10.9` is **already a production dependency** (transitively via
   `ed25519-dalek`, and directly in `tools/release-policy`), so replacement removes code without adding a
   dependency. But it is identity-bearing: any behavioural difference would alter every ObjectId in
   existence. DC-41's evidence bounds that risk; it does not eliminate it.
4. **Supply-chain trade** — first-party code has no upstream compromise surface but no external review;
   `sha2` has broad external review but is an upstream dependency. Note that `sha2` is *already* trusted
   in the runtime signing path, so this trade is partly made already.
5. **Reversibility** — can the decision be revisited later without a format break?

## The two acceptable outcomes

- **Retain** — with the rationale recorded, plus the conditions that would reopen the question (e.g. a
  future SHA-256 variant requirement, or a maintenance event).
- **Replace** — which authorizes only a subsequent, separately reviewed implementation RFC. That RFC must
  carry an identity-equivalence evidence requirement: proof that every existing ObjectId is unchanged.

**Retention is an equally acceptable outcome provided it is reasoned rather than default.** The failure
mode here is not picking the "wrong" answer; it is producing a record that reaches a conclusion the
evidence does not support, in either direction.

## Traps

- Do not implement either outcome under DC-50.
- Do not re-run or re-litigate DC-41's evidence — it is accepted.
- Do not treat "10,000 cases passed" as proof of correctness, and do not treat "hand-written crypto" as
  automatically disqualifying. Both are the shortcut version of this analysis.

## Definition of done

A decision record stating: the decision; the evidence it rests on; the conditions that would reopen it;
and, if replace, the scope boundary and identity-equivalence requirement of the follow-up RFC. No code,
dependency, or identity changes. Place it under `rfcs/handoffs/DC-50-first-party-sha256-roi-decision/`.

## Submit with

The decision record; a statement that no code changed; and — since this touches no code — only the
documentation gates (`mdbook build docs`, `git diff --check`, release-policy `reference-check`).
