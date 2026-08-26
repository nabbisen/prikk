# Non-Goals

This page collects, in one place, every decision this project has made to **permanently refuse**
building something — as distinct from work that is merely **unscheduled**.

**Refused** means a decision exists, and the answer is *no, not ever*: automation is not permitted
to do it, the design deliberately does not create the thing that would be needed, or the project
has scoped its supported surface to exclude it. **Deferred** means the opposite: the thing may be
built, nobody has scheduled it, and it lives in [`ROADMAP.md`](https://github.com/nabbisen/prikk/blob/main/ROADMAP.md)
instead — the project's planning authority for everything that is open, not refused.

**This page does not restate its sources.** Each entry below is a pointer to the page that carries
the authoritative decision, plus the citation behind it. If you cannot find a decision behind a
claim of "refused" anywhere in this project, it is not a non-goal — it is an opinion, and it does
not belong here or anywhere else as though it were settled.

## Automatic conflict resolution

Automation may not author a conflict resolution on a person's behalf. A resolution is itself a
signed patch, and signing on someone else's behalf is exactly what this project's signing model
exists to prevent — not a feature gap that a future increment closes.

**Decision:** DC-35 (signing authority), applied at the patch layer by DC-74 (merge execution).
**Full statement:** [Conflict Resolution Is Refused By
Design](patch-algebra.md#conflict-resolution-is-refused-by-design).

## Repository identity

There is no repository identifier, no peer identity, and no origin field anywhere in a persisted
object — not because nobody built one yet, but because the design never creates anything for a
future increment to add trust *around*. Identity lives only in signer keys and patch ids.

**Decision:** the repository-identity settlement (RFC 115's own investigation). The object code
`0x0A`, formerly `ProjectGenesis`, is permanently retired and must never be reassigned.
**Full statement:** [Trust Roots and Roles](trust-threat-model.md#trust-roots-and-roles).

## The library crates are not a supported dependency surface

The seven library crates behind the `prikk` CLI (`prikk-store`, `prikk-object`, `prikk-crypto`,
`prikk-hash`, `prikk-error`, `prikk-ffi`, `prikk-replay`) are implementation detail, not a product
of their own — the `prikk` CLI is the one supported surface. This is a scope decision, stated
independently of the API-stability disclaimer each crate's own README also carries for the period
before `prikk` reaches 1.0.

**Decision:** the published-crate-posture review (2026-08-26).
**Full statement:** each crate's own README, e.g.
[`prikk-store`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/README.md).

## What is not on this page

**Networked transport was considered and left off.** `ROADMAP.md`'s own Sync theme currently
reads this as settled and by design, but RFC 116 — the ruling `ROADMAP.md` cites — states it in
deferral terms of its own ("deferred," "in this increment," "if a protocol is later wanted"), not
as a permanent refusal. Restating `ROADMAP.md`'s stronger framing here, without the owner
confirming it narrows RFC 116's own words rather than merely summarizing them, would be exactly
the kind of restatement this page exists to avoid. Not included until that is confirmed.

**A handful of other candidates were checked and did not hold up as citable refusals**: semantic
merge (`ROADMAP.md` cites DC-16, but DC-16's own non-goals section scopes what *that RFC* builds,
the same way every RFC scopes itself — it does not rule that semantic merge will never be built);
DC-57's merge-scope bounding (the same self-scoping pattern); the Windows durability gap with no
`openat` equivalent (a platform constraint, not a project decision); and symlink/rename authoring
(named as a gap in [Data Model Relationships and
Lifecycle](data-model-lifecycle.md), but the one place that discusses its future,
`ROADMAP.md`'s editor-integration theme, calls it deferred, not refused). None of these carry a
decision stating the answer is permanently no.

**One existing page names something it should not.** [Trust and Threat
Model](trust-threat-model.md#threat-boundaries)'s own "Current non-goals" list includes items its
own earlier text calls "capabilities not yet built" — deferrals, not refusals, by that page's own
words two sections above the list. That page is not this page's to rewrite; noted here so the
inconsistency is visible rather than quietly carried forward.
