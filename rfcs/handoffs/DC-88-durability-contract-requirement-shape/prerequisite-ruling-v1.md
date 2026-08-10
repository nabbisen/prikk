# DC-88 — Prerequisite Investigation Ruling v1

**Reviewing:** `.git-exclude/review-request/prikk-dc-88-prerequisite-questions-v1.md`.

**First: this report sat unread for over an hour while I processed DC-89 and DC-90.** The priority
increment's answers were waiting and I was working lower-priority items. That is my scheduling failure,
not a queueing accident, and the developer was right to say so.

**Investigation accepted, every claim independently verified.** The finding is the most consequential of
this cycle, and it does two things at once: it collapses DC-88 to almost nothing, and it shows that
**two of my own rulings were priced on a false premise.** §3 and §4 are those corrections.

## 1. Verified, not taken

I re-derived each load-bearing claim from source:

- **`sync_directory_required` has exactly two callers**, both in `worktree.rs` (`:146`, `:195`), both in
  the already-matches branch immediately before returning `Unchanged`. Nothing else in the workspace
  calls it.
- **`atomic_replace` bundles `directory.sync()`** (`linux.rs:45`), **`durable_append` bundles it**
  (`:58`), **`promote` bundles two** — destination then source (`:150`, `:152`).
- **DC-38's three durability-bearing steps reach none of them.** Step 3 → `pointer.rs:51`
  `write_file_atomically`; step 5 → `refs.rs:306` `promote_file_required`; step 6 → `log.rs:64`/`:66`
  `append_file_required`. **`durable_directory_entry` is not in DC-38's path at all.**
- `durable_directory_entry`'s Linux body is exactly `open_existing_directory_required(root, relative)`
  + `directory.sync()` (`linux.rs:170-174`), confirming §4.2's byte-for-byte argument.

The §4.3 scope limit — first-creation paths traced only for the objects DC-38 names, not the WAL, trust
store, or active-ref metadata — is stated honestly and I accept it as scoped. §5 says where it lands.

## 2. What DC-88 now is

A restatement of one method so it takes the file path and computes the parent itself, plus two one-line
caller edits. Same two primitives, same `fsync`/`fcntl_fullfsync` on the same fd. **Cleared to design
and implement** — there is no second prerequisite round.

The RFC's §1 question is answered as posed: `durable_directory_entry` states a primitive, not a
requirement, and no caller anywhere wants the directory-scoped batching its guarantee describes. The two
real callers want something narrower still — *confirm this one entry is durable* — which is worth
saying in the restated guarantee rather than approximating.

## 3. Correction: DC-88 never blocked Stage 2, and I told the owner it did

When I proposed DC-88 I attached a scope trade — *accepting this blocks DC-87 Stage 2 until it lands* —
and the owner accepted the slower path on that basis. **That trade was mispriced.** It rested on my
assumption that `durable_directory_entry` sat in DC-38's ref-publication path. It does not, and one
grep would have told me before I asked the owner to pay for it.

**Consequences, applied now:**

- **DC-88 does not block DC-87 Stage 2.** The two are orthogonal in the code.
- **DC-87 Stage 1's seam refactor is released from hold.** I paused it because DC-88 might change what
  the Windows authority type needs to be capable of. It cannot — the restatement touches no type, no
  method set, and no `MutationRoot`. That reasoning was downstream of the same wrong premise.

## 4. Correction: the Windows blocker is real, and it is somewhere else

**The most important thing in this ruling, because the opposite inference is available and wrong.**

"DC-38 does not call `durable_directory_entry`" does **not** mean DC-38 is satisfiable on Windows. It
means the obstacle was never in that method. It is in the three that DC-38 does call, each of which
bundles a **directory sync** as part of its own guarantee — and Windows has no directory sync. The
problem has not shrunk; it has been correctly located, which is worth more.

My narrow-round diagnosis was right on the mechanism — a crash between a non-durable pointer promotion
and a durable log append yields the ahead-log state DC-38 exists to prevent — and wrong about which
method to fix. The two-slot sketch in DC-88 §3 was aimed at the right *problem* and filed under the
wrong *method*: it is an alternative way to satisfy a **transition** guarantee, not a way to restate
`durable_directory_entry`.

**And here is the part that genuinely helps, which this investigation earned:** the contract is already
requirement-shaped where it counts. `atomic_replace` says *replace this file's content atomically,
durably* — it does not say "and fsync the parent." The directory sync is an implementation detail inside
`LinuxDurability`. **So a Windows implementor is already free to satisfy those three guarantees any way
it can, with no contract change at all.** DC-76 got this right everywhere except the one method DC-88
now fixes.

The Windows durability question is therefore well-formed for the first time: **can `atomic_replace`,
`promote`, and `durable_append` be satisfied on a platform with no directory durability?** It belongs to
Stage 2's design, it needs no contract amendment to attempt, and the two-slot shape is one candidate
answer to it.

## 5. Amendments

**5.1 — DC-88 §3's candidate shape is misattributed.** It describes an alternative implementation of a
transition guarantee, not a restatement of `durable_directory_entry`. Left in DC-88 it invites someone
to build a two-slot record for a method with two callers that do not need one. Amended: §3 is marked as
belonging to DC-87 Stage 2's inputs, and DC-88's own shape is the parameter restatement.

**5.2 — §4.3's untraced first-creation paths become a Stage 2 input.** The WAL's first file, the trust
store, and active-ref metadata were not individually traced. That does not affect the restatement, which
depends on none of them. It does affect a Windows implementor, which must satisfy every first-creation
path, not only the ones DC-38 names. Recorded so it is asked for once rather than discovered late.

## 6. Standing after this ruling

- **DC-88:** cleared to implement. Small. Green CI before merge; it touches filesystem-backed state, so
  the three-platform rule binds it.
- **DC-87 Stage 1 seam refactor:** unblocked, available now.
- **DC-87 Stage 2:** blocked on **DC-90** landing (before any `unsafe`) and on its own design answering
  §4's question. **No longer blocked on DC-88.**
- **DC-90:** accepted, its investigation reported separately.
