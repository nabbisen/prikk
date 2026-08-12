# RFC 102 Container-Based Durability — Prerequisite Handoff v1

**Cleared to answer §6's six prerequisites only.** Accepted 2026-08-12,
`rfcs/accepted/102-container-based-durability.md`. **No design, no implementation, no production code.**

## 1. Read RFC 101's closure first

`rfcs/archive/101-first-appearance-durability.md`, and the two rulings in
`rfcs/handoffs/101-first-appearance-durability/`.

RFC 101 was accepted and closed on the same day because **its own prerequisite trace disproved its
problem statement.** That is the model for this round, not a cautionary tale — §5.2's instruction to
derive the transition set independently is the only reason the error surfaced, and it surfaced against
the architect's document rather than the dev team's.

**This RFC's hypothesis is exactly as unproven as 101's was.** Treat it that way.

## 2. What is settled, so this round does not re-derive it

1. **No Windows primitive provides new-name durability** — documented, undocumented, or
   reverse-engineered. DC-87 Stage 2 covered the Win32 surface; RFC 101 §5.5 added TxF and `$LogFile`.
2. **Transactional NTFS is ruled unusable** — not because it does not work, but because its withdrawal
   would silently void the guarantee rather than break detectably.
3. **The problem is the storage model, not ref publication.** Content-addressing makes the filename the
   hash, so every object write creates a new name.
4. **§5.2's fifteen-transition table and 31-site call index are this round's primary input.**

## 3. Where to start, and the order matters

**§6.1 first — what have comparable systems done, and what did it cost them?**

Packed and container storage in content-addressed and version-control systems: what gets packed, what
stays loose, how durability is claimed, and **specifically how each behaves on Windows.**

It is first because it is the cheapest and it can change the owner's decision before any engineering is
spent. **If the field universally accepts weaker Windows durability, that is a finding the owner needs
before authorising a storage redesign** — it would mean prikk is holding a bar nobody else holds, which
is a legitimate position but should be taken knowingly.

Report what you find, including if it undercuts this RFC. A finding that makes the owner reconsider is
worth more than one that confirms the plan.

**§6.2 second, and it is the round's real work — re-derive §5.2's fifteen transitions against a
container model.**

**Do not assume the table transfers.** It was derived against today's one-file-per-object layout; a
container changes which transitions exist at all. Derive independently, the same way §5.2 was derived,
and treat divergence from the old table as a result rather than an error.

A transition that cannot leave the durability path is a **stop-and-report**.

Then §6.3 (the bounded fixed-name set), §6.4 (read-path and concurrency), §6.5 (the worktree question),
§6.6 (cost).

## 4. On §6.5, because it is the part most likely to be waved through

The worktree cannot be containerized. The RFC proposes a fixed-name unclean-shutdown marker so prikk
refuses to infer deletion from absence.

**That sketch is the architect's and it has not been checked against the commit-authoring code.**
Answer it from `worktree_patch/node_authoring.rs` and the T12 finding, not from §4's paragraph. If the
marker is unsound, or if absence-as-deletion is load-bearing somewhere the sketch does not anticipate,
that is the finding.

**T12 is the most serious thing in the current `FINDINGS.md`** — prikk signing a deletion the user never
made. It is controlled on POSIX and would be live on Windows. Do not let it ride on an unexamined
paragraph.

## 5. Constraints you cannot trade

1. **One storage mechanism across all platforms.** A Windows-only container is worse than not shipping
   Windows mutation. Divergence is a stop-and-report.
2. **B′ adoption semantics unchanged** — verbatim bytes, same `ObjectId`, same author signature. Object
   identity must survive any container format.
3. **No conversion of format-2's *rejection* of the ahead-log state into *recovery*.**
4. **Recoverability does not regress below DC-41 Stage 1's audited 24/24**, and the audit is re-earned
   rather than assumed.
5. **A migration must exist** for repositories already in the current format. Out of scope for §6, in
   scope for the design, and §6.6 should cost it.

## 6. A stop-and-report is a complete outcome

RFC 101 closed on one and it was the right result — it produced a correct problem statement, a mapped
new-name surface, and three findings, none of which existed before. **If this RFC ends the same way, say
so plainly and do not soften it into a partial design.**

## 7. Sequencing

DC-95 Stage 1 round 9 is submitted and under review. **This does not preempt it.** Raise sequencing
rather than assuming it, as you did last time — that question produced a better answer than any of the
options offered.
