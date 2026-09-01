# RFC 130 — A coupling invariant for `prikk-store`, and the gate that holds it

**Status.** **Proposed.** Originated by the project owner 2026-09-01, asking whether `prikk-store`
should be restructured because *"it is continuously growing now and also perhaps in the future as
well"*. Answered by an independent external architect review, reviewed and amended by the project
architect.

**Tracks.** A checked structural property of `prikk-store`. No product behaviour changes. **This RFC
does not move a single file** — that is RFC 131, and this one should land first (§7).

---

## 1. Why the evidence is restated here rather than cited

Both source documents — the external review and the architect's review of it — live under
`.git-exclude/`, which `.gitignore:30` excludes from the repository. **A fresh clone can read
neither.** RFC 120 §4 already names this failure mode for findings recorded only in
`.git-exclude/reviewed/`. So the load-bearing measurements are reproduced below, with the commands
to re-derive them, rather than referenced.

## 2. What was measured

At `04e9391`, over `prikk-store`'s **production** `use crate::<module>` edges (test files excluded),
**68 top-level modules, ~292–297 inter-module edges** (two independent derivations, differing ~2% on
import-form handling and agreeing on every conclusion).

### 2.1 The crate is not a DAG

**There is exactly one cycle: `active → refs → active`.** The active-session/WAL layer and the
ref-publication layer depend on each other. Both derivations found it, and found no other.

### 2.2 Four middle-of-graph hubs

Modules with **both** high fan-in and high fan-out — a change propagates in both directions:

| Module | fan-in | fan-out |
|---|---:|---:|
| `refs` | 16–17 | 12 |
| `patch_replay` | 12 | 6 |
| `wal` | 10–11 | 5 |
| `lifecycle_cache` | 6 | 8 |

### 2.3 The rest of the graph is healthily layered

- **Wide foundation, one-directional**: `layout` (41–42 in / 1 out), `fsutil` (**28 in / 0 out**),
  `object_store` (22/2), `byte_cursor` (13/0). High impact, but it flows one way, upward.
- **Isolated orchestrators**: `verify`, `sync_negotiation`, `patch_exchange`, `worktree_patch`,
  `seal_from_accepted` — high fan-out, near-zero fan-in. **Nothing depends on them, so changing them
  is safe regardless of their size.**
- **12 pure leaves** (zero out-edges).

**So the structure is a layered DAG with a wide one-directional foundation and an isolated
orchestrator top — spoiled by four middle-hubs and one cycle.** Those five things are where
"what breaks if I change this?" is genuinely hard to answer.

### 2.4 What is *not* the problem

- **Size.** 74,687 lines (34,883 of them tests), 123 top-level entries. Growth across 45 releases is
  8.4× in lines, but **three feature releases account for 63% of it** (0.18.0, 0.20.0, 0.23.0) and
  every other release moves a few hundred lines. That is lumpy feature growth, not entropy.
- **Compile time.** Independently measured at 3 s / 2.3 s for the lib from clean, 8 s / 7 s for the
  full workspace test build, ~1 s / 0.5 s incremental. **There is no compile-time problem to solve.**

**Neither line count nor module count is diagnostic, and neither should be gated** (§5).

## 3. The invariant

> **1. The top-level production module graph is acyclic.**
> **2. No module becomes a new middle-hub** — high fan-in *and* high fan-out — without a recorded
> reason.

Both halves are computable from one scan of production `use crate::` edges.

## 4. The gate, and the amendment that makes it correct

The external review proposed gating both halves with a bare degree bound (fan-in ≥ K **and**
fan-out ≥ K, suggesting K = 8). **A bare bound is wrong, and this project has the counter-example in
its own history from the same day.**

**RFC 122 (`7a01168`) moved two hubs.** Diffing the graph across that increment:

```
added:  patch_replay -> node_lifecycle      removed: worktree_patch -> lifecycle_cache
        patch_replay -> wal                          worktree_status -> checkout
        worktree_status -> {blob_access, lifecycle_cache,      worktree_status -> snapshot
                            node_lifecycle, patch_replay, wal}
```

`patch_replay` went **12 in / 6 out → 13 in / 8 out**; `wal` went 10/5 → 12/5; `worktree_patch` — an
orchestrator nothing depends on — lost one.

**At K = 8, `patch_replay` at 13/8 violates the bound. So the proposed gate would have rejected
RFC 122** — a correct fix for a High-severity defect, found by an external audit and required by the
architect.

**The reason is structural, not incidental: consolidation and hub-reduction pull in opposite
directions.** RFC 122's whole purpose was to replace two duplicate baseline derivations with one
shared function; sharing a derivation necessarily concentrates edges at the shared site. A degree
bound cannot distinguish **accretion** (a module acquiring unrelated reach) from **consolidation** (a
module absorbing a derivation that previously existed twice) — and the second is behaviour this
project should reward.

### 4.1 The shape the gate must take instead

**An allowlist whose entries carry reasons — the idiom this project already owns twice**:
`DECLARED_UNDOCUMENTED` (`crates/prikk-cli/src/commands/tests.rs`) and
`RFC114_ADMITTED_BUT_UNWRITTEN`. Both exist so that the gate's honesty lives in the requirement that
every entry state a real reason rather than a placeholder.

- **Acyclicity: absolute after grandfathering.** `active ↔ refs` is declared with its reason; **a
  second cycle fails the build, full stop.** No amendment needed here — the external review's version
  is right.
- **Middle-hubs: declared, not bounded.** Today's four are declared with reasons. A module newly
  crossing the threshold **fails until someone adds an entry saying why it is consolidation and not
  sprawl.** The gate forces a recorded decision; it does not adjudicate one.

### 4.2 Open design questions for the implementing increment

1. **The threshold.** K = 8 is a starting proposal, not a ruling. It should be chosen so today's four
   are the declared set and the next accretion trips it — derived from the measured distribution,
   with the derivation written down.
2. **Where it runs.** `boundary-check` is the natural home; it already runs under `cargo test`.
3. **Edge extraction is the load-bearing detail.** Two independent implementations differed ~2% on
   import-form handling (`use crate::{a, b}` grouping, re-exports, `crate::x::y` paths). The gate's
   own definition of an edge must be stated and tested, or the allowlist drifts against it.
4. **Test files must be excluded**, and that exclusion must be part of the tested definition —
   `fsutil`'s only outward edges are test-only, and a gate that counted them would report the
   cleanest module in the crate as coupled.

## 5. What must not be gated

**Line count and module count.** They are the numbers that prompted this work and the least
diagnostic of everything measured (§2.4). Gating them would institutionalise watching the wrong
thing, and would fire on healthy feature growth while a second cycle formed unwatched.

## 6. Explicitly ruled out: splitting `prikk-store` into crates

**Recorded here because the decision was taken on evidence and must survive the git-excluded reviews.**

`fsutil` is the crate's one genuinely clean seam — **0 production out-edges, 28 in-edges**, a trait
contract with per-platform implementations and a conformance suite. **And extracting it is still not
worth doing**, because its production code is *already* decoupled: a crate boundary would remove no
coupling while adding a ninth crate to the fixed per-release publish order, forcing the fixed-size
`[(&str, &str); 8]` registries in `boundary.rs` and `placement.rs` to grow, and converting 28 files'
worth of `pub(crate)` to `pub`.

`patch_algebra` is **not** a clean seam (6 production out-edges into `node_lifecycle`,
`patch_replay`, `path`, `text_span`, `lifecycle_cache`, `object_store`) — correcting the external
audit of 2026-08-31, which had listed it beside `fsutil`. `refs` is the worst candidate: 16 in, 12
out, and it is in the cycle.

**The comparative argument.** gitoxide fragmented early into ~30 crates for two reasons — reuse as a
product, and compile parallelism at scale. **Neither applies here**: `prikk-store`'s internals are
`pub(crate)`, pre-1.0, and the CLI is their only consumer, and there is no compile-time problem.
Fragmenting without those motives imports their costs and none of their benefits.

**Revisit triggers, none of which is a line count:** a leaf gains a production edge back into the
crate (e.g. `fsutil` stops being extractable); a new cycle or a new middle-hub survives review; or
incremental rebuild after a leaf change exceeds ~5 s (10× today).

## 7. Why this is separate from RFC 131, and lands first

RFC 131 (module grouping and `pub(in ...)` visibility) is the larger change and the one that touches
files. **This RFC touches none.** Two consequences:

- **Grouping without the invariant is the failure the external review named**: work that looks like
  progress on side-effect predictability while changing nothing about it. A gate that lands first
  makes the property checked before the layout moves.
- **They collide differently with work in flight.** RFC 131 moves files across the modules RFC 123
  (schema-3 authoring) and RFC 125 (decoder hardening) are editing. **This RFC adds one check and
  conflicts with nothing**, so it can land during band 1 while RFC 131 cannot.

## 8. Outstanding

The external architect offered **the coupling-gate script and the full 68-node edge list**. Taking
that offer is cheaper than rebuilding both, and their extraction is the one the measurements in §2
came from.

## 9. Non-goals

No file moves. No visibility changes. No crate split. No change to any product behaviour, object
format, or gate other than adding this one.
