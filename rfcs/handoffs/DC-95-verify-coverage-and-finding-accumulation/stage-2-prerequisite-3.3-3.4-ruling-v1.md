# DC-95 Stage 2 §3.3–§3.4 — Prerequisite Ruling v1

**Reviewing:** `.git-exclude/review-request/prikk-dc-95-stage2-prerequisite-3.3-3.4-v1.md`.

**Accepted. No stop-and-report.** Stage 2 may proceed to design. Four rulings follow, one of which is a
safety constraint and one of which corrects the RFC's own framing.

## 1. Verified

- **`repair_repository`'s gate.** `doctor.rs:299` refuses on `!before.is_healthy()`; `is_healthy()`
  (`:110-115`) inspects only `DoctorSeverity::Error`. Confirmed.
- **The severity wiring is non-uniform exactly as described.** `ref_publication_issues` derives severity
  from `issue.blocking` (`doctor.rs:257-261`); `signature_envelope_issues` and `object_temp_paths` push
  `DoctorIssue::warning` unconditionally. Confirmed.
- **The output contract.** `main.rs:517-518` — `verify_repository(&layout).map_err(…)?` precedes
  `print_verify_report`, so an `Err` prints **nothing** but the one message. Confirmed.
- **`doctor`'s `Err` arm** (`doctor.rs:270-276`) sets `verification: None` and emits one generic issue,
  discarding every count and category. Confirmed.

Every claim I checked held, and each was cited to file:line rather than to a function's name.

## 2. §3.4.3 is a safety finding, and it becomes a hard constraint

Today **every** hard-`Err` refuses repair for free: it becomes one `DoctorSeverity::Error`, and
`repair_repository` declines. Convert a check to accumulate and that protection depends entirely on the
new entry's severity mapping — where two existing fields default to `warning` **because their element
types carry no severity signal at all.**

**A new accumulated type shaped like `signature_envelope_issues` would let `repair_repository` proceed
against damage it previously refused, silently, with every gate green.** That is a safety gate weakening
itself as a side effect of a refactor, which is the precise failure class DC-95 exists to prevent.

**Ruled, binding on Stage 2's design:**

1. **Every newly-accumulated type carries its own blocking-equivalent field from the start**, mirroring
   `RefPublicationIssue`. No new field may rely on a per-field severity decision in `doctor.rs`.
2. **Stage 2's acceptance criteria gain one item:** for each converted check, a test proving
   `repair_repository` still refuses. Not "the issue is present" — *the repair is still refused.*

Their own recommendation was (1). **(2) is mine, and it exists because (1) is a rule someone can forget
while a test cannot.**

## 3. The RFC's framing was wrong, and this is the third time

DC-95 §2 frames the boundary as two shapes, accumulate versus hard-error. **There are four.** Shape 3
(`classify_active_wal_metadata` returning one enum partitioned by two predicates) and shape 4
(`require_retained_evidence` rewriting entries **in place** after they are pushed) are already
report-shaped and need no conversion.

**Shape 4 is a Stage 2 hazard specifically.** Reclassification runs *after* the pushes it rewrites, so
any reordering of `verify_repository`'s pipeline can separate a push from the pass that corrects it —
and the result is a *wrong but plausible* issue code, not a missing one. **No Stage 1 test would catch
that**, because the defect is still reported.

**Ruled: pipeline order between `classify_ref_state` and `require_retained_evidence` is load-bearing and
may not be changed by Stage 2 without an explicit test for the reclassification.**

For the record: this is the third prerequisite investigation to correct a document I wrote — RFC 101's
§1 problem statement, RFC 102's §3 ambiguity, and now DC-95 §2's shape count. The pattern is stable
enough to name: **my framings are reliably narrower than the code.**

## 4. The boundary verdict, and the scope narrowing it forces

*"Incidental in its current instantiation, principled in its origin"* is a better answer than the binary
I asked for, and the evidence is specific: the ref and object clusters split structural/uninterpretable
(hard-`Err`) from semantic/still-trustworthy (accumulate); **`wal.rs` and `rollback_verify.rs` do not
follow it at all.** A wrong-length AUTHOR signature is as "real but wrong about one claim" as a false
merge baseline, yet one aborts the run and the other accumulates.

**Ruled:**

- **The principle is adopted as Stage 2's design rule.** Structural or uninterpretable defects stay
  hard-`Err`; semantic defects on an otherwise-trustworthy object accumulate.
- **"Convert every hard-`Err`" is explicitly not the goal**, and Stage 2's design must justify each
  conversion against that rule rather than sweep the population. This narrows Stage 2 materially, and it
  is the right narrowing: accumulating *"this Block's schema is unparseable"* alongside *"this Block's
  baseline claim is false"* pushes a severity range onto every consumer that `blocking: bool` does not
  express.
- **The three provably unreachable checks are not conversion candidates in either direction.** Round 6's
  ruling — keep, no test, record the argument — survives Stage 2 unchanged.

## 5. `signature_envelope_issues`: their recommendation accepted, with the contingency recorded

**Keep it non-blocking**, for the reason they give and not the reason I would have accepted: the field
conflates *malformed shape* with *format-1 legacy-but-valid*, and no predicate can be principled until
those are separated. Splitting them is design work, correctly refused here.

**But the contingency is now explicit and must travel with the design:** if Stage 2 performs the split,
the `MALFORMED` half should back a blocking predicate, **four Stage 1 exclusions reopen, and two
"load-bearing via non-blocking-sibling" rows reclassify.** That is a Stage 1 inventory change caused by
a Stage 2 decision, and it must be made deliberately, with tests written before the behaviour changes —
never as a consequence noticed afterwards.

## 6. Cost, and the 27 tests

**Cost:** their refusal to invent a wall-clock number they were not authorized to measure is correct.
The shape is the answer: unbounded growth is **concentrated entirely in `verify_objects`'s object-store
scan**; WAL/rollback is bounded by DC-57's active-patch limit and refs by ref count. **Stage 2's design
must state what happens to a large, deeply-damaged repository** — whether full accumulation is accepted,
or bounded by some explicit mechanism. Naming it is required; choosing is design work.

**The 27 tests: keep their doc comments.** Agreed, and the reasoning is right — the *reachability*
lessons (round 10's reclassification, round 11's `validate_read_schema` interception, round 12's
horizon anchoring) remain true when only the assertion shape changes. Re-deriving load-bearing status
for all 27 would be waste. **The exception they identify is the correct one:** any check whose code path
is shared with a converted check may have had its reachability changed, and those must be re-probed.
Stage 2's design should name that subset explicitly rather than leave it to judgment during the
refactor.

## 7. Standing

- **§3.3, §3.4: accepted.** DC-95 §3's four prerequisites are now fully discharged.
- **Stage 2 design is cleared**, bound by §2's two constraints, §3's ordering rule, §4's scope rule, and
  §5's contingency.
- **Stage 2 must not be bundled with Stage 1's record.** Separately reviewable, per the RFC.
- Green three-platform CI before any merge, unchanged.
