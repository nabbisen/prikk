# Platform Support

This page is the authoritative current-state reference for which platforms Prikk runs on and,
concretely, which commands are read-only versus repository mutation. It exists because that
boundary had never been enumerated anywhere — DC-71 traced it once, here, so it does not have to be
re-derived from source on demand and cannot drift silently again (a CI job builds every listed
non-Linux target on every change; see [Non-Linux CI conformance](#non-linux-ci-conformance) below).

## The boundary

**Repository *mutation* requires Linux or macOS.** `crates/prikk-store`'s anchored filesystem
primitives use no-follow, nonblocking, atomic-rename, and no-clobber-install capabilities
([durability and crash recovery](./durability-recovery.md)) with a reviewed implementation on each of
those two platforms — `LinuxDurability`, and, since DC-81/DC-82, `MacosDurability` (G3 uses
`fcntl_fullfsync` in place of `fsync`, measured ~180x slower on the GitHub macOS runner and recorded
in `FINDINGS.md`) — and no reviewed equivalent on any other platform yet
([DC-37](https://github.com/nabbisen/prikk/blob/main/rfcs/accepted/DC-37-REQUIRED-FILESYSTEM-DURABILITY.md)).
Every mutation function's *signature* compiles on every platform; only its *body* has a real
implementor on Linux and macOS, and a caller on any other platform receives a clean runtime error
rather than a build failure or a silent no-op.

**What a Windows implementation would and would not be able to guarantee.** This is stated here rather
than left to be discovered, because it is a real difference and not a coverage gap. Anchored resolution
on Linux and macOS opens each path component with `openat(dirfd, name, O_NOFOLLOW)`, so the handle for a
component is bound to the object that was checked — the next open is scoped to that handle, not to a
re-walked path string. **Windows has no equivalent**: no Win32 primitive takes a directory handle as a
resolution root for opening a child by name, and the natural mitigation — confirming two opens landed on
the same object via a file-index/volume-serial pair — sits behind an unstable Rust API.

A Windows implementation can refuse a reparse point at each component as it is opened, which defeats a
symlink or junction that is already in place. **It cannot close the window between checking a component
and opening the next one.** So a concurrent local process that substitutes a reparse point mid-walk,
timed into that window, is not provably defeated on Windows, while it is on Linux and macOS. A passive,
already-planted reparse point is caught on every platform.

Prikk does not claim otherwise, and this difference is the reason Windows mutation is not shipped on the
strength of the primitives alone.

**The same gap exists on the read path today, in the shipped read-only configuration.** All four non-Unix
fallback read functions resolve a whole path in one operating-system call, so reparse points at
intermediate components are followed — there is no component-by-component walk on that path at all. One
of them, `read_file_if_exists`, additionally does not refuse a symlink at the *final* component, unlike
its three siblings in the same module, which use a no-follow stat. That last one is an asymmetry inside
one file rather than a platform limitation, and it is stated here rather than left implicit because the
guarantee is otherwise described per-function.

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
- **File mode / executable-bit authoring on a platform with no observable POSIX mode** (DC-87
  §3.3/§4.3): worktree authoring never derives a node's recorded mode from such a platform's
  filesystem — an existing node's already-recorded mode is always carried forward untouched, and a
  brand-new file is created non-executable by default, since there is no existing recorded mode to
  inherit and no observed signal to use. This is a missing capability (an executable file's initial
  creation cannot be authored from such a worktree), not data loss — a previously-recorded executable
  bit is never silently dropped from sealed history by this platform difference.
