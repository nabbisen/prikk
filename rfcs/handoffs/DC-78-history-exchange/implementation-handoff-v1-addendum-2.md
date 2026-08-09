# DC-78 Handoff v1 — Addendum 2: Reading A. Design is mine.

**Date:** 2026-08-09. **Authored by** the architect.
**Responds to:** `.git-exclude/review-request/prikk-dc-78-design-ownership-question-v1.md`.

## 1. Reading A. You were right to ask, and your structural reasoning is the correct one

**Design is mine — external, internal, and program.** `EXECUTION-ORDER.md` §6 rule 1 draws exactly that
line, every RFC in the tree is architect-authored, and you noticed both. **Your reasoning was better
than my wording.**

**Addendum 1 §6 was ambiguous and that is my fault.** "Proceed. Design, under rulings 1–4" meant *the
increment proceeds to its design phase, which is mine*; it reads as an instruction to you. **Ruling 4
made it worse** — "design it first and expect it reviewed hardest" was written as if addressed to a
designer. What I meant: when **I** write the design, TOFU is the part I will write first, and the part
you should expect me to have scrutinised hardest when it reaches you for implementation.

**You are not idle:** DC-81 was mid-cycle and is now merged, and DC-82 is queued behind it.

The instinct that stopped you — *"the shape of mistake 'Wait. Design is not your task' already corrected
once this project"* — is exactly right, and asking cost one round trip against a real risk of overstep.

## 2. Your §2 fork is the central design question, and you found it while explicitly not designing

**Single adopted key or multiple?** That is the question at the heart of this increment, and framing it
as *"shouldn't default quietly either way"* is correct.

Restating what it actually decides, because it is bigger than a key count:

- **(a) Received history must be sealed under a Maintainer key the receiver has adopted** — one
  maintainer, many authors. Alice seals; Bob and others author and receive. `trust.rs` needs **no
  structural change**, only the ability to adopt a key that is not the local operator's. **This matches
  prikk's existing two-role model exactly.**
- **(b) Each remote gets its own adopted key** — peer-to-peer between independent maintainers. Closer to
  what "distributed VCS" usually implies, but it **reopens DC-11's deliberate one-key design**, which was
  a considered decision rather than an omission.

**So the fork is really: which collaboration model does prikk support?** That is product scope, and it is
the owner's, not mine. **It is with them now.**

**My recommendation, recorded so the owner has one:** (a) for v1. It needs no `trust.rs` redesign,
matches the role split already built, and **still satisfies status-claim criterion 1** — two machines
exchanging history that both verify is genuine distribution, whether or not both ends can seal. (b) then
becomes a later, separately-scoped question rather than a constraint reopened under time pressure.

## 3. What happens next

**I write the design.** You will get it as an RFC amendment or a design document, not as a handoff to
draft. **Nothing is owed by you on DC-78 right now.**

If the owner rules the fork before I finish, the design reflects it; if not, I will put it to them as the
first thing the design cannot proceed without.
