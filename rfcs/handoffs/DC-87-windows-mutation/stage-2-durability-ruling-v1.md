# DC-87 Stage 2 — Transition Durability Ruling v1

**Reviewing:** `.git-exclude/review-request/prikk-dc-87-stage2-transition-durability-v1.md`.

**Investigation accepted. §3.4's stop-and-report is upheld: Windows mutation cannot proceed on the
current ref-publication design.** Stage 2 is blocked — and it is now blocked on a **design decision**,
not on further fact-finding. §5 escalates that decision, because it reaches beyond Windows.

## 1. Verified

Every first-creation line reference checks out: `refs/log.rs:64` is the idempotent-duplicate branch
(`append … &[]` when the last record matches) and `:66` is the real record append — **their correction
of DC-88's earlier reading of that pair is right**; `wal.rs:120`/`:141` is the same idempotent-versus-real
split; `active.rs:121` and `trust.rs:106`/`:132` are `write_file_atomically`.

**§3.1's decomposition is the report's best contribution and I am adopting it as the frame.** Splitting
every guarantee into an **update of an existing name** (content and atomicity only) and a
**first appearance of a name** (inherently about naming) is what makes the Windows answer precise
instead of "no." It is also what makes a fix conceivable: the two halves have different answers.

Checking whether atomicity is load-bearing rather than assuming it — read-only commands take no lock and
genuinely run concurrently with mutation — is the right instinct and I had not asked for it.

## 2. Two corrections to the record

**2.1 — `FILE_RENAME_INFO`'s documentation status, sharpened.** The report says the POSIX-semantics flag
is "not officially documented by Microsoft." I read the page: the `Flags` field **is** documented, but
only as *"used when SetFileInformationByHandle's FileInformationClass parameter is set to
FileRenameInfoEx"* — **no flag values are enumerated at all.** So the accurate statement is that the
field is documented and its values are not. The load-bearing conclusion is untouched: the flag addresses
atomicity, says nothing about durability, and rename atomicity is not rename durability.

**2.2 — A DC-87 §3.1 claim is too broad, and this report's own source narrows it.** The earlier
prerequisite round concluded: *"no, a directory handle cannot serve as a resolution root for the next
component at the Win32 layer."* `FILE_RENAME_INFO` has a `RootDirectory` member — *"If FileName specifies
a relative name, this field can be a handle to the directory relative to which the new name is
resolved."* That is a Win32 structure taking a directory handle as a resolution root.

**It does not change G1's answer** — it resolves a rename *destination*, not an open, so the
component-by-component no-follow walk still has no Win32 primitive. But the blanket form of the claim is
wrong and would mislead whoever designs the anchoring story. Narrowed here rather than left standing.

## 3. §3.3's open question, answered as far as it can be

They asked whether a one-time `init`-time first-creation gap is acceptable where an ongoing one is not,
and said they could not derive it from the contract. Correct — it is a judgment, and it is mine.

**A crash during `init` loses no history, because none exists.** The remedy is to run `init` again. That
is categorically different from a repository whose *sealed history* says something false, which is the
one thing prikk claims never happens. So an init-time-only gap would be acceptable, documented.

**But "one-time" is the wrong frame, and their own §3.3 shows why.** A new ref creates a new ref-log
file; a new WAL appears per active session. First-appearance recurs for the life of the repository, not
once at creation. So the design target is not "shrink the gap to init" — it is:

> **Does any durability-bearing transition require a name that did not previously exist?**

Every "yes" is a Windows hole. That is the question a design has to drive to zero, and it is sharper
than anything I gave them.

## 4. The ruling

**DC-38's invariant does not survive on Windows**, for the reason stated: step 6's log append is an
existing-file content append and *is* achievable, while step 5's pointer promotion needs transition
durability and is *not*. A crash between them reproduces exactly the ahead-log state DC-38 exists to
eliminate. The asymmetry is what makes it fatal — if neither were durable there would be no ordering
hazard.

**Stage 2 stays blocked.** Not on facts: this investigation has taken the fact-finding as far as it
goes, and I am not asking for another round of Windows API archaeology.

**Stage 1's seam refactor stays held, and the sequencing call is now settled by evidence rather than
argument.** Had it gone first, it would have been a refactor built to house an implementor that cannot
exist under the current design.

**They correctly did not reach for DC-38's format-1 ahead-log recovery**, and correctly did not revive
DC-88 §3's two-slot sketch on their own authority. Both were available and both would have been wrong to
take unilaterally.

## 5. Escalation: this reaches past Windows, so the scope choice is the owner's

§3.4 names the promising direction — a fixed-name record whose transitions are content updates rather
than new names converts the ongoing problem into the achievable case. **But that means changing how
prikk publishes refs**, and you would not want two different publication mechanisms across platforms.
So the realistic options are:

1. **Restructure ref publication** so every durability-bearing transition is a content update to an
   already-named file. Large, touches DC-34/DC-38's state machine, and lands on **all** platforms.
2. **Ship Windows with a documented weaker invariant**, leaning on ahead-log recovery. Cheaper, and it
   converts format-2's deliberate *rejection* of that state into *recovery* — a security change needing
   its own analysis, which I have twice recommended against.
3. **Do not ship Windows mutation.** Document it. Read-only on Windows, already CI-gated, stays.

**The question I would ask before choosing, and the reason I am proposing DC-91 rather than
recommending an option outright:** does a fixed-name, slot-based publication record have **independent
value on POSIX** — fewer reachable crash states, less dependence on directory-sync ordering — or is it
purely a Windows tax? If the former, option 1 stops being a cost imposed by a platform and becomes an
improvement that also unblocks one. If the latter, option 3 is honest and cheap, and Windows mutation
waits for a better idea.

I do not know which, and I am not going to guess at it after this cycle's record. **DC-91 asks exactly
that question and nothing else.** Whether it runs, and whether 0.20.0 waits, is yours.

## 6. Standing

- **DC-87 Stage 2:** blocked on the §5 decision.
- **DC-87 Stage 1's seam refactor:** held with it.
- **DC-91:** proposed alongside this ruling.
- Nothing here changes DC-88's or DC-90's merged state, or Windows read-only support, which is
  unaffected throughout.


---

## 6. Deferral LIFTED, 2026-08-16 — §5's option 1 was executed

**§4's ruling is discharged, and by the route §5 named.**

§5 listed three realistic options and put the scope choice to the owner. **Option 1 —** *"restructure ref
publication so every durability-bearing transition is a content update to an already-named file. Large,
touches DC-34/DC-38's state machine, and lands on all platforms"* **— is exactly what RFC 102 did**, across
six stages, completed 2026-08-15.

**§4's stated condition is now void, not merely outdated.** It read: *"step 6's log append is an existing-file
content append and is achievable, while step 5's pointer promotion needs transition durability and is not.
The asymmetry is what makes it fatal."*

**There is no step 5.** `refs/publication.rs` records the retirement: the candidate-write-then-promote
mechanism *"has no equivalent under an append-only pointer index — an append-only record has no candidate
value to stage, the append **is** the publish."* `write_ref_pointer_candidate`/`promote_ref_pointer_candidate`
are gone; only a test-only shim remains (`pointer_index.rs:302`).

**Every durability-bearing operation in a publication is now an append or a truncate on a name allocated
at `init`** — the RefState object's container and index records (`publication.rs:73`), the pointer-index
entry (`:117`), the ref-log record (`:125`), and in the interrupted-tail branch a `durable_truncate`
(`:92`). All are the achievable kind. **The asymmetry that made it fatal is gone precisely because no
operation in the sequence needs transition durability any more.**

**The only names a publication creates after `init` are lock files** — `RefLock::acquire` (`:48`) and
`acquire_container_locks` (`:58`). **These are not durability-bearing**: a lock file lost to a crash is
harmless, because the process holding it is gone too. A Windows implementor still has to create them, and
should not read the sentence above as saying publication touches nothing but appends.

**And DC-38's invariant now holds by construction rather than by primitive.** `PublicationState` has three
variants — `Ready`, `PointerLeading`, `Complete` — and **no ahead-log state exists**. A crash between the
two appends leaves *pointer leading*, the opposite of an ahead log, and completable.
`publication.rs:95-96` states it: *"The pointer-first order below is what prevents a crash from ever
producing it through normal publish."* That ordering is platform-independent.

**§5's own open question is answered too.** It asked whether a fixed-name publication record has
*"independent value on POSIX… or is it purely a Windows tax."* RFC 102 shipped it on all platforms and it
paid for itself independently: corruption isolation per record, criterion 2 closed for the repository, and
compaction. **It was not a Windows tax.**

**Ruling: Stage 2 is unblocked.** Not because the facts were re-litigated — §4 was right on its facts —
but because the design it ruled against was replaced. **Option 3 (do not ship Windows mutation) is no
longer the honest cheap answer; option 2 (weaker invariant, ahead-log recovery) was correctly refused
twice and is not needed.**

**What is still blocking is DC-87 §3.1 alone** — G1 anchored resolution, where the developer's 2026-08-16
report states a real, nameable gap: inter-component TOCTOU is open on Windows and closed on Linux/macOS.
That is a separate condition with its own accepted-once precedent, and it is not what this deferral was
about.
