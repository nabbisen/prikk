# DC-87 Stage 2 — Transition Durability Investigation: Handoff v1

**Cleared to answer §3's five questions only.** Investigation, no code, no design commitment.
**Scope authority:** `rfcs/accepted/DC-87-WINDOWS-MUTATION.md`. This is Stage 2's design work, not a new
RFC — DC-87 §3.2's question was answered and the blocker moved; this handoff states where it moved to.

## 1. Sequencing ruling: this precedes Stage 1's seam refactor

Stage 1 is available and unblocked. **Take this first anyway.**

Stage 1's seam exists for exactly one reason: to give a Windows authority type somewhere to live. If the
answer below is that Windows cannot satisfy the transition guarantees, that seam has no consumer and the
refactor is speculative work on a shape nothing will use.

**This is the same principle I applied when I put Stage 1 on hold behind DC-88 — know the requirement
before building the shape.** That application was wrong, because DC-88 turned out to touch no type and
change no capability, and I released the hold. This one is right, because this question genuinely can
change what a Windows authority type must be able to do. Same rule, correct target this time. Say so if
you disagree; you have overruled my sequencing before and were right.

## 2. Where the blocker actually is

Read `.git-exclude/reviewed/DC-88-prerequisite-questions-review-v1.md` §4 first if you have not.

Your own DC-88 investigation established it and I confirmed every claim: **DC-38 never calls
`durable_directory_entry`.** Its three durability-bearing steps go to `atomic_replace` (`pointer.rs:51`),
`promote` (`refs.rs:306`), and `durable_append` (`log.rs:64`/`:66`) — and each of those bundles its own
directory sync as an integral part of *its own* guarantee (`linux.rs:45`, `:150`/`:152`, `:58`).

So the obstacle was never the method DC-88 fixed. It is in those three. Windows has no directory sync,
so a Windows implementor cannot satisfy them the way Linux does.

**The fact that makes this tractable, and it is worth stating plainly:** the contract is already
requirement-shaped here. `atomic_replace` promises *"replace this file's content atomically, durably"* —
it does **not** say "and fsync the parent." The directory sync is an implementation detail inside
`LinuxDurability`. **A Windows implementor is already free to satisfy these three guarantees any way it
can, with no contract change at all.** DC-76 got this right everywhere except the one method you just
restated.

## 3. The questions

**3.1 — For each of `atomic_replace`, `promote`, and `durable_append`, state what its guarantee requires
of the platform**, independently of how POSIX happens to satisfy it. Work from the trait's doc comments,
not from `linux.rs`. This is the step that separates "what we need" from "what we currently do," and
everything else depends on getting it right.

**3.2 — Can each be satisfied on Windows? With what, and with what residual gap?** Known, from your own
prerequisite round and my corrections to it — do not re-derive, but do challenge if you find otherwise:

- `FlushFileBuffers` applies to files, not directories.
- `REPLACEFILE_WRITE_THROUGH` is documented, verbatim, *"This value is not supported."*
- `ReplaceFileW` is **not** atomic — its own page documents three partial-completion error codes
  (1175/1176/1177), one of which leaves the replaced file gone and the replacement under its original
  name.
- `MOVEFILE_WRITE_THROUGH`'s *guarantee* sentence is scoped to a cross-volume copy-and-delete mechanism a
  same-volume rename never uses. Its first sentence is unqualified. **This was investigated and could not
  be settled from primary sources; three corroboration attempts failed.** Do not spend more time on it
  unless something new surfaces.
- `NtFlushBuffersFileEx` is framed throughout its own documentation as driver-linkage.

**3.3 — First creation.** Your DC-88 §4.3 traced this only for the objects DC-38 names and said so.
That scope limit is now load-bearing: a Windows implementor must satisfy **every** first-creation path,
not only those. Enumerate the rest — the WAL's first file, the trust store, active-ref metadata, and
anything else you find — and say for each whether its first appearance can be made durable on a platform
with no directory sync.

**3.4 — If any guarantee cannot be fully satisfied, what is the honest weaker one, and does DC-38's
invariant survive it?** DC-38 states outright: *"Format-2 publication never permits an ahead log."* If
a Windows implementation cannot hold that, **stop and report.**

Do not design around it. DC-38's format-1 compatibility clause already defines a bounded recovery for
the ahead-log state, and it is tempting to reach for — but format-2 *rejects* that state deliberately,
and converting rejection into recovery is a security change needing its own analysis. **That is a
design increment, not this investigation's to take.**

**3.5 — Only if 3.2 says Windows is viable: price the two implementation routes.** The owner ruled
`unsafe` is allowed under control; the choice of route was deliberately deferred to measured numbers
rather than principle (`.git-exclude/reviewed/DC-87-unsafe-surface-analysis-v1.md` §8):

- **Bespoke FFI crate** — actual count of `extern` declarations needed for whatever 3.2 concludes, and
  what its `SAFETY:` comments would have to assert.
- **`cap-std`/`cap-primitives`** — you already measured 13 transitive packages on the Windows target.
  Confirm it still resolves what 3.2 needs, since your earlier finding was that it answers G1 and
  **nothing** about durability.

Report both. Do not choose — the `ALLOWED_THIRD_PARTY` amendment is mine, and a new workspace crate
without `forbid(unsafe_code)` is a standing-property change the owner has permitted but not directed.

## 4. Limits

- **No code.** No Stage 1 refactor, no Windows implementor, no experiment branch.
- **No contract change.** The three methods are already requirement-shaped; if you conclude one is not,
  that is a stop-and-report, not an edit.
- **Do not reopen DC-38's state machine.** 3.4 tests whether it survives; it does not renegotiate it.
- **"I could not determine this" remains a first-class answer**, and it was the right one for
  `MOVEFILE_WRITE_THROUGH`. Better than a confident wrong conclusion, and this project has had to
  correct several of mine.

## 5. Reporting

`.git-exclude/review-request/`, plain `.md`. Answer in order; 3.1 first, and let it discipline the rest.
Findings outside scope go in the report and I register them in `FINDINGS.md`.

## 6. Standing

- **DC-88** (`ed04c21`) and **DC-90** (`f358353`) are both accepted and merging after their CI runs.
  Neither is yours to progress further.
- **DC-90 merging is what permits the first `unsafe` line.** It does not gate this investigation, which
  writes no code.
- **DC-87 Stage 1's seam refactor:** available, but sequenced after this per §1.
