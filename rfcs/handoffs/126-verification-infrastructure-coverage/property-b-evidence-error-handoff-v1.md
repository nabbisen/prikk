# Property B found something real, and we cannot replay it — make it reproducible first

**Authority:** `rfcs/done/126-verification-infrastructure-coverage.md` §2, whose property tests
landed at `8608db0`.
**Base:** current `main` (`b81b1eb`). **Under `003-landing-work-on-main.md`.**

**This blocked the 0.30.0 release**, which is held on the owner's authorization until this lands.

**Scope: make the failure reproducible and keep the sweep honest. Do not change
`patch_algebra/commutation.rs`.** Whether the production classification is wrong is the architect's
ruling, and it needs a captured case rather than an argument — which is exactly what this increment
produces.

---

## 1. What happened

CI failed one job of fifteen on the release commit, macOS only:

```
algebra_properties.rs:770 — evidence is fully registered for every generated candidate:
  Malformed { scope: UnsealedCandidateOptional, fact: Operation,
              reason: "composed replay failed after confluence proof" }
```

**It is not macOS-specific and not caused by that commit** — which touched only `Cargo.toml`,
`Cargo.lock` and `CHANGELOG.md`. **I reproduced it on Linux**, twice: clean at 4,000 cases (six runs)
and at 250,000; it fires at **1,500,000**. Incidence is roughly one in several hundred thousand.
macOS drew an unlucky seed at the standing 4,000.

**What it means.** `check_confluence` proves every cross pair between the two sequences commutes, then
`replay_sequence_order` composes them and the composed replay **fails**. `commutation.rs:224-231`
treats that as unreachable — its reason string literally reads *"after confluence proof"*. **The
assumption is wrong.**

**Severity is bounded**: the system refuses rather than accepts, so a user meeting this in a merge
gets a malformed-evidence error instead of a clean conflict report. Bad diagnosis, not data loss.

## 2. The two defects you are fixing (neither is in the algebra)

**2.1 The failing seed is not saved.** The run prints:

```
proptest: FileFailurePersistence::SourceParallel set, but no source file known
```

`property_b` builds its runner with `TestRunner::new(Config { cases, ..Config::default() })`, and the
default persistence needs a source path the macro form supplies and this form does not. **So every
reproduction costs a ~90-second 1.5M-case hunt and the case is lost again.** Set persistence
explicitly so the seed lands in `crates/prikk-store/proptest-regressions/`, alongside the two files
already tracked there.

**Once the seed is persisted, proptest replays it first on every run** — the case becomes a
deterministic sub-second test rather than a lottery. **That is the whole point: do not raise `cases`
permanently.** 1.5M cases is ~90 seconds on every gate invocation and would be paying forever for
something a saved seed gives free.

**2.2 The test contradicts its own stated design.** Line 784 asserts *"no generated case hard-fails;
every finding is collected, not asserted per-case"* — and line 770 hard-fails on exactly such a
finding, via `.expect()`.

## 3. How the sweep should treat it — the idiom this file already uses

**Do not make the test simply tolerate evidence errors.** That would hide the finding and is the
"gate that cannot fail" shape this project has refused twice.

**Bucket by reason, and allowlist with reasons** — the shape `pair_class_bucket` /
`unknown_reason_bucket` (`algebra_properties.rs:449,459`) already establish in this same file, and
that `UNSAFE_EXEMPT_CRATES` and `DECLARED_UNDOCUMENTED` establish elsewhere:

- Collect every `EvidenceError` by its reason string into a named bucket.
- **One entry is allowlisted**: `"composed replay failed after confluence proof"`, with a comment
  naming this handoff and stating that the classification question is open and the architect's.
- **Any unlisted reason hard-fails**, exactly as today.

**Do not assert a count.** The seed is random per run, so occurrence counts vary; assert on the
*reason*, never on how many times it appeared. Report the count you observe, but do not pin it.

This keeps `main` green, keeps the finding visible in the sweep's own output, and keeps the test
lethal against anything new.

## 4. What to report — this is the increment's real deliverable

**The shrunk failing input**, in enough detail for the classification ruling:

1. The `baseline_spec` proptest shrank to.
2. The `left` and `right` operation sequences — every `OpChoice`, in order.
3. Which cross pairs `check_confluence` classified, and as what.
4. **Which operation in the composed replay failed, and why** — the `OracleFailure::Replay` arm is
   reached from `replay_operations`; say what precondition was unmet.
5. Whether either sequence alone replays cleanly. **If both do and the concatenation does not, say so
   explicitly** — that is the crux of the ruling.

**Do not draw the conclusion.** Whether this is a production classification defect or a generator that
builds sequence pairs real authoring cannot produce is mine to rule, on your evidence.

## 5. Traps

- **A proptest regression file is tracked.** After any run that fails on purpose, check `git status`
   — this project has been caught by that before.
- **Confirm the persisted seed actually reproduces it** in a fresh checkout, in under a second. A seed
  that does not replay is worse than none, because it looks like coverage.
- **Do not touch `commutation.rs`**, and do not "fix" the generator to stop producing the case. If you
  believe the generator is wrong, say so in the report with evidence — that is one of the two answers
  the ruling chooses between.

## 6. Gates and reporting

Full set from `rfcs/EXECUTION-ORDER.md` §6 rule 9 against your final commit — **not reproduced here**:
`reference-check` rejects a policy-command line outside its registered sites. **Rule 9 gained
`cargo +1.85.0 check --workspace --all-targets --locked` on 2026-09-03.**

**Run the full suite several times.** A single green run proves nothing about a test whose seed is
random; the persisted-seed replay is what must be deterministic.

Local commits on `main`; **no push, no tag, no publish.** Report to `.git-exclude/review-request/`,
and state §4's five items, the case count at which you first reproduced it, and confirmation that the
saved seed replays in under a second.
