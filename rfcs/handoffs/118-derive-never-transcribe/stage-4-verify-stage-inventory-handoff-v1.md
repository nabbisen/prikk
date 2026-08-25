# RFC 118 stage 4 — derive `verify`'s stage inventory, and gate its completeness

**Base:** current `main` (`6b8901f`). **Under `003-landing-work-on-main.md`.**
**Discharges:** RFC 118 §10 prerequisite 3's second candidate — `verify`'s stage inventory.

**This is stage 1 of 2.** Stage 2 emits `prikk verify --format json` as a *derived view* of this
inventory, dissolving the ROADMAP:141 structured-output theme rather than adding to it. **The order is
forced, for the reason in §1 — do not attempt stage 2 here.**

---

## 1. Why the JSON cannot come first

`RepositoryVerification::stage_outcomes` carries the invariant, in its own words:

> **Always exactly thirteen entries — no stage may be silently absent.**

**`VerificationStage` has fourteen variants.** I counted them mechanically, not by eye:
`Objects`, `Refs`, `RefUpdateSchemaTrust`, `WalReplay`, `WalPersistence`, `RollbackDrafts`,
`WalRecordSchema`, `ActiveWalMetadata`, `PublicationReclassification`, `CommitIndex`,
`LifecycleCache`, `WalOrdering`, `ReceivedRefs`, `LocalTagTrust`.

**Nothing asserts the count.** `stage_outcomes.len()` is never checked anywhere in the crate; there is
no `ALL` constant to check it against.

**So a machine-readable `verify` built on this today would be worse than the prose it replaces.** A CI
job asserting *"every stage evaluated"* would pass while a stage was **absent from the report entirely**
— the exact failure the structured-output theme exists to prevent, promoted from a wording nuisance to a
silent gate bypass. **Completeness must be gated before the inventory becomes an interface.**

**This staleness is mine.** `LocalTagTrust` was added in the DC-78 verify-local-tag-trust increment,
which I designed and reviewed. The enum's own doc (`verify.rs:380`) was updated to "fourteen"; **eight
other transcriptions of the count were not.** That is RFC 118's thesis with my own fingerprints on it,
and it is the reason this stage exists.

## 2. Derive the count — do not correct it

**Correcting "thirteen" to "fourteen" in eight places is the wrong fix** and will be rejected. It
re-arms the identical trap for whoever adds the fifteenth stage.

**Known stale sites** (`crates/prikk-store/src/verify.rs`): lines **39, 62, 505, 514, 515, 657, 938,
950**. **Re-derive this list yourself** — `grep -rn "thirteen\|fourteen"` across `crates/prikk-store/src`
— and report any I missed rather than trusting these eight.

**Give `VerificationStage` a canonical `ALL` slice**, and make the count derive from it
(`VerificationStage::ALL.len()`). Then **rewrite each site so it does not name a number at all** —
prose that says "every stage" or "each stage in [`VerificationStage::ALL`]" cannot go stale. **Where a
doc genuinely needs the number, it must read it from the slice, not spell it.**

**`ALL` must itself be gated**, or it is one more hand-maintained list: a test must fail if a variant
exists that `ALL` omits. **An exhaustive `match` over `self` returning a discriminant, checked to cover
every variant, is the shape that makes the compiler the gate** — pick your mechanism, but say why it
cannot drift, and prove it with control 2.

## 3. Gate the invariant the doc already promises

**Assert that a real `verify_repository` report carries exactly one outcome per `ALL` entry**, on a real
repository — not a hand-built `RepositoryVerification`. A test that constructs the value it then checks
proves nothing about the pipeline.

**Cover the `stop_on_first_error` path too.** `Halted` exists precisely so an early stop still emits an
entry per stage; that is the case most likely to drop one, and the one a CI gate would most likely hit.

## 4. `label()` is already the interface — treat it as such

`label()` already returns stable kebab-case names for all fourteen stages (`objects`, `refs`,
`ref-update-schema-trust`, …). **Stage 2 will use these verbatim as JSON keys.**

**So say plainly in `label()`'s own doc that these strings are an external interface** and that changing
one is a breaking change to tooling — before stage 2 makes that true silently. **Do not rename any.**

## 5. Out of scope

- **All JSON, `--format` handling, and CLI output.** That is stage 2.
- **Adding, removing, renaming, or reordering stages.**
- **Changing `StageStatus`** or any verification behaviour. **This increment must not alter what
  `verify` decides** — only what is derived about its shape.
- **`rollback_verify.rs`** and `refs/verify.rs`, which are separate entry points.

## 6. Controls

1. **The completeness gate fires on a real omission** — remove one stage's push into `stage_outcomes`
   in the pipeline (a reverted source mutation) and quote the failure. **Not** a stage I would guess:
   use one of `WalOrdering`, `ReceivedRefs`, or `LocalTagTrust`.
2. **The `ALL` gate fires on a forgotten variant** — add a throwaway variant and show it cannot be
   omitted from `ALL` silently. Quote it, then revert.
3. **The `stop_on_first_error` path still emits every stage** — assert it, and say which stage halted.
4. **No behaviour changed**: full suite green, and say whether the count moved and why.

**Quote every failure. A control that passes for the wrong reason is worse than none** — if a mutation
fails to apply, or the run reports `ok` without your assertion firing, **say so**.

## 7. What to report

1. **Your re-derived stale-site list** (§2), including anything beyond my eight.
2. **The `ALL` mechanism**, and **why it cannot drift**.
3. **All four controls** (§6), quoted.
4. **Full gate set against the exact commit, after the last edit.**
5. Anything here that was wrong.

**Stop and escalate, do not guess**, if: the completeness invariant turns out to be **false today** on
some path — a stage genuinely absent under some option — because that is a live defect in `verify` and
outranks this increment; or if `ALL` cannot be made compiler-gated without a dependency, since
`prikk-cli`'s zero-third-party-dependency constraint (RFC 118 §10 prerequisite 4) governs stage 2 and I
do not want it worked around here.
