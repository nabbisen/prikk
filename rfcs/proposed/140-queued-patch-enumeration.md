# RFC 140 — Which patches, not how many

**Status.** **PROPOSED, 2026-09-06.** Opened after the project owner authorized the architect's
recommendation on stikk letter 003, which included queued-patch enumeration as part 3 of one round.

**This RFC exists because the architect's recommendation understated the work, and the correction is
the substance of §3.** The recommendation called it *"a presentation of state already read"* and said
it *"needs no new command"*. The second half is true. **The first half is false**, and the reason is a
property of prikk's own operation model rather than an oversight in scoping. Parts 1 and 2 of that
round were unaffected and are already handed off
(`rfcs/handoffs/132-error-taxonomy-structure/six-preconditions-and-the-broad-arm-handoff-v1.md`).

**Author-review independence.** The architect wrote this RFC and is also its only reviewer — the
standing gap recorded on every architect-authored design here. Compensated at implementation review.

**Tracks.** A read surface. **No change to what is stored, sealed, or verified.**

---

## 1. The request

From the stikk project (a TUI/GUI front-end driving prikk through the public CLI), letter 003 §2,
ranked second of their three standing asks and described as *"the single request that would most
improve the honesty of a screen we are building right now"*:

> **What we can do now:** tell a user that a seal will freeze *N* patches.
> **What we cannot do:** tell them **which**.

Their reasoning is the part worth keeping: their seal ceremony is designed around informed consent,
and *"a count is not information about what; it is information about how much."*

**This is ours to want independently of them.** `prikk status` has the same defect for a user at the
terminal: before the most irreversible act the tool offers, it says how much and not what.

## 2. What `status` reports today, read at `ddca5db`

`run_status` (`crates/prikk-cli/src/main.rs:418-472`) prints, as prose only:

- the repository path;
- `active WAL records: N` and `trailing partial WAL bytes: N`;
- `heads/main RefState: <id>` or `<not published>`;
- `queued patches: N targeting <ref>` — DC-66 criterion 7, the count **and** the owning ref, kept
  distinct from the raw record count;
- a warn or hard-limit line at DC-57's thresholds.

**Two facts constrain any change here.**

- **`status` accepts no arguments at all, and refuses any.** `run_status_adapter`
  (`crates/prikk-cli/src/commands.rs:78-84`) returns a usage error for the first extra argument —
  RFC 121 §3 made that deliberate, because `prikk status --nonsense` used to exit `0`. Adding
  `--format` edits a ruling site, not a gap.
- **`status` has no `--format json` today.** Only three surfaces do: `verify` (via
  `args.rs`'s `VerifyOutputFormat`) and RFC 138's `trust maintainer list`/`check`.

## 3. The finding that changes the shape of this work

**A queued patch does not carry the path for the commonest kind of change.**

`OperationKind` (`crates/prikk-object/src/payload/patch/operations.rs`) is path-addressed for some
operations and node-addressed for others:

| Operation | Carries |
|---|---|
| `CreateFile` | `path: String` |
| `DeleteNode` | `path: String` |
| `RenamePath` | `old_path: String`, `new_path: String` |
| **`EditText`** | **`node_id` only** |

`EditText`'s own doc comment states it outright: *"EditText is node-addressed, not path-addressed."*
That is not an accident to route around — it is the property RFC 134 depends on, and the reason
editing a file survives a rename.

**So "patch ids and their target paths, in queue order" is not derivable from the queue alone.**
Editing an existing file — the ordinary case, and the one a seal ceremony most needs to describe —
yields a 32-byte node id and no path. Resolving it needs the **baseline live-node map**, which is
state `status` does not currently derive.

**This was not visible from the requester's side, and it was not visible from the recommendation
either.** It is recorded here rather than absorbed, because a design that had shipped option (a) below
without noticing would have satisfied the letter of the request and defeated its purpose.

## 4. The option space

**(a) Report only what the patch carries.** Patch id, operation kinds, and paths where the operation
has one; node ids where it does not.

*Cheap — no new derivation, no new cost.* **And it fails the purpose.** For a queue of ordinary edits
it renders `edit <node 8f2a…>` — a hash a human cannot recognize, presented at the moment the design
exists to make legible. It would let both projects report the feature as delivered while the screen it
was built for stayed dishonest. **Refused.**

**(b) Resolve node ids to paths against the folded baseline.** `resolve_folded_worktree_baseline`
(`prikk-store/src/patch_replay.rs:347`) already produces exactly this: the replay-derived baseline
**with the active WAL folded in**, which is the state the queued patches are addressed against.

*Correct, and it produces the thing that was asked for.* Its costs are real and are stated in §5.
**Recommended.**

**(c) Report resolved paths without patch ids.** Simpler for a UI, and loses the identity a consumer
needs to correlate an entry with anything else. **Refused** — the identity is free once (b) is done.

## 5. What (b) costs, and why it is not blocked on RFC 139

`resolve_folded_worktree_baseline` is a replay. `status` today does one WAL replay and no lineage
replay, so this is a genuine change to `status`'s cost profile, and **this project has no measurement
of it** — which is precisely the methodological weakness RFC 139 exists to retire.

**It does not have to wait for RFC 139's corpus, and the argument is not "it is probably fine."**

> **The resolution is the same derivation `commit` already performs, against the same baseline, every
> time a user commits.** `node_authoring.rs:37` imports `resolve_folded_worktree_baseline` and
> `:258` runs it on the ordinary commit path. So its cost is **bounded above by an operation the user
> already runs routinely**, and a `status` that costs no more than a `commit` is not a new class of
> cost — it is a known one appearing in a second place.

**That is a bound, not a measurement, and the RFC says so.** It is enough to proceed, and it is not
enough to claim `status` stays cheap. Two consequences:

1. **The prose `status` path must not become more expensive.** The resolution is done **only** when
   enumeration is actually requested. A user running bare `prikk status` pays nothing new.
2. **The implementing increment reports the cost it observed** at a realistic queue depth, into
   RFC 133's evidence table. When RFC 139's corpus exists, this is one of the first things worth
   re-measuring properly — and having a figure from before the corpus is how we will know whether the
   corpus changed the answer.

## 6. The surface — scoped the way RFC 138 scoped its own

RFC 138 §7.2 recorded that the general machine-readable surface is **an unopened design question**,
and that one command adopting `--format json` does not answer it. That ruling holds and this RFC does
not reopen it.

**The same narrow treatment applies here, and it is a ruling rather than a deferral:**

- `prikk status --format json` emits a document with its own `schema_version`, named for this command
  in the idiom `verify-report-v1` / `trust-list-v1` established — **not** a shared or general status
  schema.
- **It settles the format for `status` and for nothing else.** A third command adopting the flag is a
  third decision.
- **The document carries everything the prose form carries**, not only the queue. A machine-readable
  form that reports less than the prose form teaches consumers to run both and parse one, which is the
  outcome the flag exists to prevent.
- **Exit code is unchanged: `0`.** `status` answers a question; RFC 121's vocabulary and RFC 138's
  ruling both apply — an empty queue is an answer, not a failure.

## 7. What this RFC does not decide

- **The content surface** (stikk letter 003 §1 — `show`/`diff`, or content per changed path). Their
  oldest carried dependency and a much larger piece. **Separately ruled, and the ruling stands: if a
  content surface lands, its JSON form is designed at the same time, not a release later.** Nothing
  here schedules it.
- **The general machine-readable surface** — still unopened, per §6.
- **RFC 132 increment 2** — unrelated and still deferred.
- **Any change to what a patch stores.** `EditText` stays node-addressed; §3 is a finding about
  presentation, not a case for changing the operation model. **Adding a path to `EditText` would be a
  schema change to fix a display problem** and is refused here explicitly so that nobody proposes it
  later as an optimization.

## 8. Scope

**In:** `status --format json` with its own schema; queued-patch enumeration carrying patch id,
operation kinds, and **resolved** paths; the prose form's cost left unchanged; a cost figure reported
into RFC 133.

**Out:** everything in §7; any change to the prose `status` output's existing lines; any new command.

## 9. Risks

**The resolution can fail where the count cannot.** `node_authoring.rs:63-64` already names the case:
a changed path that does not resolve to a live node in the replay-derived baseline — *"e.g. a
snapshot-only baseline, which carries no node identity"* — and it **fails closed** there. Enumeration
must not inherit that: a queue whose paths cannot all be resolved should still report the patches, with
the unresolved ones marked as unresolved. **A `status` that refuses to answer because one node id did
not resolve would be worse than the count it replaces**, and this is the most likely way a careless
implementation of (b) does harm.

**A second consumer of a `pub(crate)` derivation.** `resolve_folded_worktree_baseline` is
`pub(crate)` in `prikk-store` and reached from the CLI only through the authoring path today. Exposing
it, or a narrower projection of it, is a real API decision and is the implementing increment's to make
and report — including whether it moves an edge or a hub in RFC 130's coupling gate.

**Related:** RFC 138 (the surface precedent, and the answer-not-failure ruling), RFC 121 (the
exit-code vocabulary, and the ruling that `status` refuses unknown arguments), RFC 133 (where the cost
figure goes), RFC 139 (the corpus that will let it be measured properly), RFC 134 (why `EditText` is
node-addressed), DC-66 and DC-57 (the existing queue report this extends).
