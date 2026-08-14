# RFC 103, Increment B — Handoff v1

**Authorized by the project owner 2026-08-13.** Increment A merged at `9b75656` after a green
three-platform run.

**Design:** `design-v1.md` §4 — *"collapse the plumbing, optional and separately decided. Removing it
entirely is a wide mechanical diff with no behavioural content."*

## 1. Step 0 first — and this one may end the increment

**Before any production code: establish whether Increment B should happen at all.**

`RepositoryFormat` is now a single-variant enum threaded through many signatures, and
`require_current_format` has **25 call sites**. Nothing constructs a `RepositoryLayout` outside
`open`/`init` — so after Increment A, that function can no longer reject anything.

**The question is not "can it be removed" but "should it be."**

Round 6's standing ruling: *"unreachable today is not unreachable by design."* It kept three provably
unreachable checks for exactly this reason — their unreachability follows from a property of today's
code, not a stated invariant. **`require_current_format`'s 25 call sites are a defence-in-depth layer
whose only current guarantor is that open-time rejection is complete.** Removing them makes open-time
rejection the *sole* gate, permanently.

**Report, from the code:**

1. **What does `require_current_format` actually do** beyond the format comparison? If anything, it is
   not plumbing and Increment B does not touch it.
2. **Is open-time rejection genuinely the only entry?** I found no `RepositoryLayout::new` caller outside
   the type, but I looked once and my counts have been narrower than the code five times this month.
3. **What would a future caller have to do to bypass it** — and would anything catch them?

**A recommendation to abandon Increment B is a complete outcome**, and the design already says it "may
never be worth doing." Say so plainly if that is what you find. **This increment exists to delete code
that is not doing anything; if it turns out to be doing something, there is nothing here worth having.**

## 2. If it proceeds

Mechanical only: collapse the single-variant enum and its threading. **No behavioural change**, and
therefore no test whose *assertion* changes — only signatures.

**Any test that must change its assertion is a signal you have left plumbing and entered behaviour.**
Stop and report rather than adjusting the assertion.

## 3. Out of scope, explicitly

The refused reconstruction subsystem — `RefRecoveryRepair`, `reconstruct_missing_ref_from_log`,
`DoctorRepairOptions::reconstruct_main_ref`, `--repair-main-ref`. Ruled out of RFC 103 entirely on
2026-08-13: its refusal has nothing to do with format-1, and deleting user-facing surface because a
string mentions format-1 is a different decision. **`RefRecoveryCandidate`/`recoverable_missing_ref` is
live format-2 machinery** and is not touched either.

## 4. Acceptance criteria

1. **Step 0 reported and ruled before any production code.**
2. If it proceeds: **no test assertion changes** — signatures only.
3. Full gate set, plus **green three-platform CI**.

## 5. Standing

RFC 102 Stage 1 is also open; neither track blocks the other.
