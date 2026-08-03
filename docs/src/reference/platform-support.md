# Platform Support

This page is the authoritative current-state reference for which platforms Prikk runs on and,
concretely, which commands are read-only versus repository mutation. It exists because that
boundary had never been enumerated anywhere — DC-71 traced it once, here, so it does not have to be
re-derived from source on demand and cannot drift silently again (a CI job builds every listed
non-Linux target on every change; see [Non-Linux CI conformance](#non-linux-ci-conformance) below).

## The boundary

**Repository *mutation* requires Linux.** `crates/prikk-store`'s anchored filesystem primitives use
Linux-specific no-follow, nonblocking, atomic-rename, and no-clobber-install capabilities
([durability and crash recovery](./durability-recovery.md)) that have no reviewed equivalent on other
platforms yet ([DC-37](https://github.com/nabbisen/prikk/blob/main/rfcs/accepted/DC-37-REQUIRED-FILESYSTEM-DURABILITY.md)).
Every mutation function's *signature* compiles on every platform; only its *body* is Linux-only, and
a non-Linux caller receives a clean runtime error rather than a build failure or a silent no-op.

**Read-only commands build and run everywhere.** They never reach a mutation primitive — verified by
tracing every command's call graph to `crates/prikk-store/src/fsutil`'s mutation set (`ensure_root`,
`write_file_atomically`, `write_worktree_file_atomically`, `append_file_required`,
`truncate_existing_file_required`, `truncate_file_empty_required`, `create_new_file_required`,
`remove_file_required`/`remove_file_if_present_required`/`remove_worktree_file_required`,
`promote_file_required`, `publish_immutable_file`, `ensure_directory_required`,
`sync_directory_required`), not merely by a command's name suggesting it.

## The command set

| Command | Boundary |
|---|---|
| `verify` | Read-only |
| `log` | Read-only |
| `status` | Read-only |
| `doctor` (no repair flags) | Read-only |
| `doctor --repair-wal-tail` / `--repair-main-ref` | **Mutation** |
| `checkout --plan-only` | Read-only |
| `checkout --snapshot-plan` | Read-only |
| `checkout --snapshot-materialize` | **Mutation** (writes the worktree) |
| `checkout --patch-plan` | Read-only |
| `checkout --patch-materialize` | **Mutation** (writes the worktree) |
| `checkout --patch-delete-plan` | Read-only |
| `checkout --patch-materialize-delete` | **Mutation** (writes and deletes worktree files) |
| `merge-evidence` | Read-only |
| `merge-plan` | Read-only |
| `inverse-plan` | Read-only |
| `rollback-preview` | Read-only |
| `rollback-draft` | **Mutation** (appends to the active WAL) |
| `rollback-draft-verify` | Read-only |
| `worktree-status` | Read-only, but see the note below — currently unreachable against an ordinarily-authored repository |
| `branch` / `branch list` | Read-only |
| `branch create` / `branch close` | **Mutation** |
| `tag` / `tag list` | Read-only |
| `tag create` | **Mutation** |
| `trust maintainer add` | **Mutation** |
| `init` | **Mutation** (creates `.prikk/`) |
| `commit` | **Mutation** |
| `seal` | **Mutation** |

Traced 2026-08-04 (DC-71) by following each command's implementation to whichever of the mutation
functions above it does or does not reach, including transitively — `rollback-draft`, for instance,
calls no mutation primitive directly in its own file, but reaches one through `Wal::append_patch`.
A name suggesting "plan" or "preview" is a hint, not proof; every row above was traced, not assumed.

**`worktree-status`** is read-only by the same trace, but no CLI command produces the state it
requires: `worktree_status` (`crates/prikk-store/src/worktree_status.rs:88`) calls
`prepare_snapshot_checkout_plan`, which errors unless the target block carries a snapshot blob
(`checkout.rs:94-97`). Nothing in the CLI's `commit`/`seal` path — the only way an ordinary
repository is built — ever sets one; only a test-internal helper does
(`worktree_status/tests.rs:94`, `publish_snapshot_block`). This is a capability gap, not a
mutation/read-only classification error, recorded in `MILESTONES.md` and out of DC-71's scope to fix.

## Non-Linux CI conformance

`.github/workflows/ci.yml`'s `non-linux-build` and `non-linux-verify` jobs run on GitHub's native
`windows-latest` and `macos-latest` runners on every push and pull request, so a regression in this
boundary — the exact defect DC-71 fixed, which shipped undetected because nothing built a non-Linux
target — fails CI immediately rather than being found by a user or the next trial build.
`non-linux-verify` additionally runs the read-only command set (minus `worktree-status`, per the note
above) against a fixture repository authored on Linux, so this is a demonstrated property, not merely
a successful compile.

## What is not covered here

- **Prebuilt non-Linux binaries** are not published. Building from source (`cargo build`/
  `cargo install`) is the only non-Linux install path today; see the [README's install
  section](https://github.com/nabbisen/prikk#install).
- **Non-Linux filesystem durability semantics** are out of scope — read-only paths do not need them,
  and DC-37's boundary is unchanged by DC-71.
- **`macos-latest` is Apple Silicon (`aarch64-apple-darwin`), not x86_64** — GitHub's default since
  the macOS 14 runner image. `windows-latest` is x86_64. Neither the x86_64 macOS nor the arm64
  Windows variant is separately CI-gated as of DC-71; nothing in the fix is architecture-specific
  (it is `#[cfg(target_os = ...)]`, not target-triple-specific), so this is a coverage gap in CI
  breadth, not a known or suspected difference in behavior.
