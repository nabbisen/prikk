# RFC 111 — prerequisite handoff v1

**RFC:** `rfcs/proposed/111-object-read-cost-regression.md` (ACCEPTED 2026-08-18)
**Scope of this handoff:** RFC 111 §6.1 only. **§6.2 (positional reads) is held back** — it is worth ~5%
and must not be bundled in. **§6.3 (a cost gate) is an open question for the owner** and is not yours.

**Answer these before designing, and report.** The cause is already located and measured, so this is not
an investigation into *what* is slow — §2-§5.1 settle that. It is an investigation into **what a snapshot
may safely be scoped to**, which is a correctness question the measurement cannot answer.

## 1. What is already established — do not re-derive it

- `FileObjectStore::read_object` → `lookup_object_location` → `replay_index(layout)` **decodes the entire
  object index on every lookup**. That is ~82% of the regression (`verify` at N=160: 164.90 ms → 29.20 ms
  with it memoized; the container full-read is the other ~5%).
- Restoring it restores DC-92's linear curve: 27.28 ms at N=160 against `b718623`'s 28.88 ms, exponent
  1.06.
- **The probe that proved this is unsound as a fix.** It was a process-lifetime thread-local cache.

Reproduce the numbers if you want them in your own hands — `dc92_lineage_replay_benchmark.rs`, release
build — but you are not asked to re-establish the cause.

## 2. Q1 — Which call sites read, which write, and which do both?

`lookup_object_location` has a caller that is **not** a read: `index.rs:360`, inside
`write_object_to_container`. It is there to enforce RFC 102 Stage 3's idempotency contract — *"a same-id
rewrite is a silent no-op only when its full envelope bytes match what is already stored"*, and a same-id
rewrite with **different** bytes must be an error, not a silent accept.

**That call site is the whole difficulty.** `write_object_to_container` also *appends* to the index, so
within a single writing operation the index changes underneath any snapshot taken before it.

**Produce a complete, classified inventory** of every path reaching `lookup_object_location` and
`read_object`: read-only, writing, or mixed. **Enumerate it from the code, not from the list in this
document** — the grep in §5 of the RFC was a lower bound and I did not filter test-only callers carefully.

## 3. Q2 — What is the smallest scope that is safely snapshottable?

Given Q1's inventory, answer: **what unit of work can hold a decoded index snapshot without any
possibility of reading it stale?**

Candidate shapes, not a menu to pick from unilaterally:

- **A read-only session object** created by operations that only read, passed explicitly down. Writers
  never receive one and keep today's behaviour. Explicit, verifiable by type, and invasive at every call
  site.
- **A snapshot owned by `FileObjectStore` with invalidation on append.** Less invasive, but correctness
  then depends on *every* future writer remembering to invalidate — the failure mode is silent and the
  guard is a convention.

**State which you recommend and why, with the failure mode of each named.** The question I care about is
not which is less code: it is **which one makes a stale read impossible rather than unlikely**.

## 4. Q3 — What does a stale snapshot actually break, concretely?

For your recommended shape, construct the specific scenario in which a stale snapshot would be read, and
say what the observable damage is. If your answer is "it cannot happen", **say what structurally prevents
it** — a type that writers cannot obtain, a lifetime that cannot outlive the operation — rather than that
no current caller does it. *"No caller does this today"* is a fact about today.

**If the honest answer is that the idempotency check at `index.rs:360` must always see fresh state, say
so** — that is a finding, and it may mean writers simply never get a snapshot.

## 5. Q4 — Does the fix hold under concurrency?

RFC 102 Stage 6 added container locks (`acquire_container_locks`, `LockableContainer`) and
`prikk unlock` for stale-lock recovery. **Does a read-only operation holding an index snapshot interact
with a concurrent writer in another process?** A reader with a stale snapshot is not obviously wrong —
`verify` already reports a point-in-time state — but **say what the guarantee is** rather than leaving it
implied.

## 6. Constraints

- **No format change, no identity-bearing byte change, no format bump.** This is a read-path cost defect.
- **`forbid(unsafe_code)` holds.** DC-90's boundary is not in scope and nothing here needs it.
- **No new dependency** without the workspace-dependency convention (root `[workspace.dependencies]` plus
  `{ workspace = true }`), and say why an existing crate will not do.
- **Do not implement §6.2.** If positional reads fall out of §6.1's work for free, report that and stop.

## 7. What to report

Q1's classified inventory, Q2's recommendation with rationale, Q3's concrete failure scenario, Q4's
guarantee. **Report before designing**, per this project's standing shape — and per RFC 111's own history,
where reading the code produced a plausible fix that measurement showed addressed 5% of the problem.

**If any question turns out to be the wrong question, say so and say why.** That is a finding, not a
deviation.
