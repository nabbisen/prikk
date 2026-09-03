# RFC 133 — What performance costs this project has, and what evidence holds them

**Status.** **Proposed.** Extracted from RFC 126 §5/§5a on the project owner's instruction,
2026-09-03: the measurement concern is *"an independent subject or theme"*, not a verification-culture
gate. **One ruling is carried over unchanged and is still the owner's — §6.**

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

## 6. The ruling this RFC carries over — the owner's

**Does peak RSS get any standing protection, and in what shape?** Moved intact from RFC 126 §5a,
now asked against §2's numbers instead of an abstraction.

**Criterion cannot answer it**: it measures wall-clock time against a stored baseline and has no
memory axis. `dc59_commit_benchmark.rs`'s `VmHWM` pass is the only peak-RSS instrument in the project
and is `#[ignore]`d, so **nothing stands between a regression and a release except someone
remembering to run it by hand.**

**Three shapes, unchanged from §5a:**

1. **A scheduled CI job** at one fixed size against a recorded threshold. Catches order-of-magnitude
   regressions; carries the drift risk RFC 126 §6 named.
2. **A release-cut-only check** — run before a tag rather than per increment. Cheaper; fails the
   "invisible between cuts" test.
3. **Record that peak RSS is unmeasured on a standing basis** and stop implying otherwise.

**No architect recommendation is offered, deliberately.** RFC 126 §6 chose criterion over a
hand-maintained threshold because criterion's baseline maintains itself; **for memory the better
option does not exist**, so that reasoning does not transfer. Accessibility and long-term stability
decided §6 and decide this.

**§2 does change one input to the decision**: the exposure is now known to be first-import and
mass-add, not everyday commits. Whether that makes standing protection more or less worth its cost is
the judgement being asked for.

## 7. Scope

**In:** the costs named in §2 and §3; the evidence table in §5; §6's ruling; retiring §4's phrase.

**Out:** fixing any of them. `AUD-01` and `AUD-02` keep their `ROADMAP.md` rows and their completion
conditions; this RFC is where their cost is described, not where it is repaired. **No increment
should be handed over from this RFC until §6 is ruled**, because the ruling decides whether a fix
needs to arrive with a standing measurement or without one.
