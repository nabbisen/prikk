# DC-78 Handoff v1 — Addendum 1: §4 accepted, four rulings, design cleared

**Date:** 2026-08-09. **Authored by** the architect. **Review:**
`.git-exclude/reviewed/DC-78-prerequisite-questions-review-v1.md`.

## 1. Accepted — and you found the clause of §3.1 that was wrong

Verified independently: signatures are excluded from the `ObjectId` preimage (`id.rs:114-122`);
**`trust.rs:215` is the sole production `verify_ed25519` call site and hardcodes
`SignerRole::Maintainer`**; both `security-setup.md:67` and `trust-threat-model.md:61` state plainly
there is no trust-on-first-use rule; `MissingBlockInLineage` is documented *"Never treated as genesis"*.

**Clause 2 of §3.1 was wrong and the error is mine.** "Authority is the only thing needing a decision"
presumed a verification mechanism per signer role. It exists for Maintainer and **does not exist at all
for Author** — not for received history, and not for local history either. **Clause 3 overclaimed by
tense**: TOFU is not a thing to extend, it is a thing this increment would build from nothing.

Testing the proposition instead of inheriting it is exactly what §3.1 asked for, and it worked.

## 2. Ruling 1 — the trust claim you may make

**"This history was sealed by a Maintainer key you adopted."** Buildable today with zero new
cryptographic capability. **DC-53 is therefore NOT a prerequisite.**

**Not: "the received patches' authorship is verified."** That code exists nowhere; building it here
would absorb DC-53, which is proposed, unaccepted and unscheduled.

**State the claim explicitly in the design and in any user-facing text. Never imply the stronger one.**
Your conditional answer was better than the yes/no I asked for.

**Roadmap consequence, and it is good news:** I had flagged that status-claim criteria 1 and 5 might
collapse into one chain. **They do not** — a receiver verifies exactly as much as a local user does
today, so criterion 1 is reachable without criterion 5. `MILESTONES.md` is corrected.

## 3. Ruling 2 — genesis-complete for v1

The lineage walk reaches a literal Root unconditionally; no alternate start exists; "horizon" already
means *the true genesis* and exists to assert a claimed one matches. **Exchange is genesis-complete, and
that limitation is stated rather than left implicit.** A shallow horizon would mean redesigning
`verify`'s core integrity walk — its own increment if ever wanted.

## 4. Ruling 3 — the exchange/transport split stands

Your §4 shows exchange alone composes into a real capability reusing `verify_repository` unchanged, with
only a new serialization boundary. Transport stays out of scope.

## 5. Ruling 4 — TOFU is new construction, and it is the security surface

Because clause 3 overclaimed, treat the TOFU record — its shape, where it lives, how a remote Maintainer
key is adopted and thereafter enforced — as **work this increment builds.** It is this increment's main
security surface. **Design it first and expect it reviewed hardest.**

And your point that the Author-verification gap makes RΔ5's provenance marking *more* load-bearing is
right: received history's authorship claims are exactly the ones nothing checks, so permanent
non-strippable provenance is doing real work.

## 6. Proceed

Design, under rulings 1–4. §5's stop-and-report conditions stand. **Sequencing is yours** — DC-81 is
mid-cycle awaiting a CI fix, and this is design work that cannot collide with it.
