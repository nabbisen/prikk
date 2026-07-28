# RFC (proposed) - DC-55 First-Party SHA-256 Replacement

**Status.** Proposed. Requires owner acceptance before implementation may begin.

**Independence of review — recorded deliberately, not silently.** This RFC was authored by the architect
and reviewed by the architect. Design review v1
(`.git-exclude/reviewed/prikk-dc55-design-review-v1.md`) was an **author re-examination, not an
independent review**; this project has one architect, so independent review is not an achievable state for
a design the architect wrote. That review returned a blocking finding (B1) and five notes, all resolved in
this revision by the same author. The prior calibration reserved identity-bearing increments for something
stronger than author re-examination, and DC-55 is identity-bearing; the project owner directed on
2026-07-28 that revision proceed on this basis regardless. It is named here so the gap is on record rather
than absorbed by routing convention.

The gap is deliberately compensated at the implementation axis, where independence *is* achievable — the
architect does not write the implementation. Acceptance criteria below were reworked so that the identity
claim is reproducible by a reviewer from the repository alone rather than trusted from the implementer's
report. See criterion 5 in particular.

**Target milestone.** M2 - post-correction assurance milestone.
**Schedule position.** Recommended **ahead of DC-42**. DC-42 owns NFR-PERF-01; DC-50 measured a ~5.8x
throughput gap on the primitive underneath it. Running DC-42 first would set performance requirements
against a baseline this increment is already authorized to invalidate.
**Tracks.** The **replace** decision recorded by DC-50 and closed at `4005efb`. This RFC is the
"subsequent, separately reviewed implementation RFC" that decision authorizes, and nothing more.
**Touches.** `crates/prikk-hash` (implementation, manifest, crate documentation), the root workspace
manifest, `tools/release-policy/src/boundary/placement.rs`, the DC-41 stage-3 differential test, the
assertion message in `crates/prikk-object/src/vectors/snapshot.rs`, and a test-only end-to-end check under
`crates/prikk-cli/tests`.

## Problem

DC-50 concluded that prikk should stop maintaining a first-party SHA-256 implementation and route
`prikk-hash::sha256` to `sha2` instead. The reasoning is closed and is not reopened here: `sha2 0.10.9` is
already a production dependency through `ed25519-dalek`, so a compromise of it already reaches Ed25519
signature verification; refusing it for content hashing while depending on it for signing is not a
coherent posture, and the ~5.8x throughput cost of the refusal is permanent and on the hot path.

What DC-50 deliberately did **not** do is perform the swap. This is an **identity-bearing** change: every
ObjectId, state root, ref-name path, and signature preimage in existence derives from this function. A
behavioural difference of one bit rewrites the identity of every object prikk has ever produced. DC-41's
10,022 agreeing comparisons bound that risk; they do not eliminate it, and they were collected against the
implementation being removed rather than the one replacing it.

The whole of this RFC is therefore: make the swap, and prove it changed nothing.

## Design

Six normative requirements. Items 1-4 come from DC-50's decision record; items 5 and 6 were added by the
architect review of that record (N1, N2).

Item 1 is subdivided into 1a (the equivalence campaign), 1b (committed artifacts), and 1c (end-to-end
check) following design review v1's blocking finding, which was that the original draft specified 1b only
and left the identity claim resting on evidence that could not establish it.

### 1. Identity-equivalence floor

The floor has two parts, and the first carries the weight. Design review v1's blocking finding was that
the original draft specified only the second.

#### 1a. The equivalence campaign: outgoing versus incoming

**The outgoing first-party implementation must be compared directly against the incoming `sha2`-backed one**,
over at least the 10,000 randomized cases DC-41 stage 3 used plus all 11 fixed vectors. This is the only
run that demonstrates the swap preserved identity, and it is what DC-50's floor means by "a differential
run… against the replacing implementation."

Nothing else substitutes. Comparing the new implementation against `sha2` is a self-comparison and proves
nothing. Comparing it against a third reference proves the new code is *correct*, which is a different and
weaker claim than *identical to what it replaced*.

Both implementations must therefore exist simultaneously during the campaign. Retain the outgoing
implementation as a `#[cfg(test)]`-only frozen module for the duration, run the campaign, and record the
seed and results in the evidence note. Item 5 then decides whether that module stays.

A mismatch at any case is a **stop-work finding** under DC-41's escalation clause: do not patch either
side, narrow the distribution, or adjust the seed.

#### 1b. Committed artifacts that must be byte-identical

| Artifact | Content | Regenerable |
|---|---|---|
| `crates/prikk-object/src/vectors/hard.rs` | Hard FDD vectors, incl. `empty_patch_anchor_matches_fdd_golden_vector` and `codec_sample_object_id_is_stable` | **No** — never regenerated by design |
| `crates/prikk-object/src/vectors/snapshot.txt` | 17 generated identity rows (`name\|type\|schema\|payload_hex\|object_id_hex`) | Yes, via `PRIKK_REGEN=1` — **forbidden for this increment**, see §Risks |
| `crates/prikk-store/src/state_root/tests/vectors.rs` | DC-40 state-root vectors over the v2 leaf/node/root domains | No |
| `crates/prikk-store/src/text_span/vectors.rs` | Text-span digest vectors (58 committed literals) | No |
| `crates/prikk-hash/src/tests.rs` | 11 fixed vectors — 4 canonical published (FIPS 180-2 / RFC 6234), 7 independently computed | No |
| `tests/fixtures/object-id-vectors.md` | Documented ObjectId formula and vector `5f8711b3…` | No |

**This table is not exhaustive coverage of the hashing sites, and must not be read as such.** Three
persisted, identity-bearing digests have no committed expectation anywhere:

| Site | What it computes | Test shape |
|---|---|---|
| `crates/prikk-store/src/layout.rs:379` `ref_name_storage_key` | `to_hex(sha256(ref_name))` — the **on-disk filename** for `<key>.ref`, `.log`, `.lock`, `.tmp` (lines 263-287) | none |
| `crates/prikk-store/src/wal.rs:372` `record_checksum` | WAL record checksum, persisted | writes at `:274`, verifies at `:313` — **same function on both sides** |
| `crates/prikk-store/src/refs/log.rs:253` `log_record_checksum` | Ref-log record checksum, persisted | writes at `:163`, verifies at `:203` — **same function on both sides** |

These are **covered by the primitive**, not by vectors: all three are pure functions of
`prikk_hash::sha256`, and this RFC forbids touching call sites, so 1a's campaign establishes them
transitively. They are listed because their round-trip test shape means a changed digest would pass the
entire suite while making every existing repository's refs unfindable at their old paths and every
existing WAL and ref-log record fail checksum validation. That is the blast radius if 1a is skipped, and
the reason 1a is not optional.

Note on the last artifact row: `tests/fixtures/object-id-vectors.md` is not consumed by any test — it is
documentation that happens to agree with `snapshot.txt`'s `patch_payload` row. Verify it by hand and say so.
Mechanically enforcing it is out of scope here.

#### 1c. End-to-end repository check

Create a repository under the outgoing code, then run `verify` under the incoming code and confirm it
passes clean. `crates/prikk-store/src/verify.rs` and `doctor.rs` provide the capability and
`crates/prikk-cli/tests` can drive it.

This is the only check that exercises all 11 call sites against genuinely persisted bytes, and the only
one that would catch a `layout.rs:379` storage-key change directly rather than by inference from 1a.

### 2. Allowlist amendment, in the same commit as the manifest change

`tools/release-policy/src/boundary/placement.rs:7` currently reads `("prikk-hash", &[])` — zero permitted
third-party dependencies. `sha2` today sits in `prikk-hash`'s `[dev-dependencies]`, which
`dependency_entries` deliberately excludes, so it passes. Moving it to `[dependencies]` fails
`boundary-check` closed until the entry becomes `("prikk-hash", &["sha2"])`.

Both edits land in **one commit**. Splitting them leaves an intermediate state where the gate fails, which
is a broken bisect point on an identity-bearing change.

This is a release-policy control-surface change under the DC-45 precedent: `placement.rs` is a
review-gated policy artifact, not refactorable code.

### 3. Performance confirmation on release hardware

DC-50's figures (64 B: 220 vs 1265 MB/s; 4 KB: 463 vs 2693; 1 MB: 470 vs 2732) were measured in isolation
on the author's machine. Re-measure and record. The purpose is not to re-litigate the decision — it is
closed — but to hand DC-42 a baseline it can build NFR-PERF-01 on without re-deriving it.

### 4. No public API change

`prikk-hash` exposes exactly three items: `Sha256Digest` (`lib.rs:11`), `sha256` (`lib.rs:30`), and
`to_hex` (`lib.rs:130`). All three keep their present signatures and semantics. `to_hex` is not a hash
function and stays first-party.

The change is confined to the body of `sha256` and the constants and helpers it alone uses (`H0`, `K`, the
compression routine). `sha256` has **11 production call sites across 7 files** (`prikk-object`: `id.rs:122`,
`payload/patch.rs:17`; `prikk-store`: `wal.rs:372`, `layout.rs:379`, `state_root.rs:66,78,93,108`,
`refs/log.rs:253`, `text_span.rs:150,168`), plus one `#[cfg(test)]`-gated site at
`prikk-store/src/lifecycle_cache.rs:80`. None may need editing; if one does, that is a signal the API
changed and the increment is out of scope.

`to_hex`'s consumers — `prikk-store/src/trust.rs`, `prikk-object/src/vectors.rs` (test-gated), plus
`layout.rs` and `id.rs` — are unaffected, since `to_hex` is not being replaced.

`crates/prikk-hash/src/lib.rs:6-8` currently documents the implementation as deliberately first-party
"for the initial source drop." That paragraph becomes false and must be rewritten to state what the crate
now is and which decision made it so.

### 5. Explicit disposition of the DC-41 stage-3 differential (per review N1)

`crates/prikk-hash/src/tests/hash_differential.rs:135` asserts `sha256(&input) == reference_sha256(&input)`
where `reference_sha256` wraps `sha2::Sha256`. After the swap that compares **`sha2` against `sha2`** —
trivially true, providing zero assurance, and precisely the tautology DC-41 stage 2 was written to forbid.
Leaving it in place while its name still implies coverage is not acceptable.

This is a question about **standing regression coverage in the tree afterward**, and is separate from
item 1a's campaign. 1a proves the swap preserved identity as a one-time event; item 5 decides what keeps
watching afterward. Resolving 1a means this choice no longer carries the equivalence claim, which lowers
its stakes considerably from the original draft.

Exactly one of four:

- **Delete it**, recording honestly that the swap retires 10,000 of the 10,022 comparisons that justified
  it, and that the surviving evidence is stage 2's 11 fixed vectors — which stay meaningful because four
  are canonical published values and the rest were computed independently of any Rust implementation.
- **Keep the frozen outgoing implementation** from item 1a as the differential's permanent reference. It is
  genuinely independent of `sha2`, already reviewed under DC-41, immutable because it is test-only and will
  never be maintained again, and it adds **zero dependencies**. Its cost is that first-party SHA-256 code
  remains in the tree as test scaffolding, in tension with DC-50's "stop maintaining a first-party
  implementation" — though "maintain" is doing real work in that sentence, since a frozen test-only module
  is not maintained in any meaningful sense.
- **Expand the out-of-band fixed vectors** the way DC-41 stage 2 did, with values computed outside Rust
  entirely. Also zero dependencies. Weaker than a differential — fixed vectors cover chosen inputs, not a
  distribution — but it needs no code retained at all.
- **Re-point at a new third-party reference** (`ring`, `openssl`, or similar). Note that
  `[dev-dependencies]` is outside DC-51's gate, so this adds a dependency the placement check will not
  question — which makes it a decision for design review rather than one the gate will catch. This option
  is dominated by the two above on the supply-chain axis and should be chosen only for a stated reason.

Any of the four is acceptable; silence is not.

### 6. Equivalence campaign must record its backend (per review N2)

`sha2 0.10.9` selects its backend at runtime through `cpufeatures 0.2.17`. A campaign run on a host without
SHA-NI proves equivalence for the **scalar fallback only**, while release binaries on capable hardware take
the accelerated path. The two must agree — SHA-256 is deterministic and divergence would be a `sha2` defect
— but "must agree" is exactly the assumption an equivalence campaign exists to test rather than inherit,
with every ObjectId in existence as the stake.

**State this as a reproducible build instruction, not a hardware assertion.** "I ran it on a machine with
SHA-NI" is a claim about someone else's hardware that no reviewer can check afterward. `sha2 0.10` exposes
a `force-soft` feature (confirmed in its manifest, alongside `asm` and `force-soft-compact`), so the
requirement becomes:

- Run item 1a's campaign and the fixed vectors **twice** — once with default features, once with
  `sha2/force-soft` — and require both to match the same committed vectors.
- Record the runtime probe (`is_x86_feature_detected!("sha")` or the target's equivalent) as context for
  which backend the default run actually selected.

That covers both paths by construction rather than by hardware luck, and any reviewer can reproduce it on
their own machine. It pairs with item 3: same run, two purposes.

Run this locally and record the output in the evidence note. Adding the second run to CI would mean a new
`run:` command in `.github/workflows/ci.yml`, which is a governed procedure file requiring a reviewed
classifier amendment in the same increment — unnecessary scope for a one-time campaign.

## Non-goals

- No change to object identity, canonical encoding, the state-root grammar, or any persisted byte. That is
  the entire point; a change here is a failure, not a scope extension.
- No re-litigation of DC-50's decision. It is closed at `4005efb`.
- No streaming or incremental hashing API, no SHA-512 or other variants, no `no_std` work.
- No performance tuning of call sites. Measuring the new baseline is item 3; acting on it is DC-42's.
- No mechanical enforcement of `tests/fixtures/object-id-vectors.md`.

## Risks

**The `PRIKK_REGEN=1` escape hatch is the primary hazard of this increment.** If the swap alters any
digest, `generated_snapshot_matches_committed` fails — and `snapshot.rs:3-5` documents regeneration as the
remedy, because for every previous increment it was. Here it is the opposite: regenerating would rewrite
all 17 identity rows, turn the test green, and destroy the evidence the increment exists to produce. A
snapshot diff in DC-55 is a **stop condition**, not a regeneration trigger.

The prose above is not where an implementer will be standing when this matters. The assertion message at
`snapshot.rs:26-28` repeats the regenerate-and-review instruction *at the moment of failure*, so the
tooling contradicts the handoff exactly when the contradiction is decisive. Amend that message during
DC-55 to carve out identity-bearing increments: regeneration is correct for an intended encoding change
and a stop-work condition when the increment claims identity preservation. This is a small edit to a
test-support file, not a control-surface change.

**`#![forbid(unsafe_code)]` becomes a weaker claim than it reads.** The attribute stays literally true of
`prikk-hash`, but the hashing then happens inside a dependency whose accelerated backends use `unsafe` and
CPU feature detection. Nothing needs fixing; the crate documentation should not imply a safety property the
crate no longer delivers on its own.

**Lockfile expectation.** `sha2` is already in the graph and already declared in `[workspace.dependencies]`
(root `Cargo.toml:48`, under a `# Testing` heading that stops being accurate). Moving it between tables
should leave the locked package count at 180. Report the count; a change means something unexpected
resolved and needs explaining before the increment proceeds.

## Acceptance criteria

Each criterion notes whether a reviewer can verify it after the fact from the repository alone, or must
trust the implementer's report. That distinction is what buys back the design-axis independence gap named
in the Status field, so it is part of the criteria rather than a footnote.

1. `prikk-hash::sha256` is implemented over `sha2`; the first-party compression code is gone (or retained
   test-only per item 5); the three public items are unchanged; the crate documentation reflects reality.
   *Verifiable from the diff.*
2. **Item 1a's outgoing-vs-incoming campaign passes** over ≥10,000 randomized cases plus all 11 fixed
   vectors, with seed and results recorded. *Verifiable — a reviewer can re-run it from the recorded seed.*
3. Every artifact in item 1b's table is byte-identical, verified without regeneration, and reported
   individually rather than as an aggregate pass. *Verifiable — check out the parent, record digests, check
   out the child, recompute, diff; a regenerated `snapshot.txt` would also appear in the commit diff.*
4. Item 1c's end-to-end check passes: a repository created under the outgoing code verifies clean under
   the incoming code. *Verifiable — reproducible from the recorded procedure.*
5. Both backend runs pass — default features and `sha2/force-soft` — against the same committed vectors,
   with the runtime probe recorded. *Verifiable — it is a build flag, not a hardware claim. This criterion
   was rewritten from "the accelerated path was covered" precisely because that form was not checkable.*
6. `("prikk-hash", &["sha2"])` and the manifest move land in the same commit; `boundary-check` passes.
   *Verifiable from git history.*
7. The differential's disposition is implemented and its rationale recorded. *Partly verifiable — that it
   was done is checkable; the rationale's quality is judgment.*
8. Fresh performance figures are recorded for DC-42's use. *Not independently verifiable — hardware-
   dependent, and a reviewer's numbers will differ legitimately. Accepted, because this is not
   identity-bearing and DC-42 will re-measure on its own terms.*
9. Full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9, plus test counts before and after per rule 10.
   *Verifiable — re-runnable.*
