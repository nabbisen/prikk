# RFC 141 §7b — tighten the evidence schema so a dishonest document cannot validate

**RFC:** `rfcs/accepted/141-publication-through-ci.md` — **§7b is the ruling and is settled input.**
**Base:** `main` at `86f5368`.
**Origin:** increment 1's own finding (`555cc65`), which proved a control this project's handoff
specified does not guard the hazard it was named for.

**This is small. The pre-checks in §3 are done — read them before estimating it.**

---

## 1. What is wrong

`release/schemas/release-evidence-v1.schema.json` lets a crate row claim
`"checksum_equality": "match"` while all three of `staged_sha256`, `registry_checksum` and
`fetched_sha256` are `null`.

**Verified structurally:** `$defs/crate` carries no `allOf`/`if` at all; `checksum_equality` is a bare
enum; the only top-level conditionals are `sequence == "001"` and `overall_status == "complete"`, and
the checksum constraint lives **only inside the second**. So any `pending`, `partial` or `superseded`
document may assert an equality nobody checked and be schema-valid.

**Why this matters more than a missing constraint.** Release evidence exists to be trustworthy
*without trusting its producer*. A document claiming unverified equality is not a gap — it is a false
record wearing DC-35's authority, which is worse than the absence we had before increment 1.

## 2. The rule already exists in Rust — lift its presence half into the schema

**Do not invent a rule.** `tools/release-policy/src/policy/evidence.rs::crate_checksum_state_valid`
already encodes what this project means by an honest crate row:

| `checksum_equality` | Rust already requires |
|---|---|
| `"match"` | all three present **and all three equal** |
| `"mismatch"` | all three present **and not all equal** |
| `"not-observed"` | nothing |

**Add to `$defs/crate` a conditional expressing the presence half:** if `checksum_equality` is
`"match"` or `"mismatch"`, then `staged_sha256`, `registry_checksum` and `fetched_sha256` must each be
a `sha256` (not `null`).

**The equality half cannot be expressed in JSON Schema** — there is no cross-field value comparison.
It stays in the Rust validator. **Say so in the schema's own `description` for that conditional**:
the schema bounds shape, the validator bounds agreement. A reader who finds only the presence rule
should learn from the schema why, not conclude that equality is unchecked.

**Do not weaken or duplicate the Rust check** to make the two look symmetric. They are asymmetric for
a real reason.

## 3. Pre-checks already done — nothing in the repository currently violates the new rule

**Do not spend the round discovering this. It is measured:**

- **All ten fixtures** under `release/fixtures/release-evidence-*.json` — `pending`, `partial`,
  `complete`, `superseded`, and the six hold variants — carry **zero** crate rows with the dishonest
  pattern.
- **All 146 entries** in `release/oracle/packs/release-evidence-v1.json` — scanned recursively for any
  object carrying `checksum_equality` — contain **zero** such rows.

**So this change should break nothing.** That is the expectation, and it is what makes the increment
small.

**Therefore: if something does break, it is a finding, not a fixture to edit.** A fixture that starts
failing is telling you it was asserting something dishonest. **Report it; do not quietly correct the
fixture to make the suite green.**

## 4. What to add, beyond the constraint

**A negative case.** Today nothing asserts that a dishonest document is rejected — which is precisely
why the gap survived. Add an oracle case, or a schema-level test, whose document claims `"match"` over
`null` checksums and **must be refused**. Without it the new constraint is itself unguarded.

**Check whether `produce`'s self-validation now catches more.** Increment 1's `produce` validates its
own output against this schema before returning. After this change it should reject a would-be
dishonest document at that point. **Confirm it does, and say so** — that is the shortest path from
this constraint to a real safety property.

## 5. Out of scope

- **§7a's `CRATE_ORDER`.** A separate ruling, belonging to increment 4, and it must not be touched
  here — changing it changes what the oracle asserts about crate identity and count, which is a
  different decision with a different trade.
- **The Rust validator's equality check.** Correct as it stands.
- **Any change to what evidence a release produces.** This is a contract change, not a producer
  change.
- **`release-signers.toml`.** Untouched, as always.

## 6. Controls

1. **A dishonest document is rejected.** `"match"` (and separately `"mismatch"`) over three `null`
   checksums fails validation, at `pending`, `partial` **and** `superseded` — not only at `complete`,
   since the whole point is that the old constraint was `complete`-only.
2. **An honest document still validates**, in every status: `"not-observed"` with three `null`s, and
   `"match"` with three present values.
3. **Partial absence is caught too** — two present and one `null` under `"match"` must fail. A rule
   written against "all three null" would pass this and be wrong.
4. **The ten fixtures and the oracle pack still pass**, unchanged. If any needs editing, stop (§3).
5. **`produce`'s self-validation rejects the dishonest document** (§4).

**Each control seen to fail before it passes**, with the perturbation reported per control, as the
last three rounds did.

## 7. Gates

The full set, verbatim from `rfcs/EXECUTION-ORDER.md` §6 rule 9:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test --workspace --locked`
- `cargo +1.85.0 test --workspace --locked`
- `cargo +1.85.0 check --workspace --all-targets --locked`
- `git diff --check`
- `cargo audit --no-fetch`
- `RUSTDOCFLAGS="-D rustdoc::private_intra_doc_links" cargo doc --workspace --no-deps`
- release-policy `check`, `boundary-check`, `reference-check`

**`check` runs the oracle's 73 release-evidence cases against this schema.** Run it early and often;
it is the gate that will notice first if §3's expectation is wrong.

## 8. No `CHANGELOG.md` entry

A repository-internal schema for a `publish = false` tool. Nothing a user can observe. **Ruled here
rather than left unsaid.**

## 9. Reporting

`.git-exclude/review-request/`. Include:

- the per-control perturbations;
- **whether §3's pre-check held** — if any fixture or oracle entry needed to change, that is the
  headline of your report, not a footnote;
- the wording you gave the schema `description` explaining the shape/agreement split;
- **whether you think the equality half should eventually move into the producer's own output** rather
  than living only in a validator most documents never reach (§7a's `CRATE_ORDER` currently prevents
  real documents from reaching it at all). Not work for this round — an opinion worth having on record.
