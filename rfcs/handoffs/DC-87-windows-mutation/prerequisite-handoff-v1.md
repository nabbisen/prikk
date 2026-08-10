# DC-87 Windows Mutation — Prerequisite Handoff v1

**Cleared to answer §3's six questions only.** Accepted 2026-08-10,
`rfcs/accepted/DC-87-WINDOWS-MUTATION.md`. **Design follows the answers; implementation follows the
design.** Do not begin Stage 1's refactor on the strength of this handoff.

## 1. Why this is an investigation and not a handoff to build

Read §2 of the RFC before anything else. The short version: DC-82 collapsed the *dispatch*, so a third
platform is one arm on `ACTIVE_DURABILITY` — but the layer underneath is Unix-shaped in its **types**,
not merely in its calls. `MutationRoot` holds a `rustix` fd and holds it only under
`cfg(any(linux, macos))`; off those platforms it silently degrades to a `PathBuf` and
`fallback_path` just joins onto it. `openat` has no Win32 equivalent, and **G1 is defined in terms of
the fd-anchored, component-at-a-time, no-follow walk.**

That is the whole difficulty, and I do not know its answer. My platform assertions have needed
correcting repeatedly this cycle — DC-82's criterion 3 was a target I set from outside the code and got
wrong, and it is the reason this increment exists at all. So §3 asks rather than rules.

## 2. What you do not have to worry about

Stated so you spend the investigation where the risk actually is:

- **Path policy is already Windows-hardened, cross-platform.** DC-72's validator rejects backslashes,
  colons, non-ASCII, control bytes, components ending in a space or dot, and the reserved device stems
  — on every host. Alternate data streams and drive-relative forms are unreachable because `:` is
  rejected outright. The usual Windows-port disaster is already paid for.
- **Windows already builds and already runs read-only**, CI-gated on every push. You are converting it
  from `NoDurability` to a real implementor, not starting from nothing.
- **Dispatch is done.** `anchored.rs:50-55`. Do not redo DC-82.

## 3. Where to start, and what I expect to break

**Take §3.2 first — `durable_directory_entry` on NTFS.** Not because it is the first listed, but because
it is the one most likely to change the shape of everything else. If G3's worked example cannot be
obtained on Windows, that is not a detail to route around; it reaches back into **DC-38's
ref-publication crash-recovery reasoning**, which was written against a primitive Windows may not have.
Answer the second question as carefully as the first: *does DC-38 still hold under the weaker
guarantee?* If the answer is no or unclear, **stop and report** — that comes back to me as a design
question, not something to absorb.

**§3.1 (G1 on Windows) is the one where I most want a plain answer.** If root-anchored no-follow
resolution cannot be held to Linux's standard, say so. A documented platform difference is an acceptable
outcome. A decision that Windows mutation is refused until it can be held is an acceptable outcome. **An
approximation described as G1 is not**, and I would rather read "this cannot be done" than accept
something that looks like the guarantee and isn't.

**§3.6 is a finding, not a task.** I believe `read.rs`'s non-Unix branch already holds weaker path
guarantees than Linux for read-only operation. Confirm or refute it, and **report it — do not fix it
here.** A read-path security fix and a new-platform mutation backend have different proofs; bundling
them makes a failure unattributable. That is the same reasoning that split DC-82 out of DC-81, and it
applies with more force now.

**§3.5 before you add anything.** Report the crate, feature set, and transitive count first.
`ALLOWED_THIRD_PARTY` is mine to amend, not yours to edit — and I verified the placement gate already
covers `[target.*.dependencies]` (`boundary/placement.rs:51-68`), so a Windows-only crate genuinely
cannot slip past it. Prefer `std` if `std` suffices.

## 4. Limits

- **No design in this pass**, and no Stage 1 refactor. Answers first.
- **No change to `DurabilityContract`'s method set or the nine guarantees.** If the port appears to
  require one, that is a stop-and-report.
- **No path-policy change.** DC-72 covers it; `COM0`/`LPT0` and anything else is a separate increment.
- **Green CI on all three platforms** before either stage merges, per the standing rule. Both stages
  touch filesystem-backed state throughout.

## 5. Reporting

Report to `.git-exclude/review-request/`, as usual — a plain `.md` is fine for an investigation.
Answer the six in order, and where the honest answer is "I could not determine this," say that rather
than reasoning to a conclusion the platform documentation does not support.

Findings you turn up that are outside DC-87's scope go in the report too; I register them in
`FINDINGS.md`. Reporting a finding and registering it are two different acts, and the second is mine.

## 6. Sequencing

Nothing else is live. DC-85 merged 2026-08-10 at `596edfc` after a green macOS run, and its two
follow-ups (§6.1 parse-before-authenticate ordering, §6.2 the revocation constraint) are registered and
unowned — **neither is yours to pick up here.**
