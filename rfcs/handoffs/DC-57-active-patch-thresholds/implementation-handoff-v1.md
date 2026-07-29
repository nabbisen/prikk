# DC-57 Active-Patch Thresholds - Handoff

**Cleared to start.** Accepted by the project owner on 2026-07-29, at
`rfcs/accepted/DC-57-ACTIVE-PATCH-THRESHOLDS.md`. Design review's one blocking finding — the RFC required
configurable thresholds with no mechanism to configure them — was resolved by owner decision on
2026-07-29: **environment variables**. No gate remains.
**Authored by** the architect.
**Size:** medium. Behavioural feature with CLI surface.

## What this is

Implement a requirement that has been overdue for a milestone and does not exist in any form today:

> **NFR-PERF-02.** Warn at 800 active patches; hard block at 1000 by default unless configured.
> Gate: M3. Evidence: CLI behavior tests.
> — `specs/prikk-non-functional-requirements-v1.1.md` §4.5

**Nothing of this is implemented.** No 800 or 1000 constant exists anywhere in `crates/prikk-store/src/`
or `crates/prikk-cli/src/`; no active-patch warning is emitted. The active-WAL warnings you will find in
`crates/prikk-cli/src/output/verification.rs:93,105,109` are about incomplete trailing records and stale
ref metadata — unrelated conditions, do not extend them.

**Gate label warning:** that "M3" is the **product** milestone (Block DAG and Checkout), not
`MILESTONES.md`'s corrective M3. The product capability shipped long ago, which is why this is a *missed*
gate. See `MILESTONES.md` § "Two milestone schemes" — the collision has already misled one review.

## Step 1 — define "active patches" once, before writing any threshold

**Do this first and get it reviewed before building on it.** It is the main correctness risk in the
increment.

The likely definition is the record count in the active WAL for the target ref — but confirm it against
the authoring and seal paths rather than assuming. Then:

- Compute it in **one function**.
- Make every path use that function.

A threshold that fires at different counts depending on which path reached it is worse than no threshold,
because it implies a bound that does not hold. That is the defect this increment would most plausibly
ship.

Record where the count is computed and why that definition is the right one.

## Step 2 — thresholds and behaviour

| Threshold | Default | Behaviour |
|---|---|---|
| Warn | 800 | Non-fatal. `status` recommends sealing |
| Hard block | 1000 | Operation fails with an actionable error naming `seal` as the remedy |

**The hard block must fail closed and early** — before any WAL append or object write — so a blocked
commit leaves no partial state.

Reconcile the warning with `specs/prikk-app-requirements-v1.2.md` §6.3, which already requires: "When
active patch count reaches the warning threshold, status must recommend sealing." Extend that behaviour;
do not design a second, parallel notion of the threshold.

## Step 3 — configuration by environment variable

`PRIKK_ACTIVE_PATCH_WARN` and `PRIKK_ACTIVE_PATCH_LIMIT`, defaulting to 800 and 1000.

**Why environment variables, so you don't try to improve on it:** no repository configuration mechanism
exists. `.prikk/trust/policy.toml` is hand-parsed line by line
(`crates/prikk-store/src/trust.rs:184-187`), and `prikk-store` has **no TOML dependency** — `toml = "1.1"`
lives only in `tools/release-policy`. Adding a parser to `prikk-store` would trip DC-51's placement gate
(`placement.rs:11` allows only `getrandom` and `rustix`) and require a reviewed `ALLOWED_THIRD_PARTY`
amendment — a release-policy control-surface change, for two integers.

Follow the precedent in `crates/prikk-cli/src/main.rs:431-443`, which reads `PRIKK_AUTHOR_KEY_ID` and
`PRIKK_AUTHOR_SEED` and fails closed on malformed values.

**Reject invalid values; never fall back silently.** A mistyped variable falling back to the default would
produce an unbounded repository while appearing configured. Reject: non-numeric, warn above limit, zero.

The setting is per-invocation and not persisted. That satisfies "unless configured" and is a known
limitation, not an oversight — say so if you document it. A durable per-repository policy belongs with a
future general configuration increment.

## Step 4 — coverage

Boundary tests at **799 / 800 / 999 / 1000 / 1001**, across **every** authoring and seal path that can
change the active-patch count. Enumerate those paths first; a threshold enforced on one path and not
another implies a bound that does not hold.

Configuration tests: defaults when unset, each variable overriding independently, and rejection of each
invalid form above.

## Step 5 — confirm the block does not strand a repository

If a repository reaches the hard bound and the block prevents the operation needed to recover, the bound
becomes a trap.

**Verify `seal` remains available at and above the hard bound**, and test it there. A block on committing
must never become a block on sealing.

## Traps

- **Writing the threshold before defining the count.** Step 1 exists for this.
- **Two definitions of "active patches"** reached by different paths.
- **Silent fallback on a bad environment value.** Reject.
- **Blocking `seal` along with `commit`** at the bound.
- **Adding a config file or parser.** The route is decided; anything else needs a DC-51 amendment and its
  own review.
- **Extending the unrelated active-WAL warnings** in `output/verification.rs`.

## Definition of done

One recorded definition of "active patches" with one computation site used by every path; warn at 800 and
hard block at 1000 as defaults; both overridable by their environment variables with invalid values
rejected; the hard block failing closed before any write; `seal` verified available at and above the
bound; boundary tests at 799/800/999/1000/1001 across every enumerated path; `status` reconciled with
app-requirements §6.3; `MILESTONES.md`'s missed-gate row updated to its resolved state.

## Submit with

The diff; the recorded definition of "active patches" and the enumerated paths that use it; boundary and
configuration test results; evidence that `seal` works at and above the bound; test counts per touched
crate before and after; an explicit statement of what did not change; and the full gate set from
`rfcs/EXECUTION-ORDER.md` §6 rule 9 including release-policy `check`, `boundary-check`, and
`reference-check`.
