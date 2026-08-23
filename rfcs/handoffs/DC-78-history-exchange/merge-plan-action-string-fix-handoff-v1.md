# `merge-plan`'s stale action strings: implementation handoff

**Base:** current `main` (`d66d024`). **Code, tests, and one documentation row — together.**
**Origin:** escalated by the documentation-currency sweep. The sweep correctly refused to fix it: the
**code** is stale, and the doc quoting it is accurate.

**This is a user-facing defect, not a tidying.** `prikk merge-plan` tells an operator that merge
execution is not implemented. It has been implemented since DC-74.

---

## 1. The defect

`crates/prikk-store/src/merge_evidence/merge_plan.rs`, `action_for_plan_status`:

```rust
"ConfluentSubset" => "review only; merge execution is not implemented",   // :51  STALE
...
_                 => "inspect evidence; merge execution is not implemented",  // :61  STALE
```

**Verified while scoping: `Confluent` is the only outcome `prikk merge` executes on**
(`guide/merge.md:34-39` — every other outcome refuses). So `ConfluentSubset` is precisely the status
where execution *is* available, and it is the one telling the operator it is not.

**The failure it causes:** run `merge-plan`, see `Confluent`, read *"merge execution is not
implemented"*, conclude there is no way to execute — when `prikk merge` would do it on that same
evidence, which the plan just finished computing.

## 2. Every other string in that function is correct — leave them alone

Checked individually, not assumed:

- `BlockedConflict` — *"conflict resolution is not implemented"*: **true**, conflict resolution is
  genuinely absent.
- `BlockedOrderedDependency` — *"execution ordering policy is not implemented"*: **true**, `merge`
  refuses `OrderedDependency`.
- The five remaining arms make no implementation claim at all.

**Only the two arms in §1 change.** A pass that rewords the others would be reintroducing the sweep's
own `STALE`/`CURRENT` discrimination failure at the code level.

## 3. What the new strings must say

**`ConfluentSubset`** must state that execution is available **and name the command**. A reader who
sees only *"review only"* will stop. Suggested:

```
"executable; review the evidence, then run 'prikk merge'"
```

**The `_` fallback** must stop claiming anything about implementation. It is currently unreachable —
all eight statuses are matched — so its text only matters the day someone adds a ninth, and on that day
"merge execution is not implemented" would be a fresh lie. Suggested:

```
"unrecognised plan status; inspect evidence"
```

**Exact wording is yours to improve; the two requirements are not.** Say what you chose.

## 4. The documentation row moves in the same commit

`docs/src/reference/patch-algebra.md:138` quotes the `ConfluentSubset` string verbatim:

> `| Confluent | ConfluentSubset | Review only; merge execution is not implemented. |`

**Right now that row is accurate.** The moment you fix the code it becomes stale — so **the doc row
changes in the same commit as the string, or this increment creates the exact defect it is closing.**

**Only that row.** The `Conflict` and `OrderedDependency` rows quote strings that are still true (§2).

## 5. Tests must change here — and that is the opposite of the last increment

Three assertions pin the old text:

- `crates/prikk-cli/tests/merge_plan.rs:45`
- `crates/prikk-store/src/merge_evidence/tests/merge_plan.rs:35`
- `crates/prikk-store/src/merge_evidence/tests/merge_plan.rs:108`

**Update all three.** The consolidation increment's control was *"every existing test passes
unchanged"*; **do not carry that instinct here.** That refactor changed no behaviour, so a changed test
would have proved it wrong. This one changes user-visible output deliberately, so **a test that did not
change would mean the output did not change.**

**Count them in your report.** Three assertions, three edits — if you find a fourth, say so; if one of
the three turns out not to assert this string, say that too.

## 6. Out of scope

- **The other six action strings** (§2).
- **`action_for_plan_status` taking `&str` rather than the outcome enum.** Worth recording: a
  string-keyed match with a catch-all is *why* a stale arm could sit unnoticed behind a passing test —
  an enum would have made the fallback unreachable by construction. **Not this increment**; note it in
  your report if you agree.
- **Any other `merge-plan` behaviour.** Only these strings.

## 7. What to report

1. **The two new strings**, and why you chose that wording.
2. **The three test assertions updated**, or what you found instead.
3. **Confirmation the `patch-algebra.md` row moved in the same commit** (§4) — and that no other row
   did.
4. A **manual check**: run `prikk merge-plan` on a confluent pair and paste the actual `action:` line
   an operator now sees. **This defect existed because nobody read the output**; read it.
5. The **full gate set against the exact commit, after the last edit** — the standard nine, plus
   `mdbook build` for the doc row.
6. Test counts before and after — **expected unchanged**; three assertions edited, no test added or
   removed.
7. Anything here that turned out to be wrong. **Say so plainly.**

**Stop and escalate, do not guess**, if: a fourth site asserts the old string; `prikk merge` turns out
not to execute on `ConfluentSubset` after all, which would mean §1's premise is wrong; or fixing the
string requires touching plan-status logic rather than only its display text.
