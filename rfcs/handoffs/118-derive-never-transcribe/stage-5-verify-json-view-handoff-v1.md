# RFC 118 stage 5 — `prikk verify --format json`, a derived view

**Base:** current `main` (`74a866c`, CI green). **Under `003-landing-work-on-main.md`.**
**Closes:** the ROADMAP:141 structured-output theme, by dissolving it (RFC 118 §10 prerequisite 3).

Stage 4 made the stage inventory complete and gated, and declared `label()`'s fourteen kebab-case
strings an external interface. **This stage renders that inventory, and a verdict, as JSON.**

**Read §2 first. The obvious implementation of the verdict is wrong, and it is wrong in a way that
would make the JSON actively dangerous rather than merely incomplete.**

---

## 1. Scope: emit the verdict and the stages, not the report

`RepositoryVerification` has **29 public fields** and roughly fourteen nested outcome/issue types
(`ObjectItemOutcome`, `RefPublicationIssue`, `CommitIndexDivergence`, `BlockSealVerification`, …).

**Do not serialize it.** Emitting all of it would freeze all of it as a public interface, forever,
in exchange for a CI gate that needs almost none of it. The theme's own stated requirement is that
*"`verify` [emit] something a job can assert on."*

**Emit exactly:**

- a `schema_version` string, following this repository's own convention (`release-policy-boundary-v1`
  and friends) — **name it and justify the name**;
- the **verdict** (§2);
- **one entry per `VerificationStage::ALL`**, in `ALL` order, keyed by `label()`, each carrying its
  `StageStatus` — `evaluated`, `failed` with its message, `not_evaluated` with `blocked_by`, or
  `halted` with `after`. **Stage 4's completeness gate guarantees exactly one per stage; the emitter
  must not re-derive that guarantee, and must not be able to silently drop one.**

**Counts and item-level findings stay out of v1.** If you believe one specific count is required for
the CI-gate use case, **name it and argue it** — do not add the set.

## 2. The verdict is the real work, and `has_blocking_defect()` is a trap

**There is no method on `RepositoryVerification` that means "verify would fail."**

The authoritative definition is an **else-if chain in `main.rs`'s `run_verify`**, hand-written, over
**nine** predicates: `has_stage_failure`, `has_item_failure`,
`has_active_wal_metadata_integrity_issue`, `has_blocking_ref_publication_issues`,
`has_publication_trust_issues`, `has_commit_index_divergence`, `has_lifecycle_cache_divergence`,
`has_active_wal_ordering_issue`, `has_merge_baseline_divergence`.

**`has_blocking_defect()` is `has_stage_failure() || has_item_failure()` — two of the nine** — and its
own doc says it is *"kept as public API for an external caller that only wants the yes/no answer."*

**That is exactly what you are about to build, and taking that invitation would emit `ok: true` for a
repository `prikk verify` itself exits nonzero on** — publication-trust issues, commit-index
divergence, lifecycle-cache divergence, WAL-ordering issues, merge-baseline divergence, ref-publication
issues, and active-WAL metadata integrity issues would all be reported as healthy, **to a CI gate whose
entire purpose is to catch them.** A machine-readable answer that is confidently wrong is worse than
prose a human reads.

**So: declare the failure conditions once, and have both consumers read that declaration.** The exit
chain in `main.rs` and the JSON verdict must derive from **one** list — not two lists that agree today.
Each condition needs a stable identifier (the JSON names it) and its human sentence (the prose keeps
it). **The exit chain's distinct per-condition messages must survive**; that is why it is a chain and
not a boolean, and collapsing it would be a regression.

**Adjudicate `has_blocking_defect()` explicitly** — leave it with a corrected doc, or remove it. **Do
not leave its current doc standing**, because it advertises a wrong answer to precisely the caller this
stage creates. If removing it breaks something, say what.

**The three predicates not in the chain** — `has_blocking_defect`, `has_trailing_partial_wal`,
`has_active_wal_metadata_warning` — **are outside it deliberately or by omission, and I do not know
which.** Determine it and say. **If any belongs in the chain, that is a live defect in `prikk verify`'s
exit code, and it outranks this increment — stop and report it.**

## 3. There is no hand-rolled-JSON precedent here. ROADMAP:141 is wrong about this.

The theme says *"`release-policy`'s existing `--format json` is the precedent."* **It is not a usable
one:** `tools/release-policy` depends on `serde`, `serde_json`, and `jsonschema`. **`prikk-cli` has no
third-party dependencies** and must keep none (RFC 118 §10 prerequisite 4).

**So `prikk-cli` will be the first hand-rolled JSON emitter in this repository, and every escaping
mistake is yours to avoid rather than inherit.**

`StageStatus::Failed { message }` carries **arbitrary error text** — quotes, backslashes, newlines,
tabs, and control characters all reach it from `PrikkError` and from paths. **Write a real escaper**
(`"`, `\`, and `U+0000`–`U+001F` as `\u00XX`, at minimum) and **prove it on hostile input**, not on a
happy path. **Emit valid JSON or do not emit.**

**Correct ROADMAP:141's precedent sentence** as part of this increment — it is wrong on the record and
would mislead the next reader the same way.

## 4. Interface discipline

- **`label()`'s strings are keys. Do not rename any.**
- **A stage absent from the output is a bug, not a shorter document.** Every `ALL` entry appears,
  always, including `not_evaluated` and `halted` ones.
- **Argument handling** follows the existing `parse_verify_args` shape; `--format json` is additive
  and the **default output must be byte-identical to today's prose**. Prove it.

## 5. Out of scope

- Serializing the 29 fields or any nested outcome type (§1).
- `--format json` for any other command. **This is `verify` only.**
- Changing what any stage checks, or any verification behaviour.
- `doctor`'s `is_healthy()` — a different report type, deliberately untouched.

## 6. Controls

1. **Hostile-string escaping**: force a stage `Failed` whose message contains `"`, `\`, a newline, a
   tab, and a control character; show the emitted JSON parses. **Quote the raw bytes.**
2. **The verdict catches a non-stage failure**: construct a repository that trips one of the seven
   conditions **outside** `stage_failure`/`item_failure` — publication-trust or commit-index
   divergence is easiest — and show the JSON reports failure **and** names the condition. **This is
   the control that proves §2 was actually solved**; a green run here with `has_blocking_defect()`
   underneath would say `ok: true`.
3. **One declaration, two consumers**: remove a condition from the declared list and show **both** the
   exit chain and the JSON lose it together. If they can disagree, the derivation is not real.
4. **Every stage always present**: assert the JSON carries all fourteen under `stop_on_first_error`,
   including `halted` entries.
5. **Prose output unchanged**: byte-compare default `verify` output against `74a866c`.

**Quote every failure.** If a mutation fails to apply or a control passes without your assertion
firing, **say so** — a control that passes for the wrong reason is worse than none.

## 7. What to report

1. **The schema name**, and the emitted JSON for one healthy and one failing repository.
2. **How the failure conditions are declared once**, and how both consumers read it (§2).
3. **Your `has_blocking_defect()` adjudication**, and the disposition of the three out-of-chain
   predicates.
4. **All five controls** (§6), quoted.
5. **Full gate set against the exact commit, after the last edit.**
6. **Every numbered requirement's disposition, including ones that went without incident** — stage 4
   discharged §4 correctly but did not report it, and a silent discharge is indistinguishable from a
   miss until someone checks.
7. Anything here that was wrong.

**Stop and escalate, do not guess**, if: one of the three out-of-chain predicates belongs in the chain
(§2); the single-declaration shape cannot be built without a dependency (§3); or the JSON cannot carry
a condition's identity without exposing an internal type — **naming is negotiable, leaking the report's
internals is not.**
