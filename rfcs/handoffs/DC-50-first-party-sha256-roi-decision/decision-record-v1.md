# DC-50 First-Party SHA-256 ROI Decision - Decision Record v1

**Date:** 2026-07-28
**Produced under:** `implementation-handoff-v1.md`, cleared to start after project-owner acceptance of
`rfcs/accepted/DC-50-FIRST-PARTY-SHA256-ROI-DECISION.md`, which incorporates both findings from
`.git-exclude/reviewed/prikk-dc50-author-reexamination-v1.md` (performance as a required sixth
question; the DC-51 allowlist collision).
**Scope discipline observed:** no code, dependency, manifest, or CI file changed to produce this
record. `git status --short -- crates/ Cargo.toml Cargo.lock release/ tools/` is empty. DC-41's
evidence is not re-run or re-litigated; it is taken as accepted input.

## Decision

**Replace.** `prikk-hash`'s first-party SHA-256 implementation should be retired in favor of the `sha2`
crate, via a subsequent, separately reviewed implementation RFC. This record does not perform that
replacement — see "Scope of the follow-up RFC" below for what it authorizes and what it does not.

## The six questions, answered on the evidence

### 1. Correctness assurance

DC-40, DC-41 stage 2, and DC-41 stage 3 together establish 10,022 agreeing comparisons across two
independent implementations (Python `hashlib`, RustCrypto `sha2`), spanning 957 distinct input lengths
including both padding-boundary transitions. This is strong evidence that the first-party
implementation is correct today.

It is not proof. The 10,000 stage-3 cases come from a single fixed seed — 10,000 specific inputs, not
10,000 independent samples of the input space. Taken at face value this argues mildly for *either*
outcome: it is strong enough that "we cannot trust the first-party code" is no longer a valid reason to
replace it, but it is not so strong that "correctness is fully settled" is a reason to stop asking the
question. This question is therefore close to neutral between retain and replace on its own — its real
effect, per the accepted RFC's note, is to remove one specific pro-replacement argument
(unverified correctness) rather than to supply a pro-replacement or pro-retention argument itself.

### 2. Maintenance cost

The first-party implementation is genuinely small (`crates/prikk-hash/src/lib.rs`, roughly 150 lines: a
straightforward `rotate_right`-based compression function, no unusual structure). Read once, it is not
expensive to review. But the compounding factor the handoff names is real and not hypothetical: because
SHA-256 output is woven into every ObjectId, state root, ref-name path, and signature preimage, *any*
future change to this code — even a refactor with no intended behavior change — is an identity-bearing
change requiring a fresh vector re-verification campaign, not an ordinary code review. The maintenance
cost is low in the common case (the code rarely needs to change) but structurally expensive in the
uncommon case (when it does). This weighs toward replace: `sha2` carries that same identity-bearing
review burden for its *own* maintainers, not for this project.

### 3. Replacement cost and risk

`sha2 0.10.9` is already resolved in this workspace's dependency graph as a normal (non-dev) dependency,
pulled by `ed25519-dalek` into `prikk-crypto`, which is itself a normal dependency of `prikk-store` and
`prikk` (verified via `cargo tree -i sha2`). Replacing `prikk-hash`'s implementation therefore deletes
code and changes call sites; it does not add a new crate to the graph.

The risk is real, not cosmetic: `prikk-hash` is consumed as a normal `[dependencies]` entry — not a
dev/test-only path — by `prikk-object` (`id.rs`, `payload/patch.rs`) and `prikk-store` (`layout.rs`,
`wal.rs`, `lifecycle_cache.rs`, `trust.rs`, `text_span.rs`, `refs/log.rs`, `state_root.rs`), nine
genuine sites confirmed by direct search and independently re-verified by review — this is production
code, not a test double. (An earlier draft of this citation list also named
`prikk-object/src/vectors.rs`; that module is `#[cfg(test)]`-gated and does not compile into
production. Removed per review N3.) Any
behavioral difference between the two implementations, however small, would alter every ObjectId,
state root, and signature preimage that exists. DC-41's evidence bounds this risk (10,022 agreeing
cases across the boundary conditions that matter for a Merkle–Damgård construction — length, padding,
block-boundary transitions) but does not eliminate it; only a dedicated identity-equivalence campaign
against the *actual replacing code path* closes that gap, which is why the follow-up RFC carries its own
evidence requirement rather than inheriting DC-41's.

### 4. Supply-chain trade

First-party code has no upstream compromise surface but also no external review beyond this project's
own. `sha2`/RustCrypto has broad external review and adoption but is an upstream dependency subject to a
future supply-chain event.

This trade is materially weaker than it looks in isolation, because it is **already made** in this
workspace: `sha2` is pulled in today by `ed25519-dalek` and used for SHA-512 inside Ed25519 signing —
the integrity mechanism that governs publication and trust. Using the *same, already-trusted* crate for
content hashing does not cross a new supply-chain boundary; it extends reliance on a boundary this
project already depends on for something more security-sensitive than content hashing. The marginal
new exposure from a replace decision is close to zero.

One distinct posture point, correctly separated out by the re-examination (N2): `Cargo.toml` sets
`unsafe_code = "forbid"` workspace-wide, and `sha2`'s accelerated backends use `unsafe` internally
(outside this workspace's own lint boundary, so not a violation, but a real posture change). This
argument is weakened by the same fact as above — `ed25519-dalek`'s own SHA-512 path already runs
through unsafe-using acceleration in this dependency graph, so "this workspace's SHA-256 hot path stays
entirely first-party and safe" is not a claim this project can make cleanly today regardless of what
DC-50 decides. Worth naming in this record so it isn't silently rediscovered later, but it does not
change the answer to this question.

### 5. Reversibility

The decision is reversible in both directions without a format break, provided the follow-up RFC's
identity-equivalence floor is met: if replacement produces byte-identical output to the first-party
implementation across every existing golden vector and a differential campaign at least as large as
DC-41 stage 3's, then no on-disk format or identity changes, and the swap is in principle reversible
later (in either direction) by the same evidence discipline. Reversibility is therefore close to
symmetric between the two outcomes — very slightly asymmetric toward retain, since reverting a replace
means resurrecting deleted code from history and re-running a fresh equivalence campaign, while
retaining keeps both options open at zero cost — but it is not the deciding factor here: that small
option-value difference is dwarfed by a ~5.8× standing performance tax (see question 6). It does rule
out one argument for retaining as the "safe default": retaining now does not foreclose replacing later,
and replacing now (done correctly) does not foreclose reverting later, at a small but real added cost.

### 6. Performance (added by the 2026-07-28 re-examination)

This is the question that moves the decision. `prikk-hash` is a naive scalar implementation with no
SIMD or hardware acceleration; `sha2` ships CPU-accelerated backends selected at runtime via
`cpufeatures`. The re-examination's isolated benchmark found a consistent **~5.8× throughput
difference** across 64 B, 4 KB, and 1 MB inputs — not a small-input artifact, and not a one-off
measurement to be taken on faith (the record follows the handoff's instruction not to inherit these
figures; they should be reconfirmed on release hardware before DC-42 relies on them quantitatively, but
the order of magnitude is not in question given the measurement method described in the re-examination).

This is not a peripheral concern. SHA-256 sits on the hot path for every ObjectId, every state root,
every ref-name path, and every signature preimage. State-root computation in particular hashes every
live node, so its cost scales directly with repository size — the performance gap widens in absolute
terms as repositories grow, which is exactly the regime DC-42's NFR-PERF-01 (commit must not be
dominated by avoidable work) is meant to guard against. A ~6× gap in the primitive underneath that
requirement is a direct, quantified input to it, not a footnote.

## Weighing the six

Two questions (correctness assurance, reversibility) are close to neutral — they remove objections to
either outcome rather than arguing for one. Two questions (maintenance cost, performance) argue for
replace on their own terms. One question (supply-chain trade) argues for replace once the existing
`ed25519-dalek` SHA-512 dependency is accounted for, rather than treated as if `sha2` were a wholly new
addition to the trust boundary. The remaining question (replacement cost and risk) is a real, non-zero
cost — but it is bounded and closeable by the identity-equivalence floor already specified in the
accepted RFC, not an open-ended risk.

Retaining the first-party implementation would require accepting an ongoing, measured ~5.8× performance
tax on every hash operation in the system, indefinitely, in exchange for a supply-chain benefit that is
largely already forfeited elsewhere in the same dependency graph. That is not a reasoned case for the
status quo; it is the default outcome DC-50 was written specifically to avoid reaching without
justification.

## Conditions that would reopen this decision

Not applicable in the form the handoff specifies for retain (a revisit trigger) — this record concludes
replace. The condition that would reverse *this* decision, prior to the follow-up RFC's implementation,
is: the follow-up implementation RFC's identity-equivalence campaign fails to produce byte-identical
output against the accepted golden vectors, or a differential run at least as large as DC-41 stage 3's
surfaces even one mismatch. In that event the correct response is to retain the first-party
implementation and record why, not to weaken the equivalence requirement to force a pass.

## Scope of the follow-up RFC

This decision **authorizes** a subsequent, separately reviewed implementation RFC. It does **not**
authorize implementation under DC-50, and no code, dependency, or manifest changed to produce this
record. The follow-up RFC must carry, at minimum:

1. **Identity-equivalence evidence requirement** (per the accepted RFC's floor): every committed FDD
   golden vector and canonical-encoding snapshot must produce byte-identical output through the
   replacing code path, plus a differential run at least as large as DC-41 stage 3's 10,000 cases
   against the implementation being replaced.
2. **A reviewed `ALLOWED_THIRD_PARTY` amendment** to `tools/release-policy/src/boundary/placement.rs`,
   since DC-51's gate currently grants `prikk-hash` zero third-party dependencies
   (`("prikk-hash", &[])`). Without this amendment, `sha2` in `prikk-hash`'s `[dependencies]` fails
   `boundary-check` closed. This is a release-policy control-surface change under the DC-45 precedent,
   not a routine dependency addition.
3. **Confirmation of the performance figures on release hardware**, since this record's performance
   argument is load-bearing for the decision and the re-examination's numbers were measured in an
   isolated scratch crate, not the target release environment.
4. **No change to `prikk-hash`'s public API surface** beyond what the swap requires — this decision is
   about the implementation behind the type, not the crate's shape.
5. **Explicit disposition of `crates/prikk-hash/src/tests/hash_differential.rs`** (added per review N1).
   That test currently asserts the first-party implementation against a `sha2`-backed reference. Once
   `prikk-hash` itself becomes a wrapper over `sha2`, the comparison becomes `sha2` against `sha2` —
   trivially true and a loss of the implementation-diversity check DC-41 stage 2 was written to provide.
   The follow-up RFC must either delete the differential as honestly vacuous or re-point it at a
   genuinely independent third reference implementation; it must not remain in place implying coverage
   it no longer has. (Stage 2's fixed vectors are unaffected — four are canonical published NIST/RFC
   values and the rest are independently computed, so they stay meaningful regardless of what implements
   the hash.)
6. **Equivalence campaign coverage of the accelerated backend** (added per review N2). `sha2` selects its
   backend at runtime via `cpufeatures`; an equivalence run on hardware without SHA-NI would prove
   equivalence only for the scalar fallback while release binaries on capable hardware take the
   accelerated path. The campaign must record which backend it exercised, and either run on
   accelerated-capable hardware or force and compare both paths. This pairs with item 3 — same hardware
   question, two purposes.

## What this record does not do

- Does not implement replacement.
- Does not change `prikk-hash`, any call site, any manifest, or `Cargo.lock`.
- Does not re-run or re-litigate DC-41's evidence.
- Does not select an RFC number or schedule position for the follow-up — that is
  `rfcs/EXECUTION-ORDER.md`'s and the project owner's call, informed by this record's note that the
  performance finding bears directly on DC-42.
