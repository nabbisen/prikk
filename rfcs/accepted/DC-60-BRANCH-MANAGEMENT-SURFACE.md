# RFC (accepted) - DC-60 Branch Management Surface

**Status.** **Accepted 2026-07-30; scope amended 2026-07-30** to `branch list` and `branch create` only.
`branch delete` and `branch create --continue-log` were **removed** and moved to **DC-61**.

**Why amended.** Implementation surfaced two defects in this RFC's own design review resolution
(`.git-exclude/reviewed/prikk-dc60-delete-divergence-ruling-v1.md`). Retaining the ref log on deletion —
required here on NFR-REL-01 grounds — produces "pointer absent, log present", which the shipped system
classifies as **corruption**. `verify.rs:145` recognises that state only for `record_count == 1`, and
`ensure_no_incomplete_publication` (`refs.rs:31-42`, called from every mutation path) blocks commits
**repository-wide** in both branches: `Integrity` for multi-record, `LockConflict` for single-record. There
is no record count at which deletion as specified leaves a working repository. Separately,
`publish`'s CAS model cannot represent an absent pointer with an advanced log, so `--continue-log` was
unimplementable.

The correct fix is a typed deletion record in the ref log — a **format change** this RFC's non-goals
exclude. Hence DC-61 rather than a repair here.

Original acceptance follows, after design review v1 returned two blocking
findings — a problem statement that claimed a shipped capability was absent, and a deletion step that would
have violated NFR-REL-01 — both resolved in revision at `312fc5d`. Implementation may begin.

**Independence.** Authored and reviewed by the architect; this project has one architect, so design review
here is author re-examination. It found both blocking findings by consulting `rfcs/done/` and `specs/`
rather than by re-reading the draft. Acceptance criteria are written to be reproducible from the
repository, so the implementation review carries the independent weight.
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

For a name whose log survives without a pointer, sequence 1 is **not** correct and creation is refused —
resuming such a log is DC-61's problem, not this RFC's.

The only difference from DC-13 is the target: DC-13 seals a block it just created, while this points at a
block that already exists. Nothing else about the published state may differ.

Must fail closed when:

- `<name>` fails `validate_local_branch_ref` — reuse the existing validator, do not write a second one
- `<name>` already exists — creation is not a move
- `<name>` has a **surviving ref log** with no live pointer. Fail closed with a message pointing at DC-61.
  This guard must stay even though DC-60 no longer deletes anything: such a log can survive an interrupted
  publication, and creating over it would produce the corrupt state described in the Status note.
  `publish` would refuse anyway, but a clear early error beats a generic classification failure
- `--from` does not resolve to a published ref

### 3. Deletion — removed from this RFC

`branch delete` is **DC-61**. See the Status note for why it could not ship here.

### 4. Signing

Branch creation publishes a signed ref-state object, so it requires MAINTAINER signing on the same terms
as `seal`. Follow `maintainer_signer_from_env` (`prikk-cli/src/main.rs:147`); do not introduce a second
signing path. Deletion is out of scope — see §3.

## Non-goals

- **No `branch switch` and no current-branch pointer.** This is the deliberate boundary. Introducing a
  persistent "current branch" changes default ref resolution for *every* existing command, and it collides
  with the single repository-wide active-session slot: switching while the active WAL holds a patch for
  another ref has no defined behaviour today. That deserves its own RFC, and it is better designed after
  the multi-patch queuing decision — which may replace the single slot with per-ref active WALs and change
  the answer entirely.
- **No deletion.** `branch delete` is DC-61, which must also settle the ref-log tombstone format question.
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
   with a surviving log and no live pointer — each tested against constructed state.
4. No identity artifact changes: `vectors/snapshot.txt`, `vectors/hard.rs`,
   `state_root/tests/vectors.rs`, `text_span/vectors.rs` all byte-identical.
5. Command help states that switching is not supported and names the reason.
6. Full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9, plus test counts before and after per rule 10.

All six are verifiable from the repository by a reviewer. None requires trusting the implementer's
report.
