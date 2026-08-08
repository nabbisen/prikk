# RFC (proposed) - DC-78 History Exchange

**Status.** **Proposed 2026-08-09.** Awaits owner acceptance. **Not authorized for implementation.**
**Authored by** the architect. **Independence.** Author-reviewed — the standing ceiling.
**Arises from.** `MILESTONES.md`'s status-claim criteria, **criterion 1**: a distributed VCS that cannot
distribute. Priority ruled 2026-08-09 — this is the next design work after DC-76, may run alongside macOS
mutation, and **precedes Windows mutation**.

## 1. What is missing

**Nothing in the tree exchanges history between repositories.** Branches merge within one repository;
two machines cannot share anything. This is criterion 1 of six, and the only one with no increment behind
it.

Cross-platform mutation lets more people use prikk *alone*. **Only exchange lets two people work
together**, which is the premise the two-role author/maintainer model exists to serve.

## 2. The framing this RFC proposes — separate exchange from transport

The instinct is to call this "sync" and reach for a network protocol. **That bundles two problems with
very different shapes, and only one of them is hard for prikk.**

- **Exchange** — *what* moves between repositories, and what the receiver verifies before accepting it.
  Every genuinely difficult question lives here, and they are all identity and trust questions.
- **Transport** — *how* the bytes travel. A solved problem generally, and an expensive one here
  specifically: `prikk-store` may depend on exactly `getrandom` and `rustix`
  (`tools/release-policy/src/boundary/placement.rs`), so **any network transport is an
  `ALLOWED_THIRD_PARTY` decision** covering TLS, HTTP, or SSH.

**Exchange is achievable with no transport at all.** A verifiable subset written to a file and read back
elsewhere — any file-moving mechanism carries it, and **no new dependency is needed.** That also delivers
the system proposal's RΔ2 evidence bundle, which was reaching for the same artifact from the auditor
side.

**Recommendation, for the owner to accept or reject: scope this increment to exchange only.** Transport
becomes a later, separable decision made once the trust model is settled rather than alongside it.

## 3. The questions that actually decide this — none answered here

1. **What is the unit of exchange?** Objects are content-addressed, so a bundle could carry objects,
   blocks, or patches. Patch theory suggests patches; the sealing model suggests blocks; content
   addressing makes objects the simplest. **This is not a packaging choice** — it determines what a
   receiver can verify without already holding the rest of the history.
2. **Against whose keys does the receiver verify?** The decisive question. Trust today is a **local**
   store (`trust/`) naming maintainers permitted to seal *here*. When B receives history sealed by A's
   maintainer, B must decide whether that authority means anything locally. **Exchange forces the trust
   store to become distributed, and that is an identity problem, not a networking one.**
3. **What does a receiver do with refs?** Publication is compare-and-swap against local state. Two
   repositories advancing the same ref independently is the divergence merge handles *within* a
   repository; across repositories there is no equivalent concept yet.
4. **Must a bundle be verifiable standalone, or only against a repository that already shares history?**
   NFR-PERF-04's spirit forbids a bundle that is a *summary* — it must be a verifiable subset, never a
   new root of trust.
5. **What does partial history mean?** `verify` re-derives state roots by walking lineage to genesis. A
   receiver holding a suffix cannot do that. Either exchange is always genesis-complete, or lineage
   horizons acquire a meaning they do not currently have.

## 4. Blocking prerequisites

- §3's five questions answered **from the code and the requirements**, reported before any design, in
  the pattern that has widened the recorded scope in five consecutive increments.
- **Question 2 answered first.** If distributed trust has no acceptable answer, the rest is wasted work.
- An explicit statement of whether **criterion 1 of the status-claim criteria** is satisfied by exchange
  alone, or requires transport too. **That determines whether this increment moves the badge.**

## 5. Non-goals

- **Network transport of any kind**, unless §2's recommendation is rejected.
- Any new dependency, and any change to `ALLOWED_THIRD_PARTY`.
- Conflict resolution, patch aggregation, remote-tracking refs — each separate, and none required to
  move history between two machines.
