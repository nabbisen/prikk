# RFC 111 — Object read cost regression: every object read is O(N)

**Status.** **Proposed.** Found 2026-08-18 by the architect while measuring whether badge criterion 3 was
still open. **Not a design; a measured defect with a located cause.** Independence: author-reviewed, the
standing ceiling — the measurements in §2-§4 are reproducible from the named harness and commits, which
is what compensates for it here.

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

## 6. What a design must decide

1. **Positional reads.** Decode at `entry.offset` without materializing the container — the thing
   `object_store.rs:80` already claims. Whether the anchored-fd primitive layer exposes a positional read
   on all three platforms (Windows has no `pread`; it has `ReadFile` with `OVERLAPPED`) is the first
   question, and it lands on DC-87/DC-96's authority split.
2. **Index snapshot scope.** One replay per *operation*, not per read. What object owns that snapshot, and
   what invalidates it, is a correctness question — a stale snapshot inside a writing operation is the
   failure mode, and it must fail closed.
3. **Whether a cost gate exists at all.** This regression passed every gate, three releases, and multiple
   implementation reviews. A curve that silently changes shape is exactly what rule 10's test-count
   discipline does for tests. **A benchmark that runs only when someone remembers to run it did not
   catch a 5.8× regression for seven days.**

## 7. Non-goals

- **Not a redesign of RFC 102's container model.** Containers are correct; reading them whole is not.
- **Not a caching layer.** §6.1 removes the need to read the whole file rather than remembering it.
- **No identity-bearing byte change**, no format bump: this is a read-path cost defect only.
