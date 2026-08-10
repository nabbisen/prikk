# DC-87 — Prerequisite Investigation Ruling v1

**Reviewing:** `.git-exclude/review-request/prikk-dc-87-prerequisite-questions-v1.md`.
**Verdict: accepted as an investigation.** All six answered against primary sources, uncertainty marked
where it was real rather than reasoned past, and no design attempted. The `#![forbid(unsafe_code)]`
finding is the most consequential thing in it and was not something §3 asked for.

Below: what I verified, **three corrections**, the DC-38 ruling they escalated, and what happens next.

## 1. Verified

- `#![forbid(unsafe_code)]` at `crates/prikk-store/src/lib.rs:1`, `unsafe_code = "forbid"` in the
  workspace lint table. The `forbid`-versus-`deny` point is correct Rust semantics. **This reframes
  the whole increment**, and it is why §4.5 escalates rather than rules.
- `rustix` declared unconditionally at `prikk-store/Cargo.toml:20` while every caller is
  `cfg(any(linux, macos))`-gated. Confirmed; dead weight on Windows builds.
- The placement gate walks `[target.*.dependencies]` (`placement.rs:51-68`). They re-confirmed my own
  claim rather than taking it — right instinct.
- §3.6's read-path gap. I had reached the same conclusion independently while writing the RFC; their
  read of all four branches is more thorough than mine was. Registered in `FINDINGS.md`.

## 2. Corrections

**2.1 — `ReplaceFileW` is not an atomic-replace primitive.** §3.4 calls it "the better-supported
atomic-replace primitive." I fetched its Microsoft Learn page: it documents **three distinct
partial-completion error codes** — `ERROR_UNABLE_TO_REMOVE_REPLACED` (1175),
`ERROR_UNABLE_TO_MOVE_REPLACEMENT` (1176), `ERROR_UNABLE_TO_MOVE_REPLACEMENT_2` (1177). 1176's own
text: *"the replaced file no longer exists and the replacement file exists under its original name."*
Those are documented torn states. The conclusion of §3.4 stands and is strengthened; the primitive
named as the better option is not one.

**2.2 — `REPLACEFILE_WRITE_THROUGH` is documented, verbatim, as "This value is not supported."** So
`ReplaceFileW` offers no durability lever at all. New fact, not in the report.

**2.3 — `MOVEFILE_WRITE_THROUGH` was not considered, and it is the most important unexamined lead
here.** Verbatim from the `MoveFileExW` flags table:

> The function does not return until the file is actually moved on the disk. Setting this value
> guarantees that a move performed as a copy and delete operation is flushed to disk before the
> function returns. The flush occurs at the end of the copy operation.

The first sentence is an unqualified durability statement about the move. The second narrows the
*guarantee* to the cross-volume copy-and-delete case. That ambiguity is unresolved and it matters more
than anything else in this round.

§3.2 asked "is `durable_directory_entry` implementable on NTFS" and answered it correctly: there is no
supported user-mode way to sync an arbitrary directory's entry list. But **prikk does not need a
general directory sync on Windows. It needs specific operations to be durable when they return.**
Those are different questions, and the second has a partial answer the first does not.

## 3. The DC-38 ruling

They escalated this correctly and did not guess at it. I read DC-38.

The state machine (§"Design contract", steps 1-7) requires "write and **required-sync** the pointer
candidate" (3), "promote and **required-sync** the authoritative pointer" (5), then "append/fsync
exactly one committed RefUpdate" (6). The order is pointer-first *precisely* so a crash can never leave
the log ahead of the pointer. DC-38 states the invariant outright: **"Format-2 publication never
permits an ahead log."**

If step 5's promotion cannot be made durable while step 6's append can, a crash between them yields an
ahead log with a stale pointer — **the exact released format-1 defect DC-38 exists to eliminate.** So
their instinct is right and the consequence is specific: *DC-38's invariant does not carry to Windows
on the same terms.*

It is not fatal, and here is the part the investigation could not reach without reading DC-38:
**prikk already has a bounded recovery path for that state.** DC-38's format-1 compatibility clause
defines it — retry "validates the one already-signed ahead transition and promotes its RefState pointer
without appending a record." The Windows weakening lands on a state the product already knows how to
finish.

**That is not permission to enable it.** Format-2 *rejects* that state by design, and turning rejection
into recovery is a security change, not a portability adjustment. Signature verification of the ahead
RefUpdate is necessary and not sufficient; the question is whether the state can be manufactured by
anything other than an interrupted local publication. That analysis is not DC-87's to absorb.

**Ruling.** Settle 2.3 first — if a same-volume rename is genuinely durable on return, most of this
evaporates and DC-38 carries unchanged. If it is not, the ahead-log recovery question comes back to me
as a design increment of its own, and Stage 2 waits for it.

**Hard line, restating criterion 2 for this case specifically:** Windows mutation does not ship with an
invariant weaker than Linux's left undocumented, and does not ship with a *silently* weaker one at all.

## 4. The rest

**4.1 — §3.1 (G1). Accepted, uncertainty included.** I am not requiring a primary-source proof that a
user-space per-component walk closes every race a single relative-open syscall would. That proof does
not exist for anyone, cap-std included. What I require is that the difference be **stated** in
`docs/src/reference/platform-support.md` rather than elided. Declining to claim it was correct.

**4.2 — §3.2.** See §3. Second-round question: settle 2.3.

**4.3 — §3.3 (G9). The framing is wrong, and the real hazard is in prikk's code, not Windows'.**

prikk does not record the write bit. `normalize_file_mode`
(`worktree_patch/node_authoring/worktree_files.rs:100-106`) collapses every mode to exactly two values
— `REGULAR_FILE_MODE` (`0o100_644`) or `EXECUTABLE_FILE_MODE` (`0o100_755`) — branching on
`mode & 0o111`, the **execute** bits. `FILE_ATTRIBUTE_READONLY` maps to the *write* bit, which prikk
ignores entirely. So the Git-for-Windows precedent addresses a bit prikk does not record, and "map
owner-write only" has nothing to map to.

The actual hazard is sharper. `read.rs:149-150`'s non-Unix branch hardcodes `mode = 0_u32`. Zero has no
execute bits, so `normalize_file_mode(0)` returns `REGULAR_FILE_MODE` — **every file authored on a
mutation-enabled Windows would be recorded non-executable**, and a Linux-authored executable
re-committed on Windows would silently lose its executable bit *in sealed history*. That is acceptance
criterion 7's divergence, arriving through a sentinel rather than an error. It is unreachable today
only because mutation is unsupported there. **DC-87 makes it reachable.**

**Ruling: on Windows the recorded mode must not be derived from the filesystem.** Carry it from the
node's existing recorded mode; change it only on an explicit operator action. The bounded shape of
`normalize_file_mode` makes this tractable — one bit, two values. Report the shape before implementing
it. And `read.rs`'s sentinel `0` must become an error or an explicitly-carried "unknown"; it must never
be a value that flows into authoring.

**4.4 — §3.4. Accepted**, and strengthened by 2.1/2.2. `promote`'s doc comment does not hold on
Windows; that is criterion 2 territory, and the doc comment is what has to change, not the reader's
expectations. The `FILE_SHARE_DELETE` observation is the best-reasoned part of the report: it is a
codebase-wide discipline rather than a call-site fix, and making it enforceable in **one** place is
exactly what Stage 1's seam is for.

**4.5 — §3.5. No dependency decision yet**, and the reason is §3: `cap-std` resolves §3.1 and nothing
else, and 2.3's answer may change what is needed. The measured footprint is genuinely useful work; the
`ipnet`-is-non-optional catch is the right kind of scrutiny.

Two things here are not the increment's:
- The `ALLOWED_THIRD_PARTY` amendment is mine.
- **Whether prikk gains its first `unsafe` surface at all is the project owner's.** A new workspace
  crate without `forbid(unsafe_code)` is not an implementation detail — it changes a standing property
  of the codebase, in a project whose stated position is that security is prioritized over function.
  **Escalating rather than ruling.** Adopting a third-party crate that has already encapsulated the
  unsafe code keeps `forbid` intact everywhere prikk writes; that difference is the owner's to weigh,
  and it should be weighed with 2.3's answer in hand, since 2.3 may decide whether raw native calls
  are needed at all.

**4.6 — §3.6.** Confirmed independently, registered, correctly not repaired here.

## 5. What happens next

**Stage 1 is cleared to start now.** It depends on none of the open questions: it is a Linux + macOS
behaviour-preserving refactor of `MutationRoot`'s authority into a per-platform type behind one
interface, and it does not touch `DurabilityContract`'s method set. Running it in parallel with the
unresolved durability question is deliberate — the owner asked for Windows as soon as possible *with* a
safe process, and parallel is how both are possible. Serial would trade one for the other.

**A second, narrow prerequisite round runs alongside it:**

1. **Settle `MOVEFILE_WRITE_THROUGH` (2.3).** Is a *same-volume* rename durable on return, or is the
   guarantee only the cross-volume copy-and-delete one? Primary sources; "I could not determine this"
   remains an acceptable answer and is better than a confident wrong one.
2. **Report the §3.3 mode-carrying shape** (4.3) before implementing it.

**Stage 2 stays blocked** until DC-38's consequence is ruled — by 2.3 resolving it, or by the design
increment §3 describes.

**Green CI on all three platforms before either stage merges.** Unchanged.

## 6. Registered in `FINDINGS.md`

Three rows, all from this round: the non-Unix read path's weaker G1; the `forbid(unsafe_code)`
constraint on any Windows syscall path; and the `read.rs` mode sentinel that would silently drop the
executable bit from sealed history once Windows mutation is enabled.
