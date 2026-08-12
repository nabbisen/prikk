# RFC 101 First-Appearance Durability — Prerequisite Handoff v1

**Cleared to answer §5's six prerequisites only.** Accepted 2026-08-12,
`rfcs/accepted/101-first-appearance-durability.md`. **No design, no implementation, no production
code.**

## 1. What this is

The owner has directed that **Windows mutation reach parity with Linux**. Not a weaker invariant on
Windows, not a documented gap — parity.

DC-87 Stage 2 established the obstacle: Windows has no primitive that makes a **newly-created name**
durable. Updating an existing file is durable; a name's first appearance is not. DC-38 step 6 (log
append, existing file) is achievable; step 5 (pointer promotion, new name) is not, and a crash between
them reproduces the ahead-log state DC-38 exists to forbid.

DC-91 then closed off the obvious fix: **no per-ref file shape avoids this at ref creation**, because a
new ref needs its first log record in the same transaction. The pointer's shape was never the obstacle.

**So the hypothesis here is different in kind:** route every durability-bearing transition through a
name that *already exists*, and make the per-ref pointer and log files **replayable consequences**
rather than durable steps.

## 2. Three facts already verified — do not re-derive, but do challenge

Verified in the code 2026-08-12:

- `layout.rs:161` — the active WAL is `active/default/queue.wal`. A **fixed** path, not a per-session
  generated name.
- `active.rs:147` — `finish_active_publication_cleanup` calls `Wal::truncate_empty()`. The WAL is
  **truncated, not deleted**.
- `active.rs:137` — only the ref-name metadata file is removed on cleanup.

**These are load-bearing for the whole RFC and they are mine, not yours.** I asserted the opposite the
day before — that WAL cleanup deletes the file and would reintroduce first-appearance — and corrected
it on reading the code. **If any of the three is wrong, the RFC's direction is wrong, and saying so is
the most valuable thing this round could produce.** Check them first.

## 3. Where to start, and the order matters

**§5.1 first — is the WAL created at `init`, or lazily on first append?**

It is the cheapest question and it gates the rest. If creation is lazy, the very first mutation in a
repository's life still has a first-appearance requirement, and the question becomes whether creation
can move to `init` without changing any stated guarantee. Answer it **from the code path**, not from
the shape of the layout API — `Wal::for_layout` only computes a path.

**§5.2 second, and this is the round's real work — enumerate every durability-bearing transition that
today requires a name that did not previously exist.**

**Derive this set independently.** Do not take DC-87 Stage 2's list, do not take DC-91's, and do not
take mine. DC-89 exists because a fix scoped to one file left eight more instances of the same claim
standing; the same failure here would produce a design that closes most of the holes and ships as
though it closed all of them.

Then §5.3 per transition: does routing through a fixed-name record eliminate the requirement, and
**what replays the consequence?** A transition that cannot be routed is a stop-and-report — say so and
stop, exactly as DC-87 Stage 2's §3.4 did. That was the right call then and it is the right call here.

§5.4 (`verify`/`doctor` state classes for a crash mid-replay), §5.5 (Windows primitives and the G1–G9
mapping), and §5.6 (proof-surface cost) follow.

## 4. Constraints you do not have authority to trade

1. **One publication mechanism across all platforms.** Two mechanisms is worse than not shipping
   Windows mutation. If the answer requires divergence, that is a stop-and-report.
2. **No conversion of format-2's *rejection* of the ahead-log state into *recovery*.** That is DC-87
   Stage 2's option 2, which I have recommended against twice and the owner has not taken. It is not
   reachable by the back door of "replay handles it."
3. **Recoverability does not regress below today's ceiling** — DC-41 Stage 1's audited 24/24 reachable
   states. A new design starts *unproven*, not merely unequal. State that honestly as a cost; do not
   argue it away.
4. **B′ adoption semantics unchanged**, and **object-trust/ref-authority stay separate** (DC-78 §D2).

## 5. On `unsafe`, since §5.5 will reach it

The owner's standing ruling: **`unsafe` is permitted under control, with safety and maintainability
preserved**, and formal verification (Verus or equivalent) is available if warranted. So "this needs a
raw Win32 call" is not by itself a blocker — but each use is justified individually, and
`handoffs/DC-87-windows-mutation/unsafe-surface-analysis-v1.md` is the document to extend rather than
restart.

## 6. A "no" is a complete outcome

If §5.2's enumeration turns up a transition that no fixed-name routing can reach, that ends the RFC and
returns DC-87 Stage 2 to the owner's option 2 / option 3 choice. **That is a successful round.** Do not
soften a stop-and-report into a partial design; DC-87 Stage 2 got this right and it is why the picture
is now clear enough to attempt parity at all.

## 7. Sequencing against DC-95

DC-95 Stage 1 round 9 is in flight. **This does not preempt it.** Which runs first, or whether they
interleave, is a scheduling question for the owner — raise it rather than assuming.
