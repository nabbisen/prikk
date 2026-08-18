# RFC 111 Stage 2 — handoff v1

**RFC:** `rfcs/proposed/111-object-read-cost-regression.md` (ACCEPTED 2026-08-18)
**Stage 1:** merged at `13f7a4b`. `verify` is linear again — 27.04 ms at N=160 against 167.85 ms before,
tail ratio 1.97.
**Scope:** migrate the writing and mixed entry points to `ObjectWriteSession`. §6.2 (positional reads)
remains held back.

Stage 1 fixed the read path. **`seal` — the highest-frequency write in the system — got nothing from it**
and is still O(N²): it walks the whole ancestor lineage once per call, and until this stage each of those
reads costs a full index decode.

## 1. Stage 2 needs its own gate, and it goes first

Stage 0's gate counts index decodes during **`verify`**. It says nothing about `seal`, so Stage 2 would
otherwise land with no regression protection at all — the exact situation RFC 111 exists because of.

**Build a second gate before the fix, and it must FAIL on current `main`.** Same discipline, same reason
(§7.3): a gate written after the fix is a gate nobody has seen detect anything. If it passes before the
fix, it is measuring the wrong thing — report that, do not tighten it until it fails.

**Same instrument as Stage 0**: assert the full-index-decode count for a seal does not grow with
repository size, two sizes, equality rather than a constant. **Not wall-clock** — `seal` is dominated by
`fsync` (~19 ms per call in DC-92's own numbers), which would swamp any timing threshold.

**One question to answer with it, not to assume:** `seal` lives in `crates/prikk-cli/src/seal.rs`, so a
`prikk-store` unit test cannot call it. Say what you gate — the CLI path through an integration test, or
the store-level write path that `seal` composes — and why that choice actually covers `seal` rather than
something adjacent to it. **A gate on a path `seal` does not take is worse than no gate**, because it
reads as coverage.

## 2. The ordering requirement, restated because it is the one that cannot slip

**`refs/publication.rs:70` constructs its own `FileObjectStore` and writes the RefState through it**,
inside an operation reached from `seal`, authoring, and recovery. Thread it as `&mut impl ObjectWriter`
**at or before the first writer migration, never after.**

Stage 1 was safe from this only because no production caller held an `ObjectWriteSession`. That stops
being true with your first migrated writer. `ensure_current` will catch the nested write and stay
correct — that is the guarantee working — but a migration that lands first and is made tidy afterwards
documents the hazard instead of preventing it, which this project has now ruled twice (DC-90, RFC 111
§7.3).

## 3. Measure `seal`'s curve, before and after

The harness already records **every** seal's duration (`seal_by_depth`), not only checkpoints, so this
costs nothing extra. Report the curve on current `main` and after the fix, release builds, same tree
size.

**DC-92's recorded baseline, for shape only — it is a debug build and not comparable in absolute terms:**
19.45 ms at N=5 rising to 53.22 ms at N=160.

## 4. What Stage 2 will not fix, and must not be reported as fixing

**`seal` will still be O(N) per call after this stage.** `derive_next_state_root` walks the ancestor
lineage from genesis with a **fresh `LineageStateMemo` per call**, deliberately — `block_state.rs`'s own
doc explains why, and reusing a memo across separate seal invocations would be wrong. RFC 111 removes the
O(N) cost *of each read*; it does not remove the O(N) *reads*.

So building N commits remains O(N²) in total reads even after this stage lands. **That is a real residual
and it belongs to nobody yet** — not RFC 111's to fix, and not something to discover after the fact.
State it in the Stage 2 report so it enters the record as a known, owned-by-no-one cost rather than an
unpleasant surprise the next time someone measures.

## 5. Carried from Stage 1's review

`replay_index_tail_with_extent` reads the **whole** index file before decoding only the tail. On the read
path that is once per operation and invisible. **On the write path it is once per write**, so a writer
performing M writes does M full-file reads. Those are page-cache copies — the ~5% class, not the 82%
class — so it is probably fine. **Measure it; do not assume it.** If it shows, reading only
`[known_length, EOF)` is the targeted fix and reuses §6.2's positional-read primitive.

## 6. Constraints — unchanged

- No format change, no identity-bearing byte change, no format bump.
- `forbid(unsafe_code)` holds.
- No new dependency without the workspace-dependency convention.
- **§6.2 stays held back** unless it falls out for free, in which case report it and stop.

## 7. What to report

The failing gate first (§1), then the design for the writer migrations, then implementation. **Report
before implementing**, as always — and if §1's gate question turns out to have no good answer, that is a
finding worth stopping on, not something to route around.
