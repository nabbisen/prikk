# DC-95 Stage 2 — Design v1

**Author.** Architect. **Independence.** Author-reviewed — the standing ceiling; compensated at
implementation review, per the RFC's status field.
**Inputs.** Stage 1 (complete, merged `5477df5`), and the §3.3–§3.4 prerequisite ruling's four binding
constraints.
**Status.** Design for review. **No implementation authorized by this document.**

## 1. The decision: contain errors by scope, do not convert checks individually

The obvious reading of *"collect findings rather than stop at the first"* is to rewrite ~30 hard-`Err`
sites into `Vec` pushes. **Reject that.**

`verify_repository` is a linear pipeline of twelve `?`-propagating stages, each covering one scope —
objects, refs, per-envelope schema/trust, WAL replay, WAL persistence, rollback drafts, per-record
schema, active-WAL metadata, publication reclassification, commit index, lifecycle cache, WAL ordering.
**The short-circuit lives at those twelve boundaries, not inside the checks.**

**Design: make each stage boundary contain its own failure.** A stage that returns `Err` records that
error as a finding against its scope; the pipeline continues to the next stage. Checks are not
rewritten.

Two levels, independently shippable:

- **Level 1 — stage containment.** The twelve top-level `?` become recorded findings. Delivers "every
  category reported" immediately.
- **Level 2 — item containment.** Inside the two iterating stages (`verify_objects`, `verify_refs`),
  per-object and per-ref containment, so one malformed object does not hide every other object's
  verdict.

## 2. Why this rather than per-check conversion

1. **It preserves the boundary principle instead of relitigating it.** The prerequisite ruling adopted
   *structural stays hard, semantic accumulates.* Under scope containment, a structural failure still
   aborts — **it just aborts its unit rather than the run.** That is the principle applied at the right
   scope, and it dissolves the per-check adjudication of ~30 sites, which is where a sweeping refactor
   would silently lose a check.
2. **It satisfies the repair-gate constraint by construction.** A contained stage error is blocking by
   default, so `repair_repository`'s refusal is preserved without a per-field severity audit. Constraint
   (1) of the ruling — every new type carries a blocking flag — is met by there being *one* new type.
3. **The 27 Stage 1 tests keep their meaning.** Their reachability lessons stay true; only the assertion
   shape changes, uniformly, to "a blocking finding exists for this scope with this message."
4. **It does not reorder the pipeline**, so the `classify_ref_state` → `require_retained_evidence`
   ordering ruled load-bearing is untouched. Shape 4 is not disturbed.
5. **The three provably unreachable checks are not touched in either direction.** Round 6's ruling
   survives, as ruled.

## 3. The three-state model, which is the safety core of this design

**Accumulation without an explicit not-evaluated state is worse than today's short-circuit.** If WAL
replay fails and the WAL-persistence stage simply does not run, a report showing zero WAL-persistence
findings is indistinguishable from one where that stage ran and found nothing. **A hard failure would
have become a silently incomplete, clean-looking report.**

**Every stage therefore resolves to exactly one of three states:**

| State | Meaning |
|---|---|
| `Evaluated` | The stage ran to completion; its findings are authoritative |
| `Failed` | The stage errored; the error is recorded as a blocking finding against its scope |
| `NotEvaluated` | The stage could not run because a dependency failed; **named, with the dependency named** |

**`NotEvaluated` is blocking.** A repository whose verification is incomplete is not verified.

**No stage may be silently absent from the report.** This is the design's central invariant and the one
an implementation review must check first.

## 4. Stage dependencies

Stages are not independent, and `NotEvaluated` must be derived from real dependencies rather than
assumed:

- `wal.replay()` failing ⇒ `verify_wal_persistence`, `verify_rollback_draft_wal_records`, the per-record
  schema loop, and `check_active_wal_ordering` are all `NotEvaluated`.
- `verify_refs` failing ⇒ the per-envelope schema/trust loop and publication reclassification are
  `NotEvaluated`.
- `verify_objects` failing ⇒ nothing else is blocked; it feeds `signature_issues` and
  `merge_baseline_divergences` only.

**Deriving this graph from the code, not from this list, is an implementation obligation.** My list is
read off the current pipeline and is exactly the kind of framing that has proved narrower than the code
three times this cycle.

## 5. Severity, `doctor`, and repair

- **One new finding type**, carrying scope, code, message, and a `blocking` flag — mirroring
  `RefPublicationIssue`, which `doctor.rs:257-261` already handles correctly.
- **`doctor` derives severity from the flag.** No unconditional `warning` mapping for the new type; that
  shape (`signature_envelope_issues`, `object_temp_paths`) is what would let repair proceed against
  damage it previously refused.
- **`repair_repository`'s gate is unchanged and must stay unchanged.** Its protection now comes from
  contained errors being blocking, not from `verify_repository` returning `Err`.

## 6. Output contract — a deliberate, user-visible change

Today an `Err` prints one line and `print_verify_report` never runs (`main.rs:517-518`). After Level 1
it always runs, so a structurally-broken repository produces the full report plus its findings.

**This is a real behaviour change and is accepted deliberately, not absorbed.** It must be stated in
release notes. `prikk doctor` gains the same improvement independently — its `Err` arm
(`doctor.rs:270-276`) currently discards every count and category.

`verify_repository`'s signature still returns `Result`: **failures that make the repository
uninterpretable as a whole** — layout invalid, `FORMAT` unreadable — stay whole-run fatal. Containment
applies to stages, not to the preconditions for having stages.

## 7. Cost

Unbounded growth is concentrated entirely in `verify_objects`'s object-store scan; WAL/rollback is
bounded by DC-57's active-patch limit and refs by ref count.

**Decision: accept full accumulation as the default, and add an explicit opt-out rather than a
heuristic.** A `--stop-on-first-error` flag preserves today's behaviour for anyone who needs a bounded
walk of a large damaged repository. **No implicit cap**, and no cutoff that makes the report
incomplete without saying so — that would violate §3.

## 8. Test migration

The 27 Stage 1 tests change assertion shape only, and **keep their doc comments** — the reachability
lessons remain true. Per the ruling, the exception is any check sharing a code path with a converted
one; under this design that set is small, because checks are not individually converted.

**New tests required by this design:**

1. **Per stage: a `Failed` stage does not suppress later stages' findings.** Two independent defects in
   different stages, both reported.
2. **Per dependency edge: the dependent stage is `NotEvaluated`, and says which dependency failed.**
3. **Per converted stage: `repair_repository` still refuses** — the ruling's acceptance criterion, and
   the point is the refusal, not the finding's presence.

## 9. Staging

- **Level 1** — stage containment, the three-state model, `doctor`/repair wiring, output contract,
  `--stop-on-first-error`. Independently shippable and delivers most of the value.
- **Level 2** — item containment inside `verify_objects` and `verify_refs`. Separately reviewable.

**Level 1 must merge before Level 2 begins.** Stage 1's lesson about bundling applies within Stage 2.

## 10. What does not change

Stage 1's classification table and its 41 rows; the three unreachable checks; the
`classify_ref_state`→`require_retained_evidence` ordering; `signature_envelope_issues` staying
non-blocking (with the contingency recorded in the prerequisite ruling §5); B′ adoption semantics;
object-trust/ref-authority separation.

## 11. Acceptance criteria

1. **No stage silently absent.** Every stage appears in the report as `Evaluated`, `Failed`, or
   `NotEvaluated`.
2. **`NotEvaluated` is blocking and names its failed dependency.**
3. **`repair_repository` refuses for every defect it refuses today** — proven per stage by test.
4. Two independent defects in different stages are both reported.
5. **The dependency graph is derived from the code**, and §4's list is treated as a hypothesis to check.
6. Stage 1's 641 tests still pass, with assertion shapes changed but no coverage lost.
7. Green three-platform CI.

## 12. Open items the implementation handoff must resolve

1. **Where the new finding type lives**, and whether existing `Vec` fields fold into it or sit alongside
   it. I have deliberately not decided this — it depends on the dependency graph in §4 being confirmed.
2. **Whether `verify_objects`'s existing per-object loop already has a natural item boundary** for
   Level 2, or needs one introduced.
3. **The exact `--stop-on-first-error` surface** — flag name, and whether `doctor` gets it too.

## 13. Independence

**This design is author-reviewed**, which is the standing ceiling and a real gap: §1's central choice —
scope containment over per-check conversion — has had no independent adversary. The compensation is that
its acceptance criteria are stated as falsifiable properties rather than as intentions, and that §4 and
§12.1 are explicitly flagged as my framing rather than as derived fact. **An implementation round that
finds §4's dependency graph wrong is doing its job.**
