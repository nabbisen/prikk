# RFC 101 §5.1–§5.3 — Prerequisite Ruling v1

**Reviewing:** `.git-exclude/review-request/prikk-rfc101-prerequisite-5.1-5.3-v1.md`.

**Investigation accepted, and it has broken the RFC's hypothesis as written.** T2 is decisive. The
finding is sharper than the report states, the error it exposes is mine, and the consequence is a
reordering of §5 rather than an immediate close. All three are ruled below.

## 1. Verified

- **T2's premise.** `object_store.rs:117-118` — `object_path(envelope.object_type, id)` where
  `id = envelope.object_id()`. **The name *is* the content hash.** There is never an existing file to
  update, so every object not already present is a new name. Confirmed.
- **T15.** `prikk-cli/src/bundle.rs:38` is a bare `std::fs::write`. Their qualification is right: the
  target is an operator path outside the repository, so this is a claim-accuracy issue, not a defect.
- **T12's shape.** `worktree.rs:151` writes only after the mode/content checks fall through.
- **§5.1's read-side unification.** `read_bytes()` treating `NotFound` as `Ok(None)` is what makes
  pre-creating `queue.wal` at `init` behaviour-neutral. The trace of every reader, rather than the two
  they expected, is the right instinct and it is what makes the conclusion usable.

**They checked the handoff's three facts first and reported no correction.** I asked for that because
the facts were mine and I had gotten the adjacent claim wrong the day before. Confirming rather than
flipping them is a real result, not a null one.

## 2. T2 is worse than the report states

The report says T2 "doesn't have a routing story in what's written." Correct, and incomplete.

**Routing ref publication through the WAL while object writes remain new names does not leave T2
unaddressed — it creates a new asymmetry.** A ref pointer materialized from a durable WAL record would
reference objects whose bytes never became durable. The result is a repository that durably asserts a
history it cannot produce.

That is **the same failure DC-38 exists to prevent**, relocated from the pointer/log pair to the
pointer/object pair. Today the two halves fail together on Windows, so a crash loses both and leaves
the prior consistent state. The fix as scoped would make one half durable and not the other.

**A partial application of this RFC's hypothesis is worse than not applying it.** That must be on the
record before anyone designs from §5.3's "routes cleanly" column.

## 3. The RFC's problem statement is wrong, and that is my error

RFC 101 §1 frames the problem as DC-38's step 5 versus step 6. **That is a symptom.** The disease is
that **prikk is a content-addressed store, so every mutation creates new names** — objects always, refs
at creation, worktree files always. First-appearance is not a property of ref publication. It is a
property of the storage model.

I inherited the ref-publication framing from DC-87 Stage 2 and DC-91 and did not re-derive it when
writing §1. **That is precisely the error I have twice corrected the dev team for** — DC-87 §4 scoped
to one file when the same claim lived in eight more, and DC-93 asserting "nothing invokes it" without
grepping. §5.2's instruction to derive the set independently is the only reason it surfaced here, and
it surfaced against my own document.

## 4. The ruling: not a close, but one question now dominates

**The hypothesis as written is dead.** Do not design from it, and do not attempt routing stories for
T3, T8, T11, T12 or T9/T10 — all are downstream of a question that now dwarfs them:

> **Does Windows genuinely lack new-name durability, or does it lack a *documented guarantee* of it?**

That is §5.5, and it is no longer one prerequisite among six. It decides everything:

- **If genuinely absent** — parity is impossible for a content-addressed store, not merely hard. That
  is a clean, final answer, it closes RFC 101 with a valuable negative result, and it returns DC-87
  Stage 2 to the owner's option 2 / option 3 choice with the cost of each finally known.
- **If it exists at all** — including as an NTFS metadata-journal property Microsoft declines to
  document — then the entire first-appearance framing dissolves and neither this RFC nor WAL routing is
  needed for the reasons it was written.

**Either outcome makes the rest of §5 moot.** Running anything else first spends effort on a
superstructure whose foundation is unsettled.

**Note the distinction carefully, because I sharpened the same one in the DC-87 Stage 2 ruling §2.1**:
`FILE_RENAME_INFO`'s `Flags` field is documented while its *values* are not. "Undocumented" and
"absent" are different findings, and only one of them ends the RFC. If §5.5 returns "works in practice,
unguaranteed on paper," that is a **risk-acceptance question for the owner**, not an engineering
verdict — and the owner's standing position is that security is prioritised over speed.

## 5. Standing changes

1. **§5.5 runs next, alone.** §5.4 and §5.6 are suspended — both presuppose a design.
2. **DC-95 Stage 1 round 9 resumes now, in parallel.** My sequencing ruling paused it because ten of
   eleven remaining rows sit in machinery RFC 101 might restructure. **That risk has receded**: under
   either §5.5 outcome, ref publication is now less likely to be restructured than when I ruled. The
   dependency that justified the pause is gone.
3. **§5.2's transition table is retained as a durable artifact** regardless of what happens to this
   RFC. Fifteen transitions with durability-bearing status and a 31-site call index is the map of
   prikk's new-name surface, and nothing else in the project has one. It belongs in the code's
   documentation before this RFC closes either way — the same ruling round 7 made of DC-95's inventory.
4. **Three findings registered in `FINDINGS.md` independent of RFC 101**: T12's silent signed deletion,
   T11's `verify` gap on `refs/received/`, and T15's contract bypass.

## 6. On not declaring the stop-and-report themselves

They reported three candidate stop-and-reports and declined to pick one, on the grounds that which is
disqualifying is a cost/benefit judgment about the RFC. **Correct, and it is the same restraint DC-87
Stage 2 showed in not reaching for format-1 ahead-log recovery and not reviving DC-88 §3's sketch
unilaterally.**

Reporting all three rather than stopping at the first is what let §3's generalisation be seen. Had they
stopped at T2, the conclusion would have been "objects are a gap"; because they carried on to T12 and
T9/T10, the pattern — *new names are everywhere, not in ref publication* — became visible.

## 7. Open questions carried forward

Their four are adopted as stated. **Their question 1 is promoted**: whether Windows' missing primitive
applies to *directory* names as to file names is not a footnote — T2's per-prefix object directories
and T12's worktree directories both depend on it, and it folds naturally into §5.5.
