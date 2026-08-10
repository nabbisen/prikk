# DC-88 Durability Contract Requirement Shape — Prerequisite Handoff v1

**Cleared to answer §4's four questions only.** Accepted 2026-08-10,
`rfcs/accepted/DC-88-DURABILITY-CONTRACT-REQUIREMENT-SHAPE.md`. **Design follows the answers.**

## 1. Read this first, because it changes your sequencing

**I am revising an earlier ruling of mine.** The DC-87 prerequisite ruling cleared Stage 1's seam
refactor to run in parallel with the open durability question. That was correct when the question had no
owner. It now has one — this increment — and the situation changed:

- DC-88 §4.2 asks whether restating `durable_directory_entry` leaves Linux and macOS byte-for-byte
  unchanged. **If Stage 1 is reshaping `MutationRoot` concurrently, that baseline moves underneath the
  question** and the answer stops meaning anything.
- More importantly, Stage 1's seam exists to accommodate a Windows authority type. **DC-88 may change
  what that type has to be capable of.** Designing the seam before knowing is the DC-82 mistake run
  backwards — building the shape first and discovering the requirement after.

**So: DC-88 runs alone. Stage 1's seam refactor waits for its answer.** DC-87's mode-carrying fix is
already done and is unaffected; it is only the seam that waits.

This costs a little parallelism and buys a seam designed against a known requirement. Say so if you
disagree — you have overruled me correctly before.

## 2. What this increment is actually about

Not Windows. **A contract question that Windows merely exposed.**

DC-76's thesis is in the contract's own module documentation: *"Guarantee, not syscall — the whole
point."* `durable_directory_entry` is the one method that does not meet that bar. Its guarantee — every
mutation under this directory since the last durability point survives a crash — is a directory-scoped
batching concept that exists **because POSIX has directory fsync**. The doc half-concedes it: "satisfied
on Linux by `fsync` on the directory fd."

That is my read, and §4.1 exists to test it rather than assume it. **If the honest answer is that
callers genuinely want directory-scoped batching, that ends this RFC** and DC-87 Stage 2 goes back to
the weaker-invariant conversation. That outcome is not a failure of the increment.

## 3. Where to start

**§4.1 first, and let it decide the rest.** Enumerate every caller of `durable_directory_entry` and
state what each one actually requires. Not what the method gives them — what they would still need if
the method vanished. Some may want "this directory's entries are now durable." Some, I suspect, want
"this specific transition is now durable" and reach for the only tool available. **Report the split.**

**§4.4 is the one I would not let slide.** DC-38 is why this matters at all. A restatement that quietly
weakens DC-38's state machine on Linux is worse than the problem it solves — Windows is a platform we do
not yet support, and Linux is one people are running today. If the restatement cannot be shown to leave
DC-38's guarantees intact on POSIX, stop and report.

**§4.3 is the gap in my own candidate shape**, and I flagged it in the RFC rather than letting you find
it: the two-slot record addresses *transitions*, not the first creation of a pointer or log file, where
a directory entry genuinely must appear and become durable. I do not know what that costs. Report it.

**On §3's candidate shape generally: it is a proposition, not a ruling.** My design assertions on this
increment's subject matter have needed correction twice — I set the wrong blocking question for DC-87,
and my G9 framing was wrong in a way you had to work around. Treat the two-slot sketch as evidence that
the impasse is not real, nothing more. If a better shape exists, that is the increment's to find.

## 4. Limits

- **No design in this pass.** Answers first.
- **No change to the nine guarantees.** This is about how one of them is *stated*.
- **No Linux or macOS behaviour change.** If the restatement cannot be done without one, stop and
  report.
- **No Windows implementation.** That is DC-87 Stage 2, after this.
- **Do not reopen DC-38's state machine.** §4.4 checks it survives; it does not renegotiate it.

## 5. Reporting

`.git-exclude/review-request/`, plain `.md`. Answer the four in order. Findings outside DC-88's scope go
in the report; I register them in `FINDINGS.md`.

## 6. Sequencing

- **DC-87's mode-carrying fix** (`1e10a09`) is accepted and awaits a green three-platform CI run before
  merge. Not yours to progress further.
- **DC-87 Stage 1's seam refactor: on hold**, per §1.
- **DC-87 Stage 2:** blocked on this increment, and separately on the owner's decision about whether
  prikk gains its first `unsafe` surface — still open.
- **DC-89** (platform-claim documentation accuracy) is proposed and awaiting the owner. If it is
  accepted while this is in flight, it touches only `docs/` and will not collide.
