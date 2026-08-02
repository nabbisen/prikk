# DC-57 Active-Patch Thresholds - Handoff v2

**Cleared to start.** The hold is lifted. RFC at `rfcs/accepted/DC-57-ACTIVE-PATCH-THRESHOLDS.md`.
**Authored by** the architect.
**Supersedes handoff v1, which was withdrawn on 2026-07-29** and must not be worked from — it was written
when the active WAL was capped at one record, so its premise was unreachable.

**Size:** small. Two integers, two environment variables, one check, and its tests.

## Why the hold is lifted

DC-57 was held because NFR-PERF-02's "warn at 800 / block at 1000 **active patches**" governed a number
that could not exceed 1. **DC-66 fixed that** (complete at `45af36f`): the active session now holds N
unsealed patches. The thresholds are reachable and testable for the first time.

## What I re-verified before issuing this, so you do not have to

The RFC's design was written against the old world. I checked whether it survived:

- **The threshold mechanism is unaffected.** Environment variables (`PRIKK_ACTIVE_PATCH_WARN`,
  `PRIKK_ACTIVE_PATCH_LIMIT`), chosen 2026-07-29 by owner decision over a TOML file (which `prikk-store`
  cannot take a parser dependency for, per DC-51's placement gate) and over a second ad-hoc config format.
  None of that reasoning depended on the cap. **It stands unchanged.**
- **The count now has an obvious source**: the active WAL's record count. Each queued commit appends one
  patch record, so records = active patches. Confirm that reading yourself against DC-66's code before
  relying on it — that is the kind of assumption this program keeps catching.
- **`status` already reports queue health.** DC-66 added it. DC-57's "at 800, `status` recommends sealing"
  therefore **extends DC-66's existing output rather than inventing a surface.** Do not add a second
  reporting path.

## What the RFC requires that is easy to get wrong

**Fail closed and early.** The hard block must fire **before any WAL append or object write**, so a
blocked commit leaves no partial state. Roughly where DC-66's removed guard used to sit.

**Malformed configuration fails closed**, following the precedent of `PRIKK_AUTHOR_KEY_ID` /
`PRIKK_AUTHOR_SEED` (`main.rs:431-443`). A garbage `PRIKK_ACTIVE_PATCH_LIMIT` must not silently fall back
to the default — that would make a safety threshold silently absent.

**The setting is per-invocation, not persisted.** The RFC states this plainly and it is deliberate: it
satisfies "unless configured" without giving a repository durable policy. A persisted threshold belongs to
a future general configuration increment. **Do not invent a config file here.**

## Traps

- **Working from handoff v1.** Withdrawn; its premise was unreachable.
- **Adding a second `status` surface** instead of extending DC-66's.
- **Defaulting on malformed input** instead of failing closed.
- **A config file.** Explicitly out of scope; it would need a DC-51 `ALLOWED_THIRD_PARTY` amendment for a
  two-integer setting.
- **Blocking after a partial write.** The check is early or it is wrong.
- **Testing only at the boundary.** Test 799/800/801 and 999/1000/1001 behaviour, and that a blocked
  commit left nothing behind.

## Definition of done

Warn at the configured warn threshold via DC-66's existing `status` output; hard block at the configured
limit with an actionable error naming `seal`; both configurable by environment variable with 800/1000
defaults; malformed values fail closed; the block fires before any append or write, proven by asserting no
state changed; boundary tests either side of both thresholds; full gate set per `rfcs/EXECUTION-ORDER.md`
§6 rule 9 with test counts before and after.

## Submit with

The diff; boundary tests; a test proving a blocked commit leaves no partial state; the malformed-value
tests; test counts per touched crate before and after; an explicit statement of what did not change; and
the full gate set on a **clean checkout**, with commands reported **verbatim** (`--locked`, `--no-fetch`,
`+1.85.0`).

## Standing request

This RFC is the one that was **held for four days because its premise did not hold** — the dev team read
the code at Step 1, found the WAL capped at one record, and stopped instead of building against it. That
report is why DC-66 exists. If something here contradicts what the code actually does, stop and report it.
