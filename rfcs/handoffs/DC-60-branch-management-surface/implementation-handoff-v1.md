# DC-60 Branch Management Surface - Handoff

> ## ⚠ SCOPE AMENDED 2026-07-30 — `delete` and `--continue-log` removed
>
> Your report was accepted in full. `branch delete` as specified bricks repository-wide commits at **every**
> record count — the single-record case also blocks mutation via `publication_issues`, which your report
> understated. And `--continue-log` is not expressible in `publish`'s CAS model.
>
> **DC-60 is now `branch list` + `branch create` only.** Both ship as submitted. Deletion and log
> continuation moved to **DC-61**, which needs a ref-log tombstone — a format change DC-60 excluded.
>
> **Submit the working slice as its own review request**, and remove the deliberately-failing test with it;
> its finding is recorded in `.git-exclude/reviewed/prikk-dc60-delete-divergence-ruling-v1.md` and in
> DC-61, which are the durable places for it.
>
> Steps 1 and 2 below stand. **Step 3 is void.** The delete-then-recreate decision is void.

**Cleared to start.** Accepted by the project owner on 2026-07-30, at
`rfcs/accepted/DC-60-BRANCH-MANAGEMENT-SURFACE.md`. Design review v1 returned two blocking findings, both
resolved at `312fc5d` — the RFC you are working from is the revised one.
**Authored by** the architect.
**Size:** medium. One new CLI command with **two** subcommands (list, create), plus read-only enumeration.
**Touches:** `crates/prikk-cli` and read-only enumeration in `crates/prikk-store`. **No new object type, no
format change, no persisted byte change.**

## What this is

`prikk branch` — **list and create**. Part of the user-facing half of requirements §6.5, whose internal half
already ships. Deletion is DC-61.

## Read this first: creation already exists

**`commit --ref heads/topic` then `seal --ref heads/topic` already creates a branch.** That is DC-13
Non-Default Ref Genesis, shipped and in `rfcs/done/`. The implementation is at
`node_authoring.rs:212,221` — `WorktreeBaseline::Genesis` when the target ref has never been published.

An earlier draft of this RFC claimed no branch creation existed. It was wrong, and design review caught it.

**Why this matters to you:** do not build a second creation path. `branch create --from` must publish the
**same ref-state shape** DC-13's genesis publishes, so that a branch is indistinguishable afterward
regardless of which route made it:

- `update_seq = 1`
- `previous_ref_state_id = None`
- maintainer-signed
- `kind` matching DC-13's branch genesis

The only permitted difference is the target: DC-13 seals a block it just created; you point at a block that
already exists. **Nothing else about the published state may differ.** Read DC-13 before you start.

## Step 1 — `prikk branch` (list)

Enumerate `.prikk/refs/by-id/*.ref` and read each pointer file's recorded name. Report name plus current
ref-state object id. Read-only.

**Names are recoverable and you do not need an index.** Filenames are `sha256(ref_name)` and one-way, but
the pointer file body carries the plaintext name — magic `PREFPTR1`, then e.g. `heads/main`. Verified
against `crates/prikk-cli/tests/fixtures/dc55_pre_swap_repo/`. `RefStatePayload.ref_name`
(`prikk-object/src/payload/refs.rs:41`) carries it too.

**`by-id/` is the complete set of pointers** — established at design review. `layout.rs` `ref_pointer_path`
is the only function producing a `.ref` path and it always joins `by-id`. Logs, locks, and tmp files live in
`logs/`, `locks/`, `tmp/` and are not pointers. You do not need to search elsewhere, and you should not.

Sort by name. Deterministic output is what makes this testable and scriptable.

## Step 2 — `prikk branch create <name> [--from <ref>]`

Publish a new ref-state object for `<name>` targeting `--from`'s block (default: the current default ref).
Shape as stated above.

Reuse what exists — do not write new versions of any of these:

- `validate_local_branch_ref` for name validation
- `refs.rs:101` `publish` for the CAS publication
- `maintainer_signer_from_env` (`prikk-cli/src/main.rs:147`) for signing. Ref state is maintainer-signed —
  `prikk-object/src/signature.rs:49-50` defines `Maintainer = 2` as "Maintainer publishing/sealing a block
  or ref state"

**Fail closed when:**

- `<name>` fails `validate_local_branch_ref`
- `<name>` already exists — creation is not a move
- `--from` does not resolve to a published ref
- `<name>` has a **surviving ref log** with no live pointer — fail closed, no escape flag. Such a log can
  survive an interrupted publication, and creating over it produces the corrupt state DC-61 exists to
  resolve. `publish` would refuse anyway, but a clear early error beats a generic classification failure.

## Step 3 — VOID, moved to DC-61

*Retained below only as the record of what was asked for. Do not implement it.*

### (void) `prikk branch delete <name>`

**Remove the pointer only. Do not delete the ref log. Do not delete objects.**

An earlier draft said to remove the log. That would have violated:

- **NFR-REL-01** (`specs/prikk-non-functional-requirements-v1.1.md:108`) — "preserves objects and reports
  manual repair rather than deleting data"
- **§6.5** — "ref update logs must support rollback detection and recovery," which is the log's whole purpose
- **DC-13 goal 3** — "missing pointer plus non-empty log is **not** genesis." That state is one the system
  already reasons about; deleting the log destroys what the distinction rests on

**Fail closed when:**

- `<name>` does not exist
- `<name>` owns a **non-empty** active WAL. `require_active_ref_for_non_empty_wal` already encodes this, and
  **DC-13 goal 4** already establishes the rule. Cite and reuse; do not restate it as new
- `<name>` is the last remaining branch — a repository with no refs has no reachable history

**Say in the output that no objects were reclaimed.** Garbage collection is NFR-REL-02 and out of scope; a
user should not believe deletion freed space.

### Delete-then-recreate — the decision is already made

Deleting `heads/topic` leaves its log behind. Recreating that name later is therefore **not** genesis under
DC-13 goal 3, so publishing at `update_seq = 1` would contradict a shipped invariant.

- **Default: reject**, with a message naming `--continue-log` as the remedy.
- **With `--continue-log`:** publish at `last_seq + 1`, with `previous_ref_state_id` set to the last
  ref-state recorded in that log. An ordinary CAS update, not a genesis.

Do not implement silent continuation, and do not restart at sequence 1. The first hands a user
rollback-detectable history they cannot see; the second breaks DC-13 goal 3.

## Not in scope

- **No `branch switch`, and do not add a current-branch pointer.** Nothing stores a current branch today;
  every command takes `--ref` defaulting to a literal `"heads/main"` (`args.rs:90,135`). Adding one changes
  default ref resolution for every existing command and collides with the single repository-wide active
  slot. Separate increment, better designed after the multi-patch queuing decision.
- **No deletion** — DC-61.
- No tagging (§6.6), no remote or tracking branches (§6.11), no garbage collection (NFR-REL-02).
- No change to how existing commands resolve `--ref`.

**Command help must state that switching is unsupported and why.** Shipping list and create without switch
or delete is a partial surface; a user should learn that from `--help`, not by trying.

## Traps

- **Building a second creation path** instead of matching DC-13's shape. The most likely mistake here.
- **Implementing anything from the void Step 3.** Deletion is DC-61's, and its design is not settled.
- **Searching for ref pointers outside `by-id/`.** There are none, and looking invites finding logs or
  locks and treating them as refs.
- **Writing a second name validator or signing path.** Both exist.
- **Non-deterministic listing order.** Untestable and unscriptable.

## Definition of done

`branch` lists deterministically; `branch create` publishes a DC-13-shaped ref-state and fails closed on all
four conditions; `branch delete` removes the pointer only, retains log and objects, fails closed on all
three conditions, and says nothing was reclaimed; delete-then-recreate rejects by default and continues
correctly under `--continue-log`; help states the switch limitation; no identity artifact changed.

## Submit with

The diff; test results for each fail-closed condition constructed as real state rather than asserted in
isolation; evidence that delete-then-recreate was tested **both** ways; confirmation that
`vectors/snapshot.txt`, `vectors/hard.rs`, `state_root/tests/vectors.rs`, and `text_span/vectors.rs` are
byte-identical; test counts per touched crate before and after; an explicit statement of what did not
change; and the full gate set from `rfcs/EXECUTION-ORDER.md` §6 rule 9 including release-policy `check`,
`boundary-check`, and `reference-check`.

**One request, given this increment's history.** Both blocking findings against the RFC came from checking
`rfcs/done/` and `specs/` rather than from reading the draft. If something here contradicts a shipped RFC or
an accepted requirement, that is a finding worth stopping for — as DC-57's team did. It will be treated as
one.
