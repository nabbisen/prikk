# RFC (proposed) - DC-71 Non-Linux Build Conformance

**Status.** **Accepted by the project owner on 2026-08-04**, together with the §3 ruling below.
Implementation may begin; handoff at
`handoffs/DC-71-non-linux-build-conformance/implementation-handoff-v1.md`.

> **Owner ruling, 2026-08-04 — §3 question 1: portable read-only is a REQUIREMENT, not an aspiration.**
> Asked as *"should a macOS or Windows user be able to verify a prikk repository?"*; answered **"Yes.
> Cross platform support is required."**
>
> **Reading applied, stated so it can be corrected:** this settles **read-only portability**, which is
> what was asked. **DC-37's Linux-only *mutation* boundary is left standing** — it exists for filesystem
> durability guarantees, and reopening it is a materially larger question than this increment. If the
> owner meant cross-platform *mutation* as well, that is a separate ruling and a separate increment.
**Authored by** the architect.
**Independence.** Author-reviewed — the standing ceiling.
**Arises from.** DC-70's trial builds (2026-08-03), which found `x86_64-pc-windows-gnu` does not compile.
**Requirement.** **DC-37**'s stated design, which this contradicts. Also closes the long-standing
`MILESTONES.md` row *"Public portability claim exceeds Linux-only mutation support."*

## 1. The defect

**`prikk-store` does not compile at all off Linux.** Not "cannot mutate" — will not build.

`fsutil/anchored/{immutable,regular,read}.rs` import helpers that are `#[cfg(target_os = "linux")]`-gated
at their definition sites in `anchored.rs`, but the imports themselves are **inconsistently gated**:
`read.rs:11-12` gates one import, others are ungated. So someone began gating and did not finish.

**DC-37 intends Linux-only *mutation* with read-only commands portable.** The code does not meet that. This
is a defect against the project's own stated design, not a scope decision.

Verified by trial build for `x86_64-pc-windows-gnu`. **macOS is untested and expected to fail identically**
— the cause is an ungated module, not anything Windows-specific.

## 2. The reason it drifted, which is the more important half

**Nothing builds a non-Linux target.** `ci.yml` builds and tests on Linux only, so cfg-gating can rot with
no signal. The gating is *partial*, which is what silent drift looks like — a full absence would suggest a
decision; a half-finished one suggests nobody could see it.

**A fix that repairs the three files and adds no build check will rot again**, and next time may be found by
a user rather than a trial build. This is the same shape as DC-67's finding: the gates were aimed
somewhere other than where the defect was.

## 3. What must be established before designing — blocking

| Question | Why it blocks |
|---|---|
| **Is portable read-only a requirement, or an aspiration?** | DC-37 *states* it. If it is genuinely wanted, this is a defect to fix. If not, DC-37 and the README should say "Linux-only" plainly and this increment becomes a documentation correction. **The design must not stay ambiguous** — that ambiguity is what let the claim ship |
| Which targets must build? | macOS and Windows are different amounts of work. Untested today |
| Is `prikk-store` the only crate affected? | The finding names it; nothing has checked `prikk-cli`, `prikk-replay`, or the others |
| What does "read-only command" mean concretely? | `verify`, `log`, `doctor`, `checkout --plan-only`? The set has never been enumerated, and it determines how much must compile |

**The first question is the owner's**, and it changes what this increment is. The rest are answerable by
reading and trial builds.

## 4. Acceptance criteria

1. §3's four questions answered and reported before a fix is designed; question 1 ruled by the owner.
2. **If portable read-only is wanted:** the named targets compile, and the read-only command set runs on at
   least one non-Linux target — demonstrated, not asserted.
3. **A CI job builds each supported non-Linux target**, so this cannot silently rot again. Without this,
   criterion 2 is a snapshot, not a property.
4. **If portable read-only is *not* wanted:** DC-37, the README, and every public portability claim are
   corrected to say Linux-only plainly, and `MILESTONES.md`'s release-claim-mismatch row closes.
5. Mutation remains Linux-only either way. **DC-37's mutation boundary is not reopened by this increment.**
6. Full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9, plus test counts before and after.

**Criterion 3 is the one that matters.** The defect is cheap; the absence of a signal is why it existed.

## 5. Non-goals

- Non-Linux **mutation**. DC-37's boundary stands.
- Publishing non-Linux binaries. That is DC-70's surface, and it depends on this landing first.
- Windows/macOS filesystem durability semantics. Out of scope; read-only paths do not need them.
