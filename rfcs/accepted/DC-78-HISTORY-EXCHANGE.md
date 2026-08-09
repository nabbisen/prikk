# RFC (accepted) - DC-78 History Exchange

**Status.** **ACCEPTED by the project owner 2026-08-09**, same day as proposal.

**Cleared for §4's investigation only.** Design may not begin until §4 is answered and reported —
including §3.2's dependency question, which may itself change when this increment can proceed.
§2's recommendation (scope to exchange, defer transport) is accepted; **§3.1 is a position to test, not
a ruling**, and the RFC says so.
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
2. **Against whose keys does the receiver verify?** The decisive question — **see §3.1, which states a
   starting position rather than leaving this blank.** Trust today is a **local** store (`trust/`) naming
   maintainers permitted to seal *here*. When B receives history sealed by A's maintainer, B must decide
   whether that authority means anything locally. **Exchange forces the trust store to become
   distributed, and that is an identity problem, not a networking one.**
3. **What does a receiver do with refs?** Publication is compare-and-swap against local state. Two
   repositories advancing the same ref independently is the divergence merge handles *within* a
   repository; across repositories there is no equivalent concept yet.
4. **Must a bundle be verifiable standalone, or only against a repository that already shares history?**
   NFR-PERF-04's spirit forbids a bundle that is a *summary* — it must be a verifiable subset, never a
   new root of trust.
5. **What does partial history mean?** `verify` re-derives state roots by walking lineage to genesis. A
   receiver holding a suffix cannot do that. Either exchange is always genesis-complete, or lineage
   horizons acquire a meaning they do not currently have.

### 3.1 A starting position on question 2 — to argue against, not to inherit

Added 2026-08-09 at the owner's request, after they named the real tension: **security and cleanliness
against usefulness and intuitiveness.** That tension is genuine. This is offered so the investigation
starts from a position to attack rather than a blank page; **it is not a ruling, and §4 still requires
question 2 answered from the code and the requirements.**

**Most of the sharpness comes from conflating two things that need not be conflated: *having* history and
*trusting* it.**

- **Reception needs no trust.** Objects are content-addressed, so nothing can be forged into an existing
  object id and receiving bytes cannot corrupt what the receiver already holds.
- **Authority is the only thing needing a decision** — whether the sealer's maintainer key means anything
  locally.

**So the trust question is asked once per key, at the moment received history is made authoritative for a
local ref — not per object, and not at reception.** Fetching is cheap and safe; adopting is the act that
requires a decision. That separation is also what users already expect, which is why the ergonomic cost
is far lower than "verify everything against keys you must obtain first" suggests.

**First contact is then the only hard case**, and the proposal is **trust on first use, recorded and
thereafter enforced**: the store notes that key X was accepted at block Y, and every later exchange is
checked against that record. Strictly weaker than out-of-band key agreement, and strictly stronger than
silent TOFU — a later key substitution stops validating and is detectable. It is also honest about what
it is, which matters more here than claiming a strength the model does not have.

**The line that must not be crossed.** Received history must **never** be trusted by default for
convenience. This is the same defect shape as the system proposal's RΔ5 Git-import delta, ruled on
2026-08-02: imported history must be **distinguishable at the object level, permanently and
non-strippably**, or the central claim becomes false for any repository that ever used the bridge.
**Received-but-unadopted history has exactly that property.** If it is indistinguishable from locally
verified history, the claim dies quietly the first time anyone pulls.

### 3.1a §3.1 tested — two clauses were wrong. Corrected 2026-08-09.

§4's investigation tested the proposition rather than inheriting it, as instructed, and found this:

- **Clause 1 holds.** Reception needs no trust; `ObjectId` derives from
  domain ‖ type ‖ schema ‖ len ‖ payload (`id.rs:114-122`) with signatures excluded, so storing received
  bytes is safe independent of any trust decision.
- **Clause 2 was wrong.** *"Authority is the only thing needing a decision"* presumes a verification
  mechanism exists per signer role. **`trust.rs:215` is the sole production `verify_ed25519` call site
  and it hardcodes `SignerRole::Maintainer`.** Author signatures are **never** cryptographically
  verified — for received *or* purely local history. Adopting a remote author's key would have nothing
  to verify against.
- **Clause 3 overclaimed by tense.** TOFU does not exist: DC-11 declined to build it, and
  `security-setup.md:67` and `trust-threat-model.md:61` both state plainly that there is no
  trust-on-first-use rule. **This increment builds it from nothing.**

**The RΔ5 line is unaffected and is now more load-bearing, not less:** because authorship is exactly what
nothing checks, permanent non-strippable provenance marking does real work.

### 3.1b Ruled 2026-08-09 — the trust claim this increment may make

**"This history was sealed by a Maintainer key you adopted."** Buildable today with zero new
cryptographic capability — `add_trusted_maintainer` already is ask-once-per-key adoption, never yet
pointed at a remote peer's key.

**Not: "the received patches' authorship is verified."** That code exists nowhere, and building it here
would absorb DC-53. **State the claim explicitly in design and in user-facing docs; never imply the
stronger one.**

**Also ruled:** exchange is **genesis-complete for v1** — the lineage walk reaches a literal Root
unconditionally and no partial-history concept exists anywhere, including `specs/`. Recorded as a stated
limitation. And **TOFU is new construction**, so it is the part designed first and reviewed hardest.

### 3.2 A sequencing consequence, corrected

An earlier architect framing said a receiver could "verify structurally on receipt." **That is weaker
today than it sounds:** `verify` performs no cryptographic verification of author signatures at all — the
product's only crypto verification call site is a policy signature (`crates/prikk-store/src/trust.rs:215`).

**Superseded 2026-08-09 by §4's investigation: DC-53 is *conditionally* a prerequisite, and under
§3.1b's ruling it is not one.** A receiver verifies exactly as much as a local user does today, so
criterion 1 is reachable without criterion 5. The original claim below stands only for the stronger
trust claim that §3.1b rules out: A receiver cannot meaningfully check what it was sent while
nothing checks author signatures. This should be settled before this increment is sequenced, not
discovered during it.

## 4. Blocking prerequisites

- §3's five questions answered **from the code and the requirements**, reported before any design, in
  the pattern that has widened the recorded scope in five consecutive increments.
- **Question 2 answered first**, with §3.1 treated as a position to test rather than adopt. If
  distributed trust has no acceptable answer, the rest is wasted work.
- **§3.2's dependency confirmed or refuted:** is DC-53 genuinely a prerequisite? If so this increment
  cannot start before it, and the 0.20.0 sequence changes.
- An explicit statement of whether **criterion 1 of the status-claim criteria** is satisfied by exchange
  alone, or requires transport too. **That determines whether this increment moves the badge.**

## 5. Non-goals

- **Network transport of any kind**, unless §2's recommendation is rejected.
- Any new dependency, and any change to `ALLOWED_THIRD_PARTY`.
- Conflict resolution, patch aggregation, remote-tracking refs — each separate, and none required to
  move history between two machines.
