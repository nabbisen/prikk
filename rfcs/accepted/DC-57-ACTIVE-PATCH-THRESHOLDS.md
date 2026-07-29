# RFC (proposed) - DC-57 Active-Patch Thresholds

**Status.** **Accepted by the project owner on 2026-07-29.** Architect design review v1, performed after
acceptance (`.git-exclude/reviewed/prikk-dc57-dc58-post-acceptance-review-v1.md`), returned one blocking
finding: the RFC required configurable thresholds while no configuration mechanism existed. **Resolved
2026-07-29** by owner decision — environment variables, no new dependency and no DC-51 amendment. See
design §2. Implementation may begin.
**Supersedes.** Item 3 of DC-42 (`rfcs/archive/DC-42-PERFORMANCE-MAINTAINABILITY-GATES.md`).
**Requirement.** **NFR-PERF-02** — `specs/prikk-non-functional-requirements-v1.1.md` §4.5.
**Gate status.** Product **M3** (Block DAG and Checkout). **Missed and carried** — see `MILESTONES.md`
§ "Two milestone schemes"; this is not corrective M3.
**Touches.** Active-session state, the authoring and seal paths, CLI output, and configuration.

## Problem

> **NFR-PERF-02.** Warn at 800 active patches; hard block at 1000 by default unless configured.
> Gate: M3. Evidence: CLI behavior tests.

**Nothing of this exists.** Verified during DC-42 design review v2: no 800 or 1000 threshold constants
appear in `crates/prikk-store/src/` or `crates/prikk-cli/src/`, and no active-patch warning is emitted.
The only active-WAL warnings concern incomplete trailing records and stale ref metadata
(`crates/prikk-cli/src/output/verification.rs:93,105,109`) — unrelated conditions.

The gate is product M3, "Block DAG and Checkout", whose entry in the NFR matrix §5 explicitly lists
"active block limit behavior". That capability shipped — merge evidence, checkout, and snapshot
materialization are all in `rfcs/done/` and released through 0.17.7. **The requirement is therefore
overdue, not upcoming.**

DC-42 review v1 initially concluded the opposite — that this was M3 work being pulled forward and should
be deferred — by resolving the gate label against `MILESTONES.md`'s corrective scheme instead of the
product scheme. That error is corrected in review v2 and is the reason `MILESTONES.md` now carries a
mapping table.

`specs/prikk-app-requirements-v1.2.md` §6.3 supplies a related behaviour already partly present: "When
active patch count reaches the warning threshold, status must recommend sealing." Reconcile with it rather
than designing a second, parallel notion of the threshold.

## Design

### 1. Define the counted quantity precisely, first

"Active patches" must have exactly one definition, and it must be stated before any threshold is written.
The candidate is the record count in the active WAL for the target ref, but the design review must
confirm this against the authoring and seal paths rather than assume it. Ambiguity here produces a
threshold that fires at different counts depending on the path that reached it — the defect this RFC would
most plausibly ship.

Record where the count is computed and make every path use that one function.

### 2. Thresholds and behaviour

- **Warn at 800.** Non-fatal. Per app-requirements §6.3, `status` recommends sealing.
- **Hard block at 1000.** The operation fails with an actionable error naming `seal` as the remedy.
- **Both configurable via environment variables**, with these as defaults:
  `PRIKK_ACTIVE_PATCH_WARN` and `PRIKK_ACTIVE_PATCH_LIMIT`.

**Configuration route, resolved** (closes design review B1; owner decision 2026-07-29). The RFC previously
said "configurable" without a mechanism, and none exists: no repository config file is read anywhere,
`.prikk/trust/policy.toml` is hand-parsed line by line (`crates/prikk-store/src/trust.rs:184-187`), and
`prikk-store` carries **no TOML dependency** — `toml = "1.1"` lives only in `tools/release-policy`.

Environment variables were chosen over the alternatives:

| Route | Why not chosen |
|---|---|
| TOML config file in `prikk-store` | Needs a parser dependency, which `placement.rs:11` rejects — `prikk-store` is allowed only `getrandom` and `rustix`. Would require a DC-51 `ALLOWED_THIRD_PARTY` amendment, a release-policy control-surface change, for a two-integer setting |
| Hand-rolled config file | No new dependency, but establishes a **second** ad-hoc config format alongside `trust/policy.toml` |
| Fixed defaults, no override | Leaves "unless configured" unimplemented and closes NFR-PERF-02 only partially |

Environment variables add no dependency, no file format, and no parser, and they follow the precedent
already set by `PRIKK_AUTHOR_KEY_ID` and `PRIKK_AUTHOR_SEED` (`crates/prikk-cli/src/main.rs:431-443`),
including its fail-closed handling of malformed values.

**Consequence to state plainly:** the setting is per-invocation, not persisted in the repository. That
satisfies "unless configured" but does not give a repository a durable policy. If a persisted threshold is
later wanted, it belongs with a general configuration increment, not here.

The hard block must fail **closed and early** — before any WAL append or object write — so a blocked
commit leaves no partial state.

### 3. Coverage

Boundary tests at **799 / 800 / 999 / 1000 / 1001**, across **every** authoring and seal path that can
change the active-patch count. Enumerate those paths in the design; a threshold enforced on one path and
not another is worse than none, because it implies a bound that does not hold.

Configuration tests must cover: defaults applied when both variables are unset, each override honoured
independently, and **invalid values rejected rather than silently falling back to the default** — a
non-numeric value, a warn threshold above the hard limit, and zero. Silent fallback would let a
mistyped variable produce an unbounded repository while appearing configured.

## Non-goals

- No change to what sealing does, or to block or WAL formats.
- No merge-scope work — NFR-PERF-03 is a separate requirement, not in scope.
- No commit-path performance work — that is DC-56.
- No ELOC or structure work — that is DC-58.

## Risks

**Two definitions of "active patch."** Covered in design item 1; it is the main correctness risk.

**A hard block that strands a repository.** If 1000 is reached and the block prevents the very operation
needed to recover, the bound becomes a trap. The design must confirm that `seal` remains available at and
above the hard bound — a block on committing must not become a block on sealing.

**Configuration as an escape hatch.** "Unless configured" permits raising the bound arbitrarily. That is
the requirement's own wording and is not this RFC's to change, but the design should record that a
configured override is a deliberate act with consequences for merge scope (NFR-PERF-03), not a routine
tuning knob.

## Acceptance criteria

1. "Active patches" has one recorded definition and one computation site, and every path uses it.
2. Warn at 800 and hard-block at 1000 are implemented as defaults, overridable by
   `PRIKK_ACTIVE_PATCH_WARN` and `PRIKK_ACTIVE_PATCH_LIMIT`. No new dependency, no config file, no parser.
3. The hard block fails closed before any WAL append or object write.
4. `seal` is verified to remain available at and above the hard bound.
5. Boundary tests at 799/800/999/1000/1001 across every enumerated authoring and seal path.
6. Configuration tests cover defaults, independent overrides, and rejection of invalid values
   (non-numeric, warn above limit, zero) — rejection, never silent fallback.
7. `status` behaviour reconciles with `specs/prikk-app-requirements-v1.2.md` §6.3.
8. `MILESTONES.md`'s missed-gate row is updated to its resolved state.
9. Full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9, plus test counts before and after per rule 10.

All nine are verifiable from the repository by a reviewer. No criterion here requires trusting the
implementer's report.
