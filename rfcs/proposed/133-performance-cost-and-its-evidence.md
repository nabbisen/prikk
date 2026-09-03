# RFC 133 — What performance costs this project has, and what evidence holds them

**Status.** **ACCEPTED by the project owner 2026-09-03**, as the extraction they instructed: the
measurement concern is *"an independent subject or theme"*, not a verification-culture gate.

**Accepting this RFC did not rule §6, and §6 has since been rewritten.** Its first draft asked
"does peak RSS get standing protection, and in what shape?" — **the wrong question**, as the owner's
challenge established: most of it was already settled by the 2026-07-30 steady-state ruling and by
DC-86's exchange limits. **§6 now asks the one thing genuinely outstanding: whether memory
independence from repository size should be a stated requirement at all.**

**§7 holds: no increment is handed over from this RFC until §6 is ruled**, because the ruling decides
whether a fix must arrive with a standing measurement or without one.

**Tracks.** Cost and the evidence for it. **No behaviour change is proposed here.**

---

## 1. Why this is its own RFC

**The concern was scattered across three documents, and one of its own labels was undefined.**

| Where it lived | What |
|---|---|
| RFC 126 §5/§5a | the standing peak-RSS measurement question |
| `ROADMAP.md` `AUD-01` | `IndexSnapshot::lookup` is a linear scan; `verify`/`seal` do O(objects) lookups |
| `ROADMAP.md` `AUD-02` | `wal.rs` replays the whole WAL on every append — O(N²) over N queued commits |
| `ROADMAP.md`:328 | "the two performance walls", listed Unscheduled |
| nowhere | the commit create-path memory shape measured in §2 below |

**RFC 126 is "gates that do not exist", and all four of its flanks have been reached.** Keeping it
open for a cost question kept a finished body of work nominally unfinished and filed a performance
defect where nobody would look for one.

## 2. The measured memory shape — and a correction to a claim this project has been repeating

**DC-62's problem statement is quoted in this repository as though it described the present.** It
does not, and the architect repeated the error on 2026-09-03 before checking:

> Commit memory is O(total worktree bytes) regardless of change size: a 1 GB worktree allocates 1 GB
> whether one byte changed or none.

**That sentence describes the state DC-62 was written to measure. The very next line says DC-56
*will* fix it.** DC-56 did.

### 2.1 What was measured

Release build at `995d144`, Linux, worktrees under `/tmp` (tmpfs), peak RSS from
`getrusage(RUSAGE_CHILDREN).ru_maxrss` — an exact kernel figure, not `dc59`'s sampled `VmHWM`, so it
cannot miss the peak of a short run.

**Genesis commit, against total worktree bytes** (128 files of increasing size):

| worktree | peak RSS | over baseline |
|---|---|---|
| 8 MiB | 17.2 MiB | +9.2 |
| 16 MiB | 25.2 MiB | +9.2 |
| 32 MiB | 41.5 MiB | +9.5 |
| 64 MiB | 73.3 MiB | +9.3 |
| 128 MiB | 137.0 MiB | +9.0 |

**Slope 1.0 against a ~9 MiB constant.** A 1 GB first import would take roughly 1 GB of RAM.

**Genesis commit, against file *count*** (256-byte files): 100 → 14.5 MiB, 1,000 → 14.2, 4,000 →
14.0, 8,000 → 21.7. **Flat.** Cost follows bytes, not paths.

**Incremental commit** — 1 MiB changed in an already-committed 128 MiB repository, measured in a fresh
process so `RUSAGE_CHILDREN`'s running maximum could not contaminate it:

```
genesis, 128 MiB worktree            -> 137.3 MiB
incremental, 1 MiB of 128 MiB changed ->  16.3 MiB
```

### 2.2 What that means

**DC-56 works, and the common case is fixed.** An incremental commit costs ~16 MiB whatever the
repository's size — the changed-path index genuinely skips reads for unchanged files, which is
exactly what it was built to do.

**The create path is what remains.** `node_authoring.rs:360` accumulates
`create_candidates: Vec<(String, Vec<u8>, u32)>` — every newly-created file's **content**, pushed at
`:449`, sorted at `:476`, drained at `:477`. So first import and any mass-add hold all new content at
once.

**The buffer exists for a correctness reason**, stated in its own comment: fresh creates are minted in
canonical path order so node-id assignment does not depend on worktree traversal order. **But that
ordering needs the paths sorted, not the bytes held.** Whether the bytes must be resident is an open
question this RFC records rather than answers.

**This is the narrow, true version of the sentence in §2** — and it took measurement to find, which
is DC-62's own point turned back on this project: *"the specific risk is not 'did memory improve' but
is there still a path that loads everything."*

## 3. Two costs carried in from the corrective program

Both re-verified at `995d144` rather than trusted from rows written against `0.27.1`:

- **`AUD-01`** — `object_store.rs:174-179`: `self.entries.iter().rev().find(|e| e.object_id == id)`.
  A linear scan; `.rev()` is what gives last-entry-wins. `verify`/`seal` do O(objects) lookups.
- **`AUD-02`** — `wal.rs` calls `self.replay()?` inside append, so appending to a queue of N commits
  replays all N.

**Neither is measured.** Both are read from source, exactly the standard §2 shows to be unreliable.

## 4. "The two performance walls" — a label with no definition

The phrase appears three times and is defined nowhere: `ROADMAP.md`:328 lists it as unscheduled, RFC
126:107 says it is *"tracked in `ROADMAP.md`'s corrective program"*, and the architect's own criterion
handoff repeated it a third time. **Following either pointer leads back to the other.**

It most likely means `AUD-01` and `AUD-02`. **That is an inference, and an inference is not a record.**
**This RFC retires the phrase.** If two specific walls were meant that are not `AUD-01`/`AUD-02`,
whoever knows must say so; otherwise the named costs above are the whole list.

## 5. What is held by evidence, and what is asserted

| Property | Held by |
|---|---|
| `verify` is not superlinear in history length | **A gate** — `rfc111_index_decode_cost_gate.rs`, observed failing before its fix |
| `seal`'s decode cost | **A gate** — `rfc111_seal_decode_cost_gate.rs` |
| Incremental commit memory is independent of repository size | **One measurement, in §2 of this RFC.** No gate |
| Create-path memory is O(added bytes) | **One measurement, in §2.** No gate |
| `AUD-01`/`AUD-02` costs | **Nothing.** Source reading only |

**Time has two gates; memory has none.** That asymmetry is the subject of §6.

## 5.1 What the published documentation tells users — one row is badly stale

`docs/src/reference/architecture.md`'s known-limits table is the user-facing version of §5. Two rows
concern this RFC:

| Published row | State |
|---|---|
| *"`prikk verify` is roughly **O(N³)** in sealed block count — 34 s at 160 blocks — Tracked, unowned"* | **FALSE, and has been since 2026-08-18.** `MILESTONES.md` criterion 3 records `verify` **linear**, **27.04 ms at N=160**, per-doubling ratio 1.97 — MET via DC-92 and RFC 111, and now held by `rfc111_index_decode_cost_gate.rs` |
| *"Commit cost is not yet bounded independently of repository size (NFR-PERF-01) — Reduced, still missed"* | **Status genuinely open** — see §6 |

**The first row tells readers a solved problem is live, by a factor of about 1,250.** It is the
inverse of the usual documentation risk: the project is understating itself in public. **Correcting it
is documentation currency, not a decision** — the architect owns it and it needs no ruling.

## 6. The ruling this RFC carries — corrected 2026-09-03 after the owner questioned its shape

**The first draft of this section asked "does peak RSS get standing protection?" That was the wrong
question**, and the owner was right to ask what part of it is even this project's responsibility.
Checking the record answers most of it:

**What is already settled, and was before this RFC existed:**

- **Memory is not covered by any requirement at all.** DC-56 §163, verbatim: *"Objective 2 is not
  covered by any requirement. NFR-PERF-01 bounds cost in a latency sense; nothing names"* memory.
- **The owner already ruled the genesis case out of scope on 2026-07-30**: *"NFR-PERF-01 bounds
  **steady-state** commit cost, not every commit including the first."* §2's 137 MiB genesis figure
  is therefore outside what this project has ever committed to bound — **the owner's instinct that a
  large import is the user's matter is not a new opinion, it is the standing ruling.**
- **Untrusted input is already validated.** DC-86's `PRIKK_BUNDLE_MAX_BYTES` /
  `PRIKK_EXCHANGE_MAX_BYTES` / `..._MAX_OBJECTS` bound what a received bundle can make this process
  allocate. That is the part which genuinely was a validation problem, and it is done.

**So the residue is one question, and it is a requirements question rather than a testing one:**

> **Should "commit cost does not scale with repository size" be a stated requirement covering
> *memory*, as `NFR-PERF-01` already states it for latency — or is memory deliberately left
> unbounded?**

**Why it is the owner's:** requirements are. Nothing else here needs a ruling — if the answer is yes,
the evidence follows mechanically from §2's method and the architect writes it; if no, §5's table
gets a row saying memory is deliberately unbounded and this RFC closes.

**What §2 contributes to the decision:** the property is currently **true and free**. Incremental
commit measured **16.3 MiB against a 128 MiB repository**, independent of size — DC-56 already won
it. The question is only whether winning it should be *kept* by evidence rather than by nobody
noticing it broke.

**The cost of yes:** a test that builds two repositories of different sizes and asserts the ratio
between their incremental-commit peaks, run in the ordinary suite. It needs no threshold to
maintain — a ratio does not drift when the machine changes, which was RFC 126 §6's whole objection to
hand-maintained numbers. Its real cost is build time on every gate run, at sizes large enough for the
signal.

**The cost of no:** honest, and cheaper. It means a future change that reintroduces a full-worktree
read on the incremental path passes all 1,558 tests and reaches a release, and the first report comes
from a user with a large repository.

**A separate matter, also the owner's, surfaced by the same reading:** `NFR-PERF-01` itself is
recorded **unmet** — `MILESTONES.md`'s M1 row, DC-56's criterion 8 (*"Recorded 2026-07-31: still
missed"*), and the published table above. **Nothing has re-checked that claim since DC-64, RFC 111,
and DC-92 landed.** Whether it can now be claimed was always the owner's on evidence (DC-92 §101),
and no such evidence has been gathered. **This RFC does not ask for that ruling — it records that the
question is open and unmeasured**, since §2 measured memory and `NFR-PERF-01` bounds latency.

## 7. Scope

**In:** the costs named in §2 and §3; the evidence tables in §5 and §5.1; §6's ruling; retiring §4's
phrase; correcting `architecture.md`'s stale `O(N³)` row.

**Out:** fixing any of them. `AUD-01` and `AUD-02` keep their `ROADMAP.md` rows and their completion
conditions; this RFC is where their cost is described, not where it is repaired. **No increment
should be handed over from this RFC until §6 is ruled**, because the ruling decides whether a fix
needs to arrive with a standing measurement or without one.
