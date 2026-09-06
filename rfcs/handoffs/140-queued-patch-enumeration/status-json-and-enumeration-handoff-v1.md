# RFC 140 — `prikk status --format json`, and which patches are queued

**RFC:** `rfcs/accepted/140-queued-patch-enumeration.md` — **accepted in full 2026-09-06.** §4's
ruling (option (b), resolve against the folded baseline) and §6's surface scoping are settled input.
**Base:** `main` at `02ac5c0`.
**Origin:** stikk letter 003 §2, verified in
`.git-exclude/reviewed/stikk-letters-003-004-review-v1.md`.

**§3 is the part to read twice.** The obvious implementation ships something that satisfies the
request and defeats its purpose, and it will look correct in every test written against a
freshly-created file.

---

## 1. What to build

Two things, and the second is the reason the first exists.

**`prikk status --format json`** — a machine-readable form of everything `status` already prints.

**Queued-patch enumeration** in that form: for each queued patch, in queue order — its **patch id**,
its **operation kinds**, and the **paths those operations affect**, resolved.

## 2. `--format json` on `status` edits a ruling site

**`status` accepts no arguments today and refuses every one of them.**
`run_status_adapter` (`crates/prikk-cli/src/commands.rs:78-84`) returns
`CliError::Usage("unknown status argument: …")` for the first extra argument. **That is RFC 121 §3
deliberately**, because `prikk status --nonsense` used to exit `0`.

So: accept `--format json`, and **keep refusing everything else, including `--format` with any other
value.** The two RFC 138 sites are the pattern to copy — `main.rs:344-351` and `:380-387` reject a
non-`json` value with a usage error and use `mark_seen` to refuse a repeated flag. Do not loosen the
adapter into accepting arbitrary arguments.

**Schema name:** its own, in the established idiom (`verify-report-v1`, `trust-list-v1`), naming this
command. **It settles the format for `status` and nothing else** (RFC 140 §6). Do not invent a shared
or general status schema, and do not reuse `verify`'s.

**The JSON must carry everything the prose form carries** — repository path, WAL record count,
trailing partial bytes, the `heads/main` RefState, the queue count and its owning ref, and the DC-57
warn/hard-limit condition. A machine-readable form that reports *less* than the prose form teaches
consumers to run both and parse one.

**Exit code is unchanged: `0`.** An empty queue is an answer, not a failure.

## 3. The path problem — read this before writing any test

**A queued patch does not carry a path for the commonest kind of change.**

`OperationKind` (`crates/prikk-object/src/payload/patch/operations.rs`) is not uniform:

| Operation | Carries |
|---|---|
| `CreateFile` (`:33`) | `path: String` |
| `DeleteNode` (`:98`) | `path: String` |
| `RenamePath` (`:272`) | `old_path`, `new_path` |
| **`EditText` (`:167`)** | **`node_id` only** — its doc says *"EditText is node-addressed, not path-addressed"* |

**Why this will not show up in a naive test.** A test that creates a file and queues it produces
`CreateFile`, which carries a path, and passes. **Editing an existing file is the ordinary case**, and
it produces `EditText` with no path at all. **Write the editing case first.**

**The resolution.** `resolve_folded_worktree_baseline` (`prikk-store/src/patch_replay.rs:347`) gives a
`FoldedWorktreeBaseline` whose `state: NodeLifecycleState` answers exactly this:

```
state.live_node(&node_id) -> Option<&LiveNode>     // query.rs:9
LiveNode { path: RepoPath, kind, content }         // types.rs:26-33
```

**This is the single derivation every worktree-comparing command already uses** (RFC 122 §3):
`commit` (`node_authoring.rs:37`, `:258`) and `worktree-status` (`worktree_status.rs:24`) both call
it rather than reconstructing baseline state a second way. **Use it. Do not write a second
resolution**, even a small one — a second implementation that happens to agree today is the exact
defect RFC 122 exists to have fixed.

`resolve_folded_worktree_baseline` is `pub(crate)` in `prikk-store`. **Exposing it, or a narrower
projection of it, is your decision and your report's to justify** — including whether it moves an
edge or a hub in RFC 130's coupling gate. A narrower projection (node ids → paths for a given queue)
is likely the better boundary than making the whole folded baseline public; say which you chose and
why.

## 4. Unresolved node ids are reported, never fatal

**`live_node` returns `Option` by construction, and that `None` is a real state, not a defect to
assert away.** `node_authoring.rs:63-64` names the case: a changed path that does not resolve to a
live node in the replay-derived baseline — *"e.g. a snapshot-only baseline, which carries no node
identity"* — and **`commit` fails closed there, correctly, because it is about to author.**

**`status` must not.** It is a read. A queue with one unresolvable node id must still report every
patch, with that entry **marked unresolved** and carrying the node id it could not resolve.

**A `status` that refuses to answer because one node id did not resolve is worse than the count it
replaces**, and this is the most likely way a careful implementation of §3 still does harm.

## 5. The prose path must pay nothing new

The baseline derivation is a replay. **`prikk status` with no arguments must not perform it.** Resolve
only when enumeration is actually requested.

**Measure and report what it costs** at a realistic queue depth — wall clock, against an empty queue
and a deep one. This is a number, not an impression, and it goes into the report; RFC 133 is where it
will be filed, and RFC 139's corpus is what will let it be re-measured properly later. **Having a
figure taken before the corpus exists is how we will know whether the corpus changed the answer.**

## 6. Out of scope

- **A content surface** — `show`/`diff`, or content per changed path. The same reporter's larger,
  older request. **Ruled separately: if a content surface lands, its JSON form is designed at the
  same time. Nothing here schedules it, and nothing here should anticipate its shape.**
- **The general machine-readable surface.** Still unopened (RFC 138 §7.2). One more command adopting
  the flag does not open it.
- **Adding a path to `EditText`.** **Explicitly refused in RFC 140 §7** — it would be a schema change
  to fix a display problem. If §3 tempts you toward it, that is the temptation the refusal anticipates.
- **Any change to the prose `status` output's existing lines.**
- **`--format json` on any third command.**

## 7. Controls

1. **A queue of `EditText` operations enumerates with real paths.** Edit an existing, committed file;
   confirm the entry names the file, not a node id. **This is the control the whole increment exists
   for** — write it first, and watch it fail before §3's resolution is wired.
2. **A mixed queue.** Create, edit, delete and rename in one queue; all four report paths, and the
   rename reports both.
3. **Queue order is queue order.** Not sorted by path, not by id. Assert against a queue whose
   insertion order differs from any natural sort.
4. **An unresolvable node id does not fail the command.** Construct the case, confirm exit `0`, every
   other patch reported, and the one entry marked unresolved (§4).
5. **An empty queue.** Exit `0`, valid JSON, an empty list — not an error and not an absent field.
6. **The prose path does not regress.** `prikk status` with no arguments produces byte-identical
   output to before this change, and does **not** perform the baseline derivation. Assert the second
   half, do not assume it from the first.
7. **Argument refusal survives.** `prikk status --nonsense` still exits `2`; `--format yaml` exits
   `2`; a repeated `--format` exits `2`. RFC 121 §3 is not loosened by this change.
8. **The JSON parses and carries its `schema_version`.** This crate has no `serde_json` — mirror
   `rfc138_trust_read_surface.rs`'s own hand-written syntax checker, as RFC 138 did, rather than
   adding a dependency.

## 8. Gates

The full set, verbatim from `rfcs/EXECUTION-ORDER.md` §6 rule 9:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test --workspace --locked`
- `cargo +1.85.0 test --workspace --locked`
- `cargo +1.85.0 check --workspace --all-targets --locked`
- `git diff --check`
- `cargo audit --no-fetch`
- `RUSTDOCFLAGS="-D rustdoc::private_intra_doc_links" cargo doc --workspace --no-deps`
- release-policy `check`, `boundary-check`, `reference-check`

**`boundary-check` carries the RFC 130 coupling gate.** Widening a `prikk-store` visibility or adding
a CLI→store edge is exactly what it watches. Run it early; if it fires, the entry needs a reason
**and** a `what_would_remove_it`, and a new cycle is a finding worth reporting rather than a
declaration to write quickly.

**Docs are a gate, not a courtesy.** `docs/src/reference/commands.md` is a declared document and its
`status` line must name `--format json`. Check rule (A)/(B) in `crates/prikk-cli/src/commands/tests.rs`
if you are unsure whether your wording satisfies them.

**Cross-target clippy only if your own diff introduces `#[cfg(target_os)]`/`#[cfg(unix)]`/
`#[cfg(windows)]`.**

## 9. This adds a user-facing surface, so it needs a `CHANGELOG.md` entry

Under `## Unreleased`. Name both halves: `status --format json`, and that the queue can now be
enumerated rather than counted. **Say what it is for** — knowing *which* patches a seal will freeze,
not only how many.

**This instruction is here because its absence has shipped undocumented features twice**
(`.prikkignore` in 0.29.0, `prikk key`/`prikk setup` in 0.33.0). Both times the cause was a handoff
that did not mention it.

## 10. Reporting

`.git-exclude/review-request/`, per the standing convention. Include:

- **the cost figures from §5**, empty queue and deep queue, with the depth stated;
- **which boundary you chose** for exposing the resolution, and why that rather than making the whole
  folded baseline public;
- whether the coupling gate moved, and what you declared if so;
- how you saw control 1 fail before §3's resolution was wired;
- **anything you found that suggests option (b) was the wrong ruling.** The RFC ruled it on a bound
  rather than a measurement; you will be the first to hold a measurement.
