# DC-41 Stage 4 - Property/Fuzz (Implementation Handoff)

**Authority.** Elaborates the accepted RFC's stage-4 acceptance bar
(`rfcs/accepted/DC-41-INTEGRITY-EVIDENCE-CAMPAIGN.md`, "Stage 4 - Property/fuzz") and its dependency plan.
It adds no requirement beyond that bar, and **§3 flags one place where the bar as written cannot be met
literally** — that needs an explicit decision, recorded, not a silent reinterpretation. Items marked
*recommended* are design proposals the implementer may decline with a recorded reason.
**Authored by** the architect in the function-designer role. Stage-4 implementation review remains
independent, because implementation is authored by a different developer.
**Predecessor.** Stage 3 committed as `540d4db`; `Cargo.lock` baseline is now
`18a8b40aa83396974c2cacd9af56e7496d9f645cd07bda0e4164e4d0b68f0d53`.
**Scope.** Stage 4 only — the final DC-41 stage. No CI job added; no production code.

---

## 1. This stage is not like stage 3

Stage 3 turned out to be a three-line dependency *edge*. Stage 4 is the real dependency change, and the
only DC-41 stage that meaningfully grows the graph. Measured, not estimated — via a reversible
`cargo add proptest --dev -p prikk-object` against the real repository, then restored (`Cargo.lock` back
to `18a8b40a…`):

| Effect | Measured |
|---|---|
| Locked package count | **169 → 180** (+11) |
| New crates | `fnv`, `ppv-lite86`, `proptest 1.11.0`, `quick-error`, `rand 0.9.4`, `rand_chacha 0.9.0`, `rand_core 0.9.5`, `rand_xorshift`, `rusty-fork`, `unarray`, `wait-timeout` |
| Duplicate versions introduced | **`rand_core`** — 0.9.5 arrives alongside the existing 0.6.4 (different semver majors, so both persist) |
| `cargo tree -d` | **clean today → reports `getrandom` 0.2.17 / 0.3.4 / 0.4.3 after**. Those versions are already in the lockfile; adding `proptest` makes them *reachable* in the default tree view, so the tool starts reporting them. |
| Rust 1.85 | `proptest 1.11.0` **builds and tests PASS** (verified in an isolated scratch crate) |

Everything else resolved to prikk's existing pins — Cargo did **not** bump `bitflags`, `libc`, `syn`,
`quote`, `zerocopy`, etc., even though a fresh resolution would have.

**Two things to do rather than be surprised by:**

- **Report the `cargo tree -d` change as expected.** It has been clean at every review since onboarding.
  Someone will notice. It is a consequence of dev-only test tooling, not of product dependency drift, and
  it should be stated in the evidence note — not "fixed."
- **Re-verify MSRV on the integrated workspace.** My check was `proptest` in isolation. Run
  `cargo +1.85.0 test --workspace --locked` on the real candidate before proposing the version; if it
  fails, pin older rather than weakening DC-46's release-blocking contract.

Expect a second `Cargo.lock` re-freeze, and record the new hash as superseding `18a8b40a…`.

## 2. Where proptest must be a dev-dependency, and why it matters

Two of the four target families are **not reachable from integration tests**:

| Target | Entry point | Visibility |
|---|---|---|
| Object-envelope framing | `prikk-store::file_codec::decode_envelope_file` | `pub(crate)` |
| WAL record framing | `prikk-store::wal::decode_records` | **private** `fn` |
| Payload decoding | `prikk-object::payload::*::decode_canonical` | `pub` |
| Replay/lifecycle reconstruction | `prikk-store` lifecycle-cache path | crate-internal |

So the property tests must live **inside** the crates, in the existing `src/**/tests.rs` structure — not in
`tests/`. That means `proptest` is a dev-dependency of **`prikk-object`** and **`prikk-store`** (add it to
`prikk-replay` only if you target its types directly). Do **not** widen any visibility to make a target
reachable — that would be a production change, out of scope.

`decode_records` being private is fine: `wal/tests.rs` is a child module of `wal`, so it can call it.

## 3. The RFC's target list cannot be met literally — decide and record

The RFC lists as a stage-4 target *"canonical object-envelope decoding for every current `ObjectType`
variant (`Patch`, `Block`, `RefState`, `RefUpdate`, `Tag`, `Attestation`, `Blob`, `BlockSummaryCache`,
`RecoveryNote`, `ProjectGenesis`)"*.

I verified the variant list is accurate — those are exactly the ten in `prikk-object/src/id.rs`. But
**only five have a canonical payload decoder**:

| Has `decode_canonical` / equivalent | No payload decoder |
|---|---|
| `Patch` (`payload/patch.rs:130`), `Block` (`block.rs:64`), `RefState` (`refs.rs:56`), `RefUpdate` (`refs.rs:159`), `Blob` (`blob.rs:63`) | `Tag`, `Attestation` (modules exist, no decoder), `BlockSummaryCache`, `RecoveryNote`, `ProjectGenesis` (no module) |

This is my omission at design review — I checked the ten variants against the enum but not against the
decoders. The bar as written is unmeetable for five of them.

**Recommended resolution (split the target by layer):**

- **Envelope layer — all ten variants are meaningful.** `decode_envelope_file` parses the type code and
  schema generically, so generate envelopes carrying each of the ten type codes and assert admission
  behaves per the DC-40 format allowlist. This is genuinely valuable: `BlockSummaryCache`, `RecoveryNote`,
  and `ProjectGenesis` are *rejected* from format-2 identity positions, so these cases exercise the
  rejection path rather than a decoder.
- **Payload layer — the five that exist.** Round-trip and malformed-input rejection against the five real
  decoders.

Record the split in the evidence note and state that the five decoder-less variants are covered at the
envelope layer only, with the reason. Do not silently drop them, and do not add decoders to satisfy the
wording — that would be production work outside DC-41.

## 4. Target list and properties

For each target, the two properties worth asserting:

- **Round-trip:** `decode(encode(x)) == x` for generated valid values.
- **Malformed-input rejection:** arbitrary bytes either decode successfully or return an error — **never
  panic**, never hang, never return a value that re-encodes differently.

| # | Target | Property |
|---|---|---|
| 1 | Envelope framing (all 10 type codes) | decode of arbitrary bytes is total; type/schema admission matches the DC-40 allowlist |
| 2 | Payload decoders (5) | round-trip and total-decode |
| 3 | WAL record framing (`decode_records`) | round-trip via `encode_record_for_test`; arbitrary bytes yield a `WalReplay` or an error, with trailing-partial handling exercised |
| 4 | Ref-log entry framing | as above for inline `RefUpdate` records |
| 5 | Patch operation decoding | round-trip with **bounded generation** |

**Generation bounds are test-tractability limits, not production thresholds.** The RFC is explicit and it
is worth restating in code comments: op count, path depth, path segment length, and content size bounds
exist to keep property tests fast. They are **not** the `NFR-PERF-02` active-block thresholds (800/1000),
which govern an unrelated concern. Do not let the two names drift together.

## 5. Budgets and CI wiring

- **Fast budget:** 256 cases/target, enforced on every CI run (`PROPTEST_CASES` or
  `ProptestConfig { cases: 256, .. }`).
- **Campaign budget:** 100,000 cases/target, run at least once, results recorded. Not gating ordinary CI.
- **Measure first.** Stage 3 measured ~15µs/case for SHA-256. Decoders will be slower, and there are five
  targets rather than one. Measure the fast-budget wall time and report it; if 256×5 is materially slower
  than stage 3's 0.15s, say so rather than letting CI quietly lengthen.
- **Existing jobs only.** Wire into `cargo test --workspace --locked`, which both `stable` and
  `msrv-1.85.0` already run. **Do not add a CI job** — a new job means a governed-procedure-file
  classifier amendment under DC-45/DC-47/DC-48, which stage 4 does not need and which is the descoped
  platform matrix's problem, not yours.

## 6. Corpus policy — already decided, follow it

Only minimized `proptest-regressions` failure files are committed, **one packed file per crate**, not one
file per case. Generated (non-failure) corpora are not committed and regenerate from the seed. This was
settled at design time specifically to avoid re-running the DC-45 237-file objection at owner-acceptance
time — do not reopen it by committing a large corpus.

`proptest` writes `proptest-regressions/<module>.txt` by default. Consolidate or configure so the tracked
footprint stays one file per crate, and state the resulting file count in the evidence note.

## 7. The clause most likely to be exercised

*"A discovered behavior defect opens a dedicated corrective RFC instead of being silently normalized into
a test expectation."*

Stages 1-3 found nothing, because they targeted a single well-understood pure function and existing
well-asserted tests. **Stage 4 targets canonical decoders under arbitrary input, which is where something
will plausibly be found.** If a decoder panics, hangs, or accepts input that re-encodes differently:

1. **Stop.** Do not adjust the generator to avoid the input, do not add a `#[should_panic]`, do not encode
   the observed behaviour as expected.
2. Commit the **minimized reproducer** (that is what `proptest`'s shrinking is for).
3. Open a corrective RFC referencing it, and record the open follow-up in the stage-4 evidence note.
4. Stage 4 can still be accepted with a recorded finding — the RFC's bar is "zero **unexplained**
   findings," not "zero findings."

A malformed-input panic in a canonical decoder would be a genuine robustness defect against NFR-SEC-04
("malformed objects, WAL entries… never panic or corrupt state"), so finding one is a success for the
campaign, not a failure of the stage.

## 8. Definition of done

- `proptest` in `[dev-dependencies]` of `prikk-object` and `prikk-store` only — never `[dependencies]`.
  Per the stage-1 B4 finding, **no mechanical gate catches misplacement**; verify by direct manifest
  inspection and state it explicitly.
- All five target families have a property test at the fast budget, wired into existing CI jobs.
- The §3 envelope-vs-payload split is implemented and its reasoning recorded.
- Campaign budget run at least once; results recorded.
- Zero unexplained findings; any finding has a committed minimized reproducer and an open corrective-RFC
  reference.
- `Cargo.lock`: package count reported (expect 169 → ~180), **new hash recorded** as superseding
  `18a8b40a…`; `cargo tree -d` change reported as expected.
- Test counts reported before/after (`prikk-object` 64, `prikk-store` 531, `prikk-hash` 13 at this
  baseline).
- Fast-budget CI wall-time measured and reported.
- Tracked `proptest-regressions` footprint stated (target: one file per crate).
- Frozen identities otherwise unchanged: `Cargo.toml`, other package manifests, both command inventories,
  oracle manifest, `release-signers.toml`.
- Gates green: `cargo fmt --all -- --check`;
  `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
  `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`; `git diff --check`;
  `cargo audit --no-fetch`; release-policy `check`, `boundary-check`, `reference-check`.

## 9. Submit for implementation review with

- The diff, with manifest changes clearly visible.
- An evidence note covering: the §3 target-split decision and reasoning; per-target properties; fast and
  campaign budgets with measured runtimes; before/after `Cargo.lock` hash and package count; the
  `cargo tree -d` delta; before/after test counts; tracked regression-file footprint; and any finding with
  its reproducer and follow-up reference.
- Gate output per §8.
- An explicit statement that `proptest` is in `[dev-dependencies]` and not in `[dependencies]`.

## 10. After stage 4

DC-41's four stages are then complete, and the campaign's completion condition is met **except** the
descoped platform matrix, which is not a DC-41 completion condition. At that point:

- DC-41 becomes eligible to move to `done/` **only when it ships in a release** under RFC-000 — it is an
  unshipped accepted increment until then, and inherits release conditions accordingly.
- The **first-party SHA-256 ROI question** becomes answerable on stage 3's evidence. That is a recorded
  deferred decision, not a dropped thread — it needs its own increment.
- The **platform matrix** remains blocked on the M1 portability-claim correction, which happens inside an
  activated release lane.

---

**Boundaries.** Stage 4 grants no authority to add a CI job, add the platform matrix, widen any target's
visibility, move DC-41 to `done/`, or take any release-lane action. The release lane is **parked**;
nothing here activates it, and architect recommendations are explicitly non-authoritative for activation.
DC-39 and DC-40 remain unshipped M1 increments, and the 0.17.7 no-go for production, repository-format
stabilization, and public preview stands.
