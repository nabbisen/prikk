# RFC (proposed) - DC-60 Branch Management Surface

**Status.** Proposed. Requires design review before implementation may begin.
**Requirement.** `specs/prikk-app-requirements-v1.2.md` §6.5 (Branch and Ref Management).
**Gate.** Product **M1** (Core Storage and Identity) owns the ref machinery, which is complete. This RFC
adds the missing user-facing surface over it. Not a missed gate — §6.5's *internal* requirements are all
met; no command exposes them.
**Touches.** `crates/prikk-cli` (new `branch` command, args, output) and read-only enumeration in
`crates/prikk-store`. **No new object type, no format change, no persisted-byte change.**

## Problem

**Branch creation already exists.** `rfcs/done/DC-13-NONDEFAULT-REF-GENESIS.md` shipped it:
`commit --ref heads/topic` followed by `seal --ref heads/topic` creates and publishes an unborn branch as a
signed Root block at update sequence 1 (`DC-13` design goals 1-2; implementation at
`node_authoring.rs:212,221` — `WorktreeBaseline::Genesis` when the target ref has never been published).

Design review v1 found that an earlier draft of this RFC claimed no branch creation existed. It was wrong.
The actual gap is narrower:

| Capability | State |
|---|---|
| Create a branch **by committing to it** | **Exists** — DC-13 |
| Create a branch **at an existing target, without committing** | **Missing.** DC-13 requires content; there is no "branch off here" |
| **List** branches | **Missing** |
| **Delete** a branch | **Missing** |
| Switch branches | Missing — deliberately out of scope, see Non-goals |

Everything §6.5 requires *internally* is satisfied and shipped:

- Branch heads are signed ref-state objects — `RefStatePayload` (`prikk-object/src/payload/refs.rs:39`)
- Ref files point to ref-state objects — `refs.rs:147` `read_current_ref_state_id`
- Ref updates use compare-and-swap — `refs.rs:101` `publish`, comparing `update_seq` at `:208`
- Ref update logs support rollback detection and recovery — DC-38, shipped
- Ref locks are path-safe — `validate_local_branch_ref`, DC-15

**Listing is the largest genuine gap.** A user can create branches today but has no way to discover which
exist — arguably worse than lacking creation, because the repository accumulates state the user cannot see.

## What this requires that does not exist yet

*Mandatory section. Three consecutive RFCs in this program specified work whose prerequisite was absent —
DC-56's index design, DC-59's PRNG and signing setup, DC-57's configuration mechanism. This section exists
so that check is structural rather than remembered.*

| Needed | Exists? |
|---|---|
| Create a ref-state object and publish it under a new name | **Yes** — `refs.rs:101` `publish` with CAS |
| Recover branch names for listing | **Yes.** `.prikk/refs/by-id/<sha256(name)>.ref` filenames are one-way, but the pointer file body carries the plaintext name (magic `PREFPTR1`, then e.g. `heads/main`), and `RefStatePayload.ref_name` carries it too. **Verified against the DC-55 fixture.** No reverse-hashing and no new index needed |
| Certainty that `by-id/` holds *every* ref pointer | **Yes — resolved at design review v1.** `layout.rs` `ref_pointer_path` is the only function producing a `.ref` path and it always joins `by-id`. Logs, locks, and tmp files go to `logs/`, `locks/`, `tmp/` respectively and are not pointers. One code path, one location |
| Ref-name validation and path safety | **Yes** — `validate_local_branch_ref` |
| **A persistent current-branch pointer ("HEAD")** | **NO.** Nothing stores which branch is current. `default_active_ref_name_path()` tracks which ref owns the single active-session slot — that is per-session bookkeeping, not a user-facing current branch |

That last row determines this RFC's scope. See Non-goals.

## Design

### 1. `prikk branch` — list

Enumerate `.prikk/refs/by-id/*.ref`, read each pointer file's recorded name, and report name plus current
ref-state object id. Read-only; no lock required beyond what reading a ref already takes.

Output must be deterministic — sort by name — so it is scriptable and testable.

### 2. `prikk branch create <name> [--from <ref>]`

Publish a new ref-state object for `<name>`, targeting the same block as `--from` (default: the current
default ref).

**This must produce the same ref-state shape DC-13's genesis produces**, so that a branch is
indistinguishable afterward regardless of which path created it. Two creation paths yielding different
shapes would be a real defect. Concretely, for a name with **no surviving log**: `update_seq = 1`,
`previous_ref_state_id = None`, maintainer-signed, `kind` matching DC-13's branch genesis. `update_seq` is
per-ref (compared at `refs.rs:208` for CAS), so sequence 1 is correct for a new ref regardless of how old
its target block is.

For a name whose log survives a previous deletion, sequence 1 is **not** correct — see §3's resolution.

The only difference from DC-13 is the target: DC-13 seals a block it just created, while this points at a
block that already exists. Nothing else about the published state may differ.

Must fail closed when:

- `<name>` fails `validate_local_branch_ref` — reuse the existing validator, do not write a second one
- `<name>` already exists — creation is not a move
- `<name>` has a surviving ref log from a previous deletion, unless `--continue-log` is given — see §3
- `--from` does not resolve to a published ref

### 3. `prikk branch delete <name>`

Remove the ref **pointer** for `<name>`. **The ref log is retained** — see below. Must fail closed when:

- `<name>` does not exist
- `<name>` is the ref that currently owns a **non-empty** active WAL. `require_active_ref_for_non_empty_wal`
  already encodes this relationship, and **DC-13 design goal 4** already establishes the rule — "prevent a
  queued active WAL from being sealed to a different ref than the ref it was authored for." Cite and reuse
  it; do not restate it as a new invariant. Deleting under it would orphan an unsealed patch
- `<name>` is the last remaining branch. A repository with no refs has no reachable history

**Deletion removes the pointer only. It deletes no objects and no history.**

Design review v1 found that an earlier draft deleted the ref log too, which violates two accepted
requirements:

- **NFR-REL-01** (`specs/prikk-non-functional-requirements-v1.1.md:108`): "On uncertainty, Prikk preserves
  objects and reports manual repair rather than deleting data."
- **§6.5**: "Ref update logs must support rollback detection and recovery" — the log's whole purpose.

There is also an existing invariant it would have broken. `DC-13` design goal 3 records that "missing
pointer plus non-empty log is **not** genesis." A log surviving without its pointer is a state the system
already reasons about, and deleting the log destroys what that distinction depends on.

**Consequence, resolved here rather than left to implementation.** Deleting `heads/topic` and later
recreating it leaves a non-empty log from the previous incarnation. Under DC-13 goal 3 that state is *not*
genesis, so publishing at `update_seq = 1` would contradict an existing invariant.

**Resolution: `branch create` rejects a name with a surviving log, and names the remedy.** Continuation is
available only through an explicit opt-in (`--continue-log`), which publishes at `last_seq + 1` with
`previous_ref_state_id` set to the last ref-state recorded in that log — an ordinary CAS update, not a
genesis.

Rejecting by default follows **DC-13 goal 5** — "keep non-default genesis explicit; never infer" — and
NFR-REL-01's preference for reporting over guessing. The alternative, silently continuing a deleted
branch's history, would hand a user a branch carrying rollback-detectable history they did not ask for and
cannot see. The alternative of silently restarting at sequence 1 would break DC-13 goal 3 outright.

Garbage collection of now-unreferenced objects is NFR-REL-02 and out of scope. Say so in the command's
output so a user does not believe deletion reclaimed space.

### 4. Signing

Branch creation publishes a signed ref-state object, so it requires MAINTAINER signing on the same terms
as `seal`. Follow `maintainer_signer_from_env` (`prikk-cli/src/main.rs:147`); do not introduce a second
signing path. Deletion removes a pointer and does not create a signed object.

## Non-goals

- **No `branch switch` and no current-branch pointer.** This is the deliberate boundary. Introducing a
  persistent "current branch" changes default ref resolution for *every* existing command, and it collides
  with the single repository-wide active-session slot: switching while the active WAL holds a patch for
  another ref has no defined behaviour today. That deserves its own RFC, and it is better designed after
  the multi-patch queuing decision — which may replace the single slot with per-ref active WALs and change
  the answer entirely.
- No tagging — §6.6, its own increment.
- No remote or tracking branches — §6.11, product M5.
- No garbage collection of unreferenced objects — NFR-REL-02.
- No new object type, schema, or persisted-byte change.
- No change to how existing commands resolve `--ref`.

## Risks

**Deleting a branch under an active session.** Covered by the fail-closed condition in §3, and the
existing helper already models the relationship. The test must construct the state rather than assert the
guard exists in isolation.

**Listing that silently misses branches.** If enumeration reads only `by-id/*.ref` and some ref exists in
another form, listing under-reports — worse than failing, because a user would believe a branch is gone.
The design review should confirm `by-id/` is the complete set of branch pointers; my reading is that it is,
but "the only place refs live" is exactly the kind of assumption this program has been caught on.

**A user expecting `switch`.** Shipping create/list/delete without switch is a partial surface. The command
help must say so plainly rather than leaving a user to discover it.

## Acceptance criteria

1. `branch` lists all branches with deterministic ordering, verified against a fixture with more than one
   branch.
2. `branch create` publishes a signed ref-state object; the new branch appears in listing; `verify` passes
   afterward.
3. `branch create` fails closed on an invalid name, an existing name, an unresolvable `--from`, and a name
   with a surviving log absent `--continue-log` — each tested.
4. Delete-then-recreate is tested both ways: rejected by default with an actionable message, and with
   `--continue-log` publishing at `last_seq + 1` with the correct predecessor, after which `verify` passes.
5. `branch delete` removes the pointer **only**; the ref log and all objects are demonstrably retained;
   `verify` passes afterward.
6. `branch delete` fails closed on a missing branch, on a branch owning a non-empty active WAL, and on the
   last remaining branch — each tested by constructing the state.
7. No identity artifact changes: `vectors/snapshot.txt`, `vectors/hard.rs`,
   `state_root/tests/vectors.rs`, `text_span/vectors.rs` all byte-identical.
8. Command help states that switching is not supported and names the reason.
9. Full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9, plus test counts before and after per rule 10.

All nine are verifiable from the repository by a reviewer. None requires trusting the implementer's
report.
