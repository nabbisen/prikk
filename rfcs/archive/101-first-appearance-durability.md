# RFC (archived) - 101 First-Appearance Durability

**Status.** **CLOSED 2026-08-12 with a negative result, and SUPERSEDED by RFC 102.** Accepted
2026-08-12 on the owner's Windows-parity direction; closed the same day after §5.1–§5.3 and §5.5.
**No code was produced.** §6's own standing was that a stop-and-report is a successful outcome, and this
is one.

**What it established, and why it was worth running:**

1. **§1's problem statement was wrong.** The obstacle is not DC-38's step 5 versus step 6 — that is a
   symptom. **prikk is content-addressed, so every object write creates a new name**, and the problem
   belongs to the storage model. Found by §5.2's independently derived transition trace (T2), against
   this document.
2. **The hypothesis would have made Windows worse.** Routing ref publication through the WAL while
   object writes remained new names would produce a durable ref pointing at a non-durable object — the
   DC-38 failure relocated, not removed.
3. **No Windows primitive provides new-name durability**, and **Transactional NTFS did and is being
   withdrawn** — ruled unusable because its removal would silently void the guarantee rather than break
   detectably (§5.5 ruling §4).
4. **§5.2's fifteen-transition table and 31-site call index survive this closure** as the map of prikk's
   new-name surface, and are RFC 102's primary input.
5. **Three `FINDINGS.md` rows survive independently** — T12's silent signed deletion, T11's `verify` gap
   on `refs/received/`, and T15's contract bypass.

**Successor.** [RFC 102](../accepted/102-container-based-durability.md) — container-based durability,
which takes the storage model as the unit of change. **Windows read-only is a staging state, not a
verdict.**

**Everything below is preserved as written and its §1 and §3 are known wrong.** Read §2 and §5 for what
still holds. This supersedes the scope of DC-91's §5 recommendation — see §2.3.
**Independence.** Author-reviewed — the standing ceiling.
**Arises from.** DC-87 Stage 2's transition-durability ruling (2026-08-11) and DC-91's evaluation
ruling §3, which together established that the obstacle is not the ref pointer's shape.
**Target.** Owner's call. 0.20.0 moves if this runs.
**First RFC under RFC 100's naming rule.**

## 1. The problem, stated once

Making a **newly-created name** durable requires an `fsync` on the parent directory. DC-87 Stage 2
established that Windows offers no equivalent: **updating an existing file is durable; a new name's
first appearance is not.**

DC-38's ref publication is a seven-step state machine whose invariant is *"format-2 publication never
permits an ahead log."* Step 6 appends to the ref log — an existing file, achievable. Step 5 promotes
the pointer — a new name, not achievable. A crash between them reproduces exactly the ahead-log state
DC-38 exists to forbid.

**The asymmetry is what is fatal.** If neither step were durable there would be no ordering hazard.

## 2. What is already settled, so this increment does not re-derive it

### 2.1 The pointer's shape is not the obstacle

DC-91 evaluated a fixed-name, slot-based pointer record. It makes seals to **existing** refs fully
Windows-achievable and does **not** unblock new branch or tag creation: a new ref requires its first log
record in the same transaction, and that log file is itself a new name. Branch and tag creation are
ordinary recurring operations since DC-60, DC-61 and DC-63 — not an `init`-only event.

### 2.2 The generalisation, from DC-91's ruling §3

> **Any design that keeps per-ref files has a first-appearance problem at ref creation.**

So the question is not which per-ref shape avoids it. It is whether the durability-bearing transition
can be moved off per-ref names entirely.

### 2.3 What this supersedes, and what it does not

DC-91 §5 recommended **against** restructuring ref publication. That recommendation was scoped to
restructuring *for a partial Windows payoff* — buying existing-ref seals while leaving ref creation
blocked. It does not extend to a general solution that reaches parity, which is a different
proposition and was named in §5's own list of what remained the owner's.

**Still standing from that ruling, and load-bearing here:** recoverability today sits at an audited
24/24 reachable states (DC-41 Stage 1). Any new design starts *unproven*, not merely unequal, and must
re-earn that audit.

## 3. The direction to evaluate

**Route every durability-bearing transition through a name that already exists.**

Three facts verified in the code on 2026-08-12:

- `layout.rs:161` — the active WAL is `active/default/queue.wal`. A **fixed path**, not a per-session
  generated name.
- `active.rs:147` — `finish_active_publication_cleanup` calls `Wal::truncate_empty()`. The WAL is
  **truncated, not deleted**. Once created, the file persists.
- `active.rs:137` — only the ref-name metadata file is removed on cleanup.

**Correction to the record.** I stated on 2026-08-12 that today's WAL cleanup would reintroduce
first-appearance for the WAL itself. **That is wrong** — cleanup truncates. The direction is therefore
better-founded than when I described it, and this RFC exists partly because the correction moves the
cost estimate.

**The hypothesis to test:** if ref creation's durability-bearing step becomes an append to that
already-existing WAL, and the per-ref pointer and log files become **replayable consequences** rather
than durability-bearing steps, then no new name sits on the durable path and DC-38's invariant becomes
platform-independent.

**This is a hypothesis, not a design.** §5 is the work of finding out whether it survives.

## 4. Non-negotiable constraints

1. **One publication mechanism across all platforms.** Two mechanisms is a worse outcome than not
   shipping Windows mutation.
2. **DC-38's invariant holds identically on Linux, macOS and Windows** — stated as a property, not as a
   per-platform table.
3. **No conversion of format-2's *rejection* of the ahead-log state into *recovery*.** That is DC-87
   Stage 2's option 2, recommended against twice, and it is not reachable by the back door.
4. **B′ adoption semantics unchanged** — a merge seals the other side's patches verbatim: same bytes,
   same `ObjectId`, same author signature.
5. **Object-trust and ref-authority stay separate** (DC-78 §D2).
6. **Recoverability does not regress below today's audited ceiling**, and the audit is re-earned rather
   than assumed.

## 5. Blocking prerequisites

These precede any design. Each is an investigation with a written answer.

1. **Is the WAL created at `init` or lazily on first append?** If lazily, can creation move to `init`
   without changing any stated guarantee? Answer from the code, not from the layout API's shape.
2. **Enumerate every durability-bearing transition that today requires a name that did not previously
   exist.** The complete set, **derived independently** — not taken from DC-87 Stage 2's or DC-91's
   lists. DC-89 exists because a scoped-to-one-file fix left eight more instances standing; that lesson
   applies directly.
3. **For each transition in that set:** does routing through a fixed-name record eliminate the
   first-appearance requirement, and what replays the consequence? A transition that cannot be routed
   is a stop-and-report, as DC-87 Stage 2's §3.4 was.
4. **What must `verify` and `doctor` say about a repository crashed mid-replay?** The new state classes
   must be enumerated **before** design, not discovered during it.
5. **Which Windows primitives serve the fixed-name path**, and does a `WindowsDurability` that only ever
   updates existing files satisfy G1–G9? Map guarantee by guarantee, per DC-76's guarantee-named
   contract. Note the owner's standing ruling: `unsafe` is permitted under control with safety and
   maintainability preserved, and formal verification (Verus or equivalent) is available if needed.
6. **Re-measure the proof surface that must be re-earned.** DC-91 put ref publication at ~1,335 lines
   and 23 tests by my count, ~1,387/24 by the dev team's; neither figure was reproduced by the other.

## 6. Acceptance criteria

1. **Parity is stated as a property of the design**, not as a list of platforms that happen to pass.
2. **A negative control**: disable the replay step, demonstrate the specific failure `verify` reports,
   restore, and confirm no residual diff. Per DC-95's standing method — a passing suite is not evidence
   that a check is load-bearing.
3. **Green three-platform CI**, macOS included, per the standing rule for filesystem-backed state.
4. **The DC-41-grade recoverability audit re-earned** at the new design's own state count, with
   reachability stated rather than assumed.
5. **`unsafe` surface, if any, enumerated and justified individually** — extending
   `DC-87-windows-mutation/unsafe-surface-analysis-v1.md` rather than restarting it.

## 7. Non-goals

- **DC-91's slot-record detectability gain.** It has standalone POSIX merit and should be taken on that
  basis if wanted. It is not a Windows unblock and must never be sold as one.
- **G1 path anchoring on Windows.** DC-87 Stage 2 established there is no Win32 component-by-component
  no-follow walk. Out of scope here; this RFC is about transition durability only.
- **Windows read-only support**, which works today and is CI-gated throughout.
- **Changing what a ref log is.** It is DC-38's append-only audit trail; making it slot-shaped means
  discarding history, which is a guarantee change and not on the table.

## 8. The cost, stated plainly

This touches the most safety-critical machinery in the product. Beyond the prerequisites: the DC-34 and
DC-38 state machines, `doctor` and `verify` state derivation, the durability contract's platform layer,
and the recoverability audit. It is larger than anything currently in flight, and the 0.20.0 target
moves.

**It is proposed because the owner asked for parity, and parity is not reachable by any cheaper route
that has been evaluated.** If a prerequisite returns a stop-and-report, that is a good outcome and ends
the RFC — the same standing DC-91 was given.
