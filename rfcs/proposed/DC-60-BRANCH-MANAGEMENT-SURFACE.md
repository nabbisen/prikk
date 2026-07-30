# RFC (proposed) - DC-60 Branch Management Surface

**Status.** Proposed. Requires design review before implementation may begin.
**Requirement.** `specs/prikk-app-requirements-v1.2.md` §6.5 (Branch and Ref Management).
**Gate.** Product **M1** (Core Storage and Identity) owns the ref machinery, which is complete. This RFC
adds the missing user-facing surface over it. Not a missed gate — §6.5's *internal* requirements are all
met; no command exposes them.
**Touches.** `crates/prikk-cli` (new `branch` command, args, output) and read-only enumeration in
`crates/prikk-store`. **No new object type, no format change, no persisted-byte change.**

## Problem

Every internal requirement in §6.5 is satisfied:

- Branch heads are signed ref-state objects — `RefStatePayload` (`prikk-object/src/payload/refs.rs:39`)
- Ref files point to ref-state objects — `refs.rs:147` `read_current_ref_state_id`
- Ref updates use compare-and-swap — `refs.rs:101` `publish`, with `previous_ref_state_id`
- Ref logs support rollback detection and recovery — DC-38, shipped
- Ref locks are path-safe — `validate_local_branch_ref`, DC-15

**None of it is reachable from the CLI.** There is no `branch` command. Every command takes `--ref` and
defaults to a string literal `"heads/main"` (`prikk-cli/src/args.rs:90,135`). A user cannot create a
branch, list what branches exist, or remove one.

This is the largest gap in the product that requires no new capability — only a surface over machinery
that is already tested and shipped.

## What this requires that does not exist yet

*Mandatory section. Three consecutive RFCs in this program specified work whose prerequisite was absent —
DC-56's index design, DC-59's PRNG and signing setup, DC-57's configuration mechanism. This section exists
so that check is structural rather than remembered.*

| Needed | Exists? |
|---|---|
| Create a ref-state object and publish it under a new name | **Yes** — `refs.rs:101` `publish` with CAS |
| Recover branch names for listing | **Yes.** `.prikk/refs/by-id/<sha256(name)>.ref` filenames are one-way, but the pointer file body carries the plaintext name (magic `PREFPTR1`, then e.g. `heads/main`), and `RefStatePayload.ref_name` carries it too. **Verified against the DC-55 fixture.** No reverse-hashing and no new index needed |
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
default ref). This is an ordinary CAS publication with `previous_ref_state_id = None`, since the branch is
new.

Must fail closed when:

- `<name>` fails `validate_local_branch_ref` — reuse the existing validator, do not write a second one
- `<name>` already exists — creation is not a move
- `--from` does not resolve to a published ref

### 3. `prikk branch delete <name>`

Remove the ref pointer and log for `<name>`. Must fail closed when:

- `<name>` does not exist
- `<name>` is the ref that currently owns a **non-empty** active WAL. `require_active_ref_for_non_empty_wal`
  already encodes this relationship; deleting under it would orphan an unsealed patch
- `<name>` is the last remaining branch. A repository with no refs has no reachable history

**Deletion does not delete objects.** Ref-state objects, blocks, and patches remain; only the ref pointer
and its log are removed. Garbage collection is NFR-REL-02 and out of scope. Say so in the command's output
so a user does not believe deletion reclaimed space.

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
3. `branch create` fails closed on an invalid name, an existing name, and an unresolvable `--from` — each
   tested.
4. `branch delete` removes the pointer and log; objects are demonstrably retained; `verify` passes
   afterward.
5. `branch delete` fails closed on a missing branch, on a branch owning a non-empty active WAL, and on the
   last remaining branch — each tested by constructing the state.
6. No identity artifact changes: `vectors/snapshot.txt`, `vectors/hard.rs`,
   `state_root/tests/vectors.rs`, `text_span/vectors.rs` all byte-identical.
7. Command help states that switching is not supported and names the reason.
8. Full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9, plus test counts before and after per rule 10.

All eight are verifiable from the repository by a reviewer. None requires trusting the implementer's
report.
