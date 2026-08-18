# RFC 111 — Object read cost regression: every object read is O(N)

**Status.** **COMPLETE, merged 2026-08-18.** Stage 1 at `13f7a4b`, Stage 2 at `ffaab08`, both after green
CI. **ACCEPTED by the project owner 2026-08-18** ("RFC 111 is accepted. Proceed."). Found
2026-08-18 by the architect while measuring whether badge criterion 3 was still open. **§6.3 — whether a
cost gate should exist — RULED 2026-08-18 by the owner: "Yes, add the cost gate." §7 states its shape and
sequencing. **Delivered as two gates**: `rfc111_index_decode_cost_gate.rs` (`verify`) and
`rfc111_seal_decode_cost_gate.rs` (`seal`), each written before its fix and each observed failing first.**

**Outcome, measured.** `verify`: 167.85 ms → **27.04 ms** at N=160, tail ratio 3.51 → **1.97**. `seal`, on
the repository's own disk: 93.86 ms → **46.96 ms** at N=160, neutral at small N, ratios flat. Both are now
held by a decode-count gate rather than by a one-time measurement.

**Not a design; a measured defect with a located cause.** Independence: author-reviewed, the standing
ceiling — the measurements in §2-§5.1 are reproducible from the named harness and commits, which is what
compensates for it here.

**Arises from.** The owner's 2026-08-18 direction to take criterion 3 next. The investigation found the
criterion's board row was stale, and then found something the row did not describe.

## 1. The finding

**`FileObjectStore::read_object` is O(N) in repository size, so any operation reading O(N) objects is
O(N²).** `verify` is such an operation.

Two full-file reads happen on every single object read:

- **`object_store.rs:77`** → `lookup_object_location` → **`index.rs`'s `replay_index(layout)`**: reads and
  decodes the **entire object index**, then linear-scans it for one id. Per call.
- **`object_store.rs:82`** → `read_object_envelope_at` → **`read_file_if_exists`**: reads the **entire
  container file** into memory, then decodes the one record at `entry.offset`. Per call.

**The code states the opposite of what it does.** `object_store.rs:80` reads:

> *"One seek (design §12/§10.3): the index already named exactly where this object is, so this decodes
> directly at that offset rather than scanning the container from the start."*

The **decode** is at an offset. The **read** is the whole file. The comment describes the design intent
and the implementation defeats it, which is why the cost survived review — the line that would have
raised the question asserts the question is already answered.

## 2. Measured, with a like-for-like baseline

Same harness (`dc92_lineage_replay_benchmark.rs`), same tree size (10 files, churned), **release builds
throughout**, `verify` median ms against sealed-block count:

| N | DC-92 merge `b718623` | current `main` `2f0f5f6` |
|---:|---:|---:|
| 5 | 1.72 | 1.98 |
| 10 | 2.34 (×1.36) | 3.07 (×1.55) |
| 20 | 4.11 (×1.76) | 6.67 (×2.17) |
| 40 | 7.40 (×1.80) | 16.86 (×2.53) |
| 80 | 14.63 (×1.98) | 47.82 (×2.84) |
| 160 | 28.88 (×1.97) | **167.85 (×3.51)** |

**`b718623` settles at ×1.97 — linear, exactly as DC-92 claimed.** Current `main` climbs to ×3.51.

**A constant floor cannot explain this.** For `c + aN` with `c > 0`, the doubling ratio is always *below*
2 and rises toward it. Observing 3.51 rules out linear-plus-constant. Fitted tail exponent ≈ **1.8**.

**5.8× slower at N=160, and the gap widens with N.**

## 3. Phase attribution

Instrumented `verify` on kept repositories, same binary:

| Phase | N=40 | N=160 | growth for N×4 |
|---|---:|---:|---:|
| `replay_index` (once) | 0.070 ms | 0.167 ms | ×2.4 |
| index cross-check loop | 0.559 ms | 2.441 ms | ×4.4 |
| **container scan** | 7.157 ms | **57.580 ms** | **×8.0** |
| temp debris | 0.010 ms | 0.011 ms | ×1.1 |
| **topological (DC-92's memoized pass)** | 3.461 ms | **44.651 ms** | **×12.9** |

**Both dominant phases are superlinear, including DC-92's own memoized lineage pass** — which is O(N) in
block count by construction. It is superlinear here because the memo eliminated redundant *replays* while
each surviving object *read* underneath it became O(N).

## 4. Causation established by control, after a first hypothesis was refuted

**Refuted first:** the index cross-check loop in `verify/objects.rs` also calls
`read_object_envelope_at` per entry. Hoisting those reads changed N=160 from 167.85 ms to **159.35 ms** —
essentially nothing. **Reading the code identified a real full-file read that was not the expensive one**;
the phase table then showed why (2.4 ms of 160 ms).

**Confirmed:** memoizing both full-file reads for the process lifetime:

| | N=40 | N=160 | exponent |
|---|---:|---:|---:|
| current `main` | 16.86 ms | 167.85 ms | 1.66 |
| both reads memoized | **7.73 ms** | **27.28 ms** | **1.06** |
| `b718623` baseline | — | 28.88 ms | 1.00 |

**27.28 ms restores the DC-92 baseline and the curve goes flat.** The two reads are the whole regression.

**The memo is a probe, not the fix** — a process-lifetime cache is unsound for any path that writes.

## 5. Scope — this is not a `verify` defect

`read_object` is the product's object-read primitive. **Every** operation that reads objects pays this:
`commit`, `seal`, `checkout`, `merge`, `bundle`, `history`. `verify` is where it was measured because
criterion 3 gave a harness; it is not where the cost is confined.

**Consequences to state rather than discover:** DC-59/DC-64/DC-69's performance evidence predates the
container migration on this path, and **NFR-PERF-01's warm-path claims should be re-measured**, not
assumed to carry.

## 5.1 Which of the two reads costs — measured, because the obvious answer was wrong

The two O(N) reads were memoized **separately**, same repository at N=160, 7 runs each, median:

| | N=160 | share of the regression |
|---|---:|---:|
| baseline | 164.90 ms | — |
| **container full-read** memoized | 156.30 ms | **~5%** |
| **index replay** memoized | **29.20 ms** | **~82%** |
| both memoized | 27.28 ms | 100% |

**The index replay is essentially the whole regression; the container read is nearly free.** The reason is
that a repeated full read of the same file is served from the OS page cache and costs a copy, while
`replay_index` **decodes and validates every entry on every lookup** — CPU work no cache absorbs.

**This inverts the design ordering below.** Positional reads — the fix suggested by reading
`object_store.rs:80`'s claim — address the 5%. **The 82% is a decode, not a read**, and no I/O change
touches it. Recorded because the plausible fix and the effective fix are different ones here.

## 6. What a design must decide

1. **Eliminate the per-lookup index decode.** This is the whole increment. One decoded snapshot per
   *operation*, not per read. **What owns that snapshot and what invalidates it is a correctness
   question** — a stale snapshot inside a writing operation is the failure mode, and it must fail closed.
   The probe used a process-lifetime thread-local, which is exactly the unsound version.
2. **Positional reads, second and optional.** Decode at `entry.offset` without materializing the
   container — the thing `object_store.rs:80` already claims. **Feasible on all three platforms with safe
   std APIs** (`pread` via the existing `rustix` fd on POSIX, `seek_read` on Windows, read-and-slice for
   the `PathOnlyReader` fallback), so DC-90's `forbid(unsafe_code)` boundary is untouched and no new
   dependency is needed. `AnchoredReader` already anticipates extension — its doc describes adding a
   platform impl "with no change to the four public functions below." **Worth ~5%; do it only if it falls
   out of §6.1's work, and not before.**
3. **A cost gate. Ruled in by the owner 2026-08-18** — see §7, which is now part of this increment
   rather than an open question. This regression passed every gate, three releases, and multiple
   implementation reviews. **A benchmark that runs only when someone remembers to run it did not catch a
   5.8× regression for seven days.**

## 7. The cost gate — count operations, not milliseconds

**Ruled by the owner 2026-08-18.** The shape below is the architect's, and the reasoning matters more than
the mechanism.

### 7.1 Why a wall-clock threshold is the wrong gate

CI hardware varies run to run, so an absolute-millisecond threshold is either loose enough to miss a
regression or tight enough to fail spuriously — and **a flaky gate gets muted, which is worse than no
gate.** This project has the evidence in hand: the same `verify` at N=160 measured **2534 ms** in debug and
**28.88 ms** in release on identical code. A number that moves 88× with a build flag is not a threshold.

**What actually regressed is the shape**, and the mechanism behind the shape is countable: *the number of
full index decodes performed while verifying a repository of N objects*. Today it is proportional to N. It
should be bounded.

### 7.2 The gate

**Assert that a `verify` of a repository performs a number of full index decodes that does not grow with
repository size** — measured at two sizes (e.g. N=20 and N=80) and required to be equal, or bounded by a
small stated constant, rather than proportional.

Properties that make this the right instrument:

- **Deterministic.** No timing, no hardware dependence, no flake. It fails for a comprehensible reason and
  names the mechanism, not a number.
- **Fast.** Milliseconds, so it belongs in the ordinary suite rather than a nightly job nobody watches.
- **It fails for the right reason.** A future change that reintroduces per-read decoding trips it even if
  the machine is fast enough to hide the cost.

Counting infrastructure has precedent: `fsutil/anchored/failpoints.rs` already counts matching calls for
`fail_after(point, matching_calls_to_skip)`. A test-only counter in that thread-local shape is an
extension of an existing pattern, not a new facility.

**If a timing check is also wanted**, gate the **doubling ratio**, never absolute milliseconds — a ratio is
self-normalising against hardware speed, and it is the quantity that actually moved here (1.97 → 3.51).
The existing `dc92_lineage_replay_benchmark.rs` stays a manual instrument either way.

### 7.3 Sequencing — the gate lands first, and must fail

**Write the gate before the fix, on current `main`, where it is required to FAIL.** Then §6.1 turns it
green.

DC-90 already made half of this argument — *"a boundary added afterwards documents what happened instead
of constraining it."* Here the argument is stronger, because a live regression is available: **a gate
written after the fix is a gate nobody has ever seen detect anything.** This one gets its negative control
for free, and the failing run is the evidence that it works.

## 8. Closure — what happens when Stage 2 lands

**Badge criterion 3's `MILESTONES.md` row is updated when Stage 2 merges, not before. Authorized by the
project owner 2026-08-18** ("Update criterion 3's row after Stage 2 lands"), and recorded here rather
than carried in anyone's head — the row has already been stale once, for seven days, and that staleness
is what sent the schedule at a closed problem (see §2's own history).

**What the row must say at that point, honestly:**

- `verify` is linear in history length (Stage 1, merged `13f7a4b`) — 27.04 ms at N=160, tail ratio 1.97,
  against 167.85 ms and ×3.51 before.
- Whether `seal` satisfies the criterion depends on Stage 2's measured curve, **and on the residual recorded in `handoffs/111-object-read-cost-regression/stage-2-handoff-v1.md` §4**:
  `seal` stays O(N) per call even after Stage 2, because `derive_next_state_root` walks the lineage from
  genesis with a deliberately fresh memo. The criterion reads *"`verify` is not superlinear in history
  length"* — so decide explicitly whether it is about `verify` alone, as written, or about the cost of
  using the tool, which is what the claim behind it is really about. **Do not settle that by whichever
  reading happens to let the row be marked met.**

**Also update RFC 111's own status to done and move it out of `proposed/`** when Stage 2 merges; a
regression RFC left in `proposed/` after it is fixed is the same staleness in a different file.

## 9. Non-goals

- **Not a redesign of RFC 102's container model.** Containers are correct; reading them whole is not.
- **Not a caching layer.** §6.1 removes the need to read the whole file rather than remembering it.
- **No identity-bearing byte change**, no format bump: this is a read-path cost defect only.
