# RFC (proposed) - DC-50 First-Party SHA-256 ROI Decision

**Status.** Proposed; design review required. Recorded as an explicit deferred decision by DC-41's
non-goals so the question is scheduled rather than dropped.
**Target milestone.** M2 - post-correction assurance milestone.
**Schedule position.** Answerable only after DC-41 stage 3 is committed, because its differential evidence
is this decision's primary input. DC-41 stage 3 landed as `540d4db`, so the precondition is satisfied.
**Tracks.** Architect review N6 (the ROI half; the evidence half is closed by DC-41).
**Touches.** A decision record, and — only if the decision is to replace — a subsequent separate
implementation RFC. This RFC changes no code.

## Problem

`prikk-hash` contains a first-party SHA-256 implementation on which every ObjectId, state root, ref-name
path, and signature preimage depends. Architect review N6 raised two questions: whether its independent
evidence was adequate, and whether maintaining a custom cryptographic primitive has sufficient return.

DC-41 answered the first. `prikk-hash` now has cross-implementation agreement on 10,022 comparisons —
11 DC-40 state-root vectors and 11 stage-2 fixed vectors against Python `hashlib`, plus 10,000 stage-3
randomized cases against RustCrypto `sha2`, spanning 957 distinct input lengths. The second question is
now answerable on evidence rather than on speculation, and is the whole of DC-50.

A material fact for the decision: `sha2 0.10.9` is **already a production dependency** of this workspace,
pulled transitively by `ed25519-dalek` for Ed25519's internal SHA-512, and separately by
`tools/release-policy`. Replacing the first-party implementation would therefore remove code without
adding a dependency.

## Design

Produce one reviewed decision record that evaluates, on evidence:

1. **Correctness assurance.** What DC-41 established, and what it did not — 10,000 cases from a single
   fixed seed are 10,000 specific inputs, not a proof over the input space.
2. **Maintenance cost.** Ongoing review burden of a hand-written cryptographic primitive, including that
   every future change to it is an identity-bearing change requiring vector re-verification.
3. **Replacement cost and risk.** `sha2` is already in the graph, so replacement is a code deletion plus a
   call-site change — but it is an **identity-bearing** change: any behavioural difference would alter
   every ObjectId in existence. DC-41's evidence bounds that risk without eliminating it.
4. **Supply-chain trade.** First-party code has no upstream compromise surface but no external review;
   `sha2` has broad external review but is an upstream dependency. Note that `sha2` is already trusted in
   the runtime path today, so this trade is already partly made.
5. **Reversibility.** Whether the decision can be revisited later without a format break.

The decision must end in exactly one recorded state: **retain** (with the rationale and any conditions
that would reopen it), or **replace** (which authorizes only a subsequent, separately reviewed
implementation RFC — never direct implementation under DC-50).

## Non-goals

- No implementation of either outcome under this RFC.
- No change to object identity, canonical encoding, or any persisted byte.
- No re-litigation of DC-41's evidence, which is accepted.

## Acceptance criteria

The record states the decision, the evidence it rests on, the conditions that would reopen it, and — if
replace — the scope boundary of the follow-up implementation RFC including its identity-equivalence
evidence requirement. Retention is an equally acceptable outcome provided it is reasoned rather than
default.
