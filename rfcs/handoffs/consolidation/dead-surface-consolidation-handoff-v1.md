# Dead-surface consolidation — implementation handoff v1

**Authorized by the project owner 2026-08-15** as housekeeping, after RFC 102 completed.
**Three unrelated dead surfaces, each ruled below.** No RFC owns these; they accumulated across DC
increments and RFC 102's six stages.

This is deletion work. The value is that each of these currently *advertises* a capability or a
constraint that does not exist, and every gate reports the repository as healthy while they do.

## 1. `wal.rs:194`'s `ensure_directory_required` — remove

Identical to `wal.rs:283`'s, which Stage 6 round 3 removed under §14.8, left unruled at the time only
because it was outside that unit's scope.

`append_patch` calls `ensure_directory_required(root, parent)` immediately before
`append_file_required`. Since `d8f5240` made `durable_append` strict, **the call cannot help**: the WAL
file must already exist for the append to succeed, and it cannot exist without its directory. If the
directory were somehow absent, this call would create it and the append would then fail anyway on the
missing file.

It is also a directory-name creation on a write path, which is the same shape criterion 1 forbids for
containers.

**Remove it, with the same justification `wal.rs:278-282` already carries for its sibling** —
`default_active_dir()` is in `required_directories()` and permanent from `init`.

## 2. Ten dead `init` directories — remove from `required_directories()`. **`refs/tmp/` stays.**

`layout.rs:518-540` creates eleven directories nothing writes into. Verified reader-by-reader:

| Directory | Reader | Ruling |
|---|---|---|
| `objects/` + its six type subdirectories | none — `memory_store.rs`'s `objects` is an unrelated struct field | **remove** |
| `quarantine/` | none at all | **remove** |
| `refs/by-id/` | only `dc55_identity_evidence.rs`, which reads a **format-2 fixture**, not a live repository | **remove** |
| `refs/logs/` | same — fixture only | **remove** |
| **`refs/tmp/`** | **`refs/verify.rs:264-278`'s `candidate_issues` lists it on every `verify`** | **KEEP** |

**`refs/tmp/` is not dead in the same way and must not be removed with the others.** `verify` calls
`list_directory` on it every run, and that call is where `directory is absent: refs/tmp` comes from.
Nothing has written into it since Stage 4 removed the candidate mechanism, so the scan can only ever find
nothing — a mandatory directory whose sole purpose is to be scanned for files that can no longer exist.
**That is its own finding** (`FINDINGS.md`, the dormant candidate scan) and its own decision: retiring the
scan and the directory together is a `verify`-behaviour change, not consolidation. **Out of scope here.**

**Also remove the path accessors that only these directories justified**, each verified to have zero
production callers: `objects_dir`, `object_type_dir`, `object_path`, `ref_pointer_path`, `ref_log_path`,
`ref_tmp_path`, `quarantine_dir`. **`ref_lock_path` is live** (`lock.rs`) — keep it.

**This is not a format change.** `required_directories()` is consulted only by `init` (`layout.rs:194`);
nothing validates it at open. An existing repository keeps its now-unused directories harmlessly, and a
new one simply has fewer. **Establish that independently before relying on it** — I asserted the opposite
in published documentation yesterday and had to correct it twice.

## 3. `doctor`'s `ref_repair` — superseded, remove

`doctor.rs:178` declares `pub ref_repair: Option<RefRecoveryRepair>`; `:423` is a literal
`let ref_repair = None;`. So the variant can never occur.

**It is superseded, not unfinished** — established from history, not guessed. `RefRecoveryRepair` and
`RefStore::reconstruct_missing_ref_from_log` were built by `5d09c3f` as *"opt-in safe doctor repair for
… missing `heads/main` pointer reconstruction from verified ref-log data."* The hardcoded `None` was
introduced by **`f343d5e`, DC-38's ref publication crash recovery**, which replaced that path. The
producer function now has **only test callers** (`refs/tests.rs:681`, `:733`).

**Remove the field, the `RefRecoveryRepair` export from `lib.rs:140`, the type, and
`reconstruct_missing_ref_from_log`** — a `pub` API no production path can reach advertises a recovery
capability prikk does not have, which for a repair surface is worse than its absence.

**`crates/prikk-cli/src/branch.rs:17`'s doc comment references it as live.** Correct that too.

**If you find a reason it should stay** — a caller I missed, or an argument that the capability should be
re-wired rather than dropped — **stop and report.** Removing a recovery path is exactly the kind of
deletion that should not proceed on a reviewer's say-so alone if the implementer sees something else.

## 4. Acceptance criteria

1. **Each of the three surfaces removed or kept exactly as ruled**, with `refs/tmp/` demonstrably still
   created and still scanned.
2. **`verify` still passes on a fresh repository** — the obvious regression is removing `refs/tmp/` with
   the others.
3. **A test proves a repository without the ten removed directories opens and verifies**, so the
   not-a-format-change claim is asserted rather than argued.
4. **No production caller lost.** Enumerate what you removed and how you established each had none.
5. Full gate set per `EXECUTION-ORDER.md` §6 rule 9, plus **green three-platform CI**.
6. **`docs/src/reference/repository-layout.md` updated** — it currently documents eleven dead
   directories and the `refs/tmp` exception; after this it should document one.

## 5. Standing

- **Work on a branch.** Branch → push → green CI → merge.
- **Report counts** per rule 10. Baseline at `3aa6d51`: `prikk-store` **737**, `prikk` **117**,
  `prikk-release-policy` 83, `prikk-object` 80, `prikk-replay` 44, `prikk-hash` 14, `prikk-crypto` 7,
  `prikk-error` 0; **179 locked packages**. Report the figures; the architect updates the line at merge.
- Deletions will remove tests. **Say which and why** — a removed test is a removed guarantee unless the
  thing it guarded is gone too.
