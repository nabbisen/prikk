# DC-55 First-Party SHA-256 Replacement - Handoff

**Not yet cleared to start.** DC-55 is at `rfcs/proposed/DC-55-FIRST-PARTY-SHA256-REPLACEMENT.md` and must
move to `rfcs/accepted/` through its own design review first. This handoff removes every obstacle except
that gate; preparing it is not authorization.

**Authored by** the architect (function-designer role).
**Size:** small in diff, high in consequence. Roughly one function body deleted, one manifest line moved,
one allowlist entry, one test disposition.
**Touches:** `crates/prikk-hash/`, root `Cargo.toml`, `tools/release-policy/src/boundary/placement.rs`.

## What this is, in one sentence

Delete prikk's hand-written SHA-256, call `sha2` instead, and prove that not one byte of identity moved.

## Why the diff being small is the trap

Every ObjectId, state root, ref-name path, and signature preimage in this repository is a function of
`prikk_hash::sha256`. If the replacement disagrees with the original on any input, every object prikk has
ever written changes identity — silently, because nothing in the format records which implementation
produced it. There are **11 production call sites across 7 files**:

```
crates/prikk-object/src/id.rs:122             crates/prikk-store/src/wal.rs:372
crates/prikk-object/src/payload/patch.rs:17   crates/prikk-store/src/layout.rs:379
crates/prikk-store/src/state_root.rs:66,78,93,108
crates/prikk-store/src/refs/log.rs:253        crates/prikk-store/src/text_span.rs:150,168
```

Plus one `#[cfg(test)]`-gated site at `crates/prikk-store/src/lifecycle_cache.rs:80`
(`compute_window_hash`), which is test scaffolding and not production.

Not affected: `crates/prikk-store/src/trust.rs` and `crates/prikk-object/src/vectors.rs` consume only
`to_hex`, which is not being replaced.

You should not need to edit any of them. If you do, stop — the public API changed and that is out of scope.

## Step 0 — read this before touching anything

**Do not run `PRIKK_REGEN=1`.** Not once, not "just to see the diff."

`crates/prikk-object/src/vectors/snapshot.rs` holds 17 committed identity rows and documents regeneration
as the fix when the test fails. For every previous increment that was correct. For this one it is exactly
backwards: regeneration rewrites the expected ObjectIds to match whatever you just built, turns the test
green, and erases the only signal that the swap was not identity-preserving.

**In DC-55, a snapshot diff is a stop-work finding.** Report it with the differing rows and escalate. Do
not regenerate, do not adjust, do not investigate by regenerating first.

The same applies to the fixed vectors in `crates/prikk-hash/src/tests.rs` and the DC-40 state-root vectors
in `crates/prikk-store/src/state_root/tests/vectors.rs`: those are ground truth, four of them published by
NIST and the IETF. If the new code disagrees with FIPS 180-2, the new code is wrong.

## Step 1 — baseline, before any edit

Record, on the unmodified tree:

- Test counts per crate: `prikk-store` 543, `prikk-object` 76, `prikk-replay` 4, `prikk-hash` 13,
  `prikk-crypto` 5, `prikk-release-policy` 57. Confirm these still hold; they are the numbers to compare
  against, and a mismatch means the baseline moved under you.
- Locked package count: 180.
- Performance figures for the current `sha256` at 64 B, 4 KB, and 1 MB. You need the *before* numbers on
  *your* hardware — DC-50's were measured elsewhere and are not a valid baseline for your delta.

## Step 2 — the swap

In `crates/prikk-hash/Cargo.toml`, move `sha2 = { workspace = true }` from `[dev-dependencies]` to
`[dependencies]`.

In `tools/release-policy/src/boundary/placement.rs:7`, change `("prikk-hash", &[])` to
`("prikk-hash", &["sha2"])`.

**These two edits belong in the same commit.** `[dev-dependencies]` is deliberately outside the placement
gate's scope (see the doc comment on `dependency_entries`), so `sha2` passes today. The moment it lands in
`[dependencies]`, `boundary-check` fails closed until the allowlist names it. Splitting the edits creates a
commit where the gate fails — a broken bisect point on the one change where bisecting matters most.

In `crates/prikk-hash/src/lib.rs`:

- Reimplement the body of `sha256` (line 30) over `sha2::Sha256`. Keep the signature, the `#[must_use]`,
  and the `Sha256Digest` return type exactly as they are.
- Delete `H0`, `K`, and the compression machinery they feed — they have no other consumer.
- Leave `to_hex` (line 130) alone. It is not a hash function.
- Rewrite the crate doc at lines 6-8. It currently says the crate "intentionally contains a tiny
  first-party SHA-256 implementation for the initial source drop… A later RFC-backed decision may replace
  this." DC-55 *is* that decision; the paragraph must stop describing the crate as something it no longer
  is. Cite DC-50 for why.

While you are there: `#![forbid(unsafe_code)]` stays and stays true, but the hashing now happens inside a
dependency that uses `unsafe` for its accelerated backends. Do not let the crate documentation imply a
safety property the crate no longer provides by itself.

Root `Cargo.toml:48` declares `sha2 = "0.10"` under a `# Testing` comment. That heading is now wrong —
move the line to wherever production dependencies live in that file.

## Step 3 — prove identity did not move

Report these **individually**, not as "the suite passed." An aggregate pass is not evidence for an
identity claim; the point is to show each artifact was checked.

| # | Artifact | What to report |
|---|---|---|
| 1 | `crates/prikk-object/src/vectors/hard.rs` | Every test green, no regeneration. These are the hard FDD vectors and are never regenerated by design. |
| 2 | `crates/prikk-object/src/vectors/snapshot.txt` | All 17 rows unchanged, file byte-identical (`git diff --stat` shows it untouched). |
| 3 | `crates/prikk-store/src/state_root/tests/vectors.rs` | DC-40 state-root vectors green across the v2 leaf/node/root domains. |
| 4 | `crates/prikk-hash/src/tests.rs` | All 11 fixed vectors green — 4 canonical published, 7 independently computed. |
| 5 | `tests/fixtures/object-id-vectors.md` | Checked by hand; vector `5f8711b3…` still agrees with `snapshot.txt`'s `patch_payload` row. No test consumes this file, so a manual statement is what is being asked for. |
| 6 | Fresh differential | ≥10,000 cases against the replacing implementation. See Step 4 — whether this is meaningful depends on your disposition choice. |

## Step 4 — decide what happens to the DC-41 stage-3 differential

`crates/prikk-hash/src/tests/hash_differential.rs:135` asserts `sha256(&input) == reference_sha256(&input)`,
where `reference_sha256` wraps `sha2::Sha256`. Once `sha256` *is* `sha2`, that test compares a crate to
itself. It passes 10,000 times and means nothing — the exact tautology DC-41 stage 2 was written to forbid
("do not generate expected values with the implementation under test").

Pick one and record why:

- **Delete it.** Honest, and the simplest. Say plainly in the evidence note that the swap retires 10,000 of
  the 10,022 comparisons that justified it. What survives is stage 2's 11 fixed vectors, and they survive
  intact precisely because they were never computed by Rust.
- **Re-point it** at a genuinely independent third reference, keeping a real differential. Note this adds a
  dev-dependency, and `[dev-dependencies]` is outside the placement gate — so nothing will stop you. That
  makes the choice of reference a design question for review, not a mechanical one.

What you may not do is leave it in place, passing, with a name that still promises cross-implementation
coverage.

If you delete it, Step 3 row 6's fresh differential has nothing independent left to compare against — say
so rather than reporting a self-comparison as equivalence evidence.

## Step 5 — record which backend you exercised

`sha2 0.10.9` picks its backend at runtime via `cpufeatures`. If your host has no SHA-NI, everything above
proves equivalence for the **scalar fallback only**, while release builds on capable hardware take a
different code path.

Yes, they must agree — SHA-256 is deterministic and a divergence would be a `sha2` bug. That is the
assumption; verifying assumptions is what an equivalence campaign is for, and the stake here is every
ObjectId in existence.

Report which backend ran. Either run on hardware that selects the accelerated path, or force both and
compare. Do this on the same run as Step 6 — it is the same hardware question.

## Step 6 — performance, for DC-42's benefit

Re-measure at 64 B, 4 KB, and 1 MB and report before/after on your hardware. DC-50 measured roughly 5.8x
(220 vs 1265, 463 vs 2693, 470 vs 2732 MB/s) but on a different machine.

This is not re-litigation — DC-50 is closed and the decision does not depend on your numbers. It is so
DC-42 can set NFR-PERF-01 against a real, current baseline instead of deriving one itself.

## Traps

- **`PRIKK_REGEN=1`.** Covered in Step 0; repeated because it is the one action that can make this
  increment look successful while being wrong.
- **Splitting the manifest move from the allowlist amendment.** One commit.
- **Reporting "all tests pass"** instead of the six individual artifacts. The claim under review is
  identity equivalence, and a suite-level green does not distinguish "identical" from "consistently
  changed."
- **Editing a call site.** If one needs it, the API moved; stop and report.
- **Treating a vector disagreement as a vector problem.** Four of the fixed vectors are published by NIST
  and the IETF. They are not negotiable.
- **Touching `placement.rs` beyond the one entry.** It is a review-gated policy artifact under the DC-45
  precedent, not refactorable code.

## Definition of done

`prikk-hash::sha256` runs on `sha2`; the first-party compression code is deleted; the three public items
(`Sha256Digest`, `sha256`, `to_hex`) are unchanged; crate docs tell the truth; the allowlist and manifest
moved together; all six identity artifacts verified individually without regeneration; the differential's
fate decided and justified; the backend recorded; performance measured.

## Submit with

The diff; the evidence note covering all six identity artifacts individually plus the backend and
performance records; the differential disposition and its rationale; test counts per touched crate before
and after; the locked package count (expected to stay 180 — explain it if it did not); an explicit
statement of what did *not* change, naming the 11 production call sites and the persisted format; and the full
gate set from `rfcs/EXECUTION-ORDER.md` §6 rule 9, including release-policy `check`, `boundary-check`, and
`reference-check`.
