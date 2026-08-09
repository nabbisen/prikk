# DC-86 Bundle Decoder Hardening — Handoff v1

**Cleared to start.** Accepted 2026-08-09, `rfcs/accepted/DC-86-BUNDLE-DECODER-HARDENING.md`.
**Test and hardening only — no format change.**

## 1. Why this surface and not another

You built a parser that consumes bytes from a party the operator does not control. **That is the only
one in the product.** Object decoding reads bytes the repository already holds; bundle decoding does not.

`EXECUTION-ORDER.md` §6 rule 3 says randomized decoder input is where something will plausibly be found,
and DC-41 stage 4 acted on that for the object decoders. **Yours has one malformed-input test.** That is
not a criticism of Stage 3 — it was out of its scope — but the gap should not outlive the increment that
created it.

## 2. Two jobs

**Property/fuzz coverage** over `decode_bundle` and the received-pointer decoder, in DC-41 stage 4's
shape. **If you find a panic, that is an NFR-SEC-04 defect** — per rule 3 it opens its own corrective RFC
with a minimized reproducer. **Do not encode a panic as an expected outcome.**

**A resource bound on import**: maximum object count and maximum total decoded bytes, refused **before
anything is written**. DC-57's shape — a hard block ahead of any write, default documented, config
failing closed on bad input. Reuse that pattern rather than inventing a second one.

## 3. What I will check hardest

**Criterion 3, the negative control:** a bundle just **over** the limit is refused and one just **under**
is accepted. A bound nobody has seen fire is a bound nobody knows exists — the same reasoning behind
every negative control this project has asked for.

**Criterion 2 measured, not asserted:** show the object count before and after a refused import is
identical. "It returned an error" does not prove nothing was written.

## 4. State the ceiling honestly

**Hardening a parser is not proving it correct.** Say what the campaign covered — which inputs, which
generators, how long — so the next person knows what the evidence does and does not span. A fuzz run
reported without its scope is indistinguishable from a thorough one.

## 5. Limits

No bundle format change. No revocation, no received-ref audit trail, no merge-from-received — each is
recorded in `FINDINGS.md` and each is its own question. **Green macOS run before merge**, per the
standing rule: this touches filesystem-backed import.
