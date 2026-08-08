# DC-76 §1 — Four Questions, Answered and Reported Before Design

**Handoff followed:** `implementation-handoff-v1.md`. **Governing RFC:**
`rfcs/accepted/DC-76-FILESYSTEM-DURABILITY-CONTRACT.md`. **Cleared scope:** §1 only.

Report only, per the handoff's explicit instruction. No production code changed. The 93-gate count
(28/25/15/13/11/1 across `anchored.rs`/`directory.rs`/`read.rs`/`regular.rs`/`immutable.rs`/`failpoints.rs`)
was independently re-counted from the files, not taken from the RFC, and matches exactly.

## 1. Enumeration — what the store actually requires of a filesystem

Built by reading all six files in full, not by summarizing the RFC. Nine distinct guarantees, each
with its call sites and the primitive providing it today.

| # | Guarantee | Call sites (primary) | Primitive today |
|---|---|---|---|
| G1 | **Root-anchored resolution, no symlink traversal through *any* path component** — every intermediate directory and the final component are opened relative to an already-open parent fd, never by re-resolving a string path from the root. A symlink swapped in anywhere along the path cannot redirect a mutation outside the intended root. | `AnchoredDirectory::open/open_child/open_child_for_read/ensure_child` (`directory.rs`); `regular.rs`'s every `open_*` fn; `read.rs`'s `open_existing_directory_for_read` | `openat`/`mkdirat`/`statat` walked **one path component at a time**, each with `OFlags::NOFOLLOW` / `AtFlags::SYMLINK_NOFOLLOW` |
| G2 | **Atomic content replacement** — a reader never observes a partially-written file; a crash mid-write leaves either the complete old content or the complete new content, never a mix. | `write_file_atomically` (`anchored.rs`) | write to a fresh, same-directory, exclusively-created temp file → `fsync` the file → `renameat` over the destination |
| G3 | **Durable-after-return (two-level fsync discipline)** — once a mutation function returns `Ok`, both the file's content *and* the directory entry naming it survive a crash. POSIX treats these as two independent durability domains; syncing the file alone does not guarantee the rename/create/unlink is durable. | Every mutation fn in `anchored.rs`; `AnchoredDirectory::ensure_child`'s post-create/post-open parent syncs | `File::sync_all()` for content; `AnchoredDirectory::sync()` (`fsync` on the **directory fd**) for the entry |
| G4 | **Exclusive creation — no silent overwrite** | `open_new_regular` (`regular.rs`), used by `write_file_atomically`'s temp, `create_new_file_required`, `publish_immutable_file`'s temp | `OFlags::CREATE \| EXCL` |
| G5 | **No-clobber immutable publication, race-safe** — two racing writers either both succeed with byte-identical content (verified) or one loses cleanly; existing immutable content is never silently replaced. | `publish_immutable_file` → `compare_existing` (`immutable.rs`) | `linkat` (hardlink) install, `EEXIST` → byte-compare fallback, rather than `rename` |
| G6 | **Regular-file-type validation on every "existing file" open** — refuses a symlink, device, or FIFO that raced into the path after resolution, before any read/write touches it. | `validate_regular` (`regular.rs`); `read_file_if_exists`/`stat_file_state_if_exists` (`read.rs`) | `fstat` + `FileType::is_file()` |
| G7 | **Non-blocking opens on every path** — an open can never hang indefinitely because a FIFO or device was substituted at the resolved path. | Every `open_*` in `regular.rs`, `read.rs` | `OFlags::NONBLOCK` |
| G8 | **Concurrent-process-safe directory creation** — two `prikk` processes racing to create the same directory both succeed; a genuine `EEXIST` race is treated as success, not error, and the parent is synced on both the winning and losing path. | `AnchoredDirectory::ensure_child` (`directory.rs`) | `mkdirat` + `EEXIST` fallback to open-and-validate |
| G9 | **Mode-bit isolation** — `fchmod` accepts only permission bits (`0o7777`); a recorded mode carrying `S_IFREG` file-type bits must be masked first. | `set_regular_file_mode_required` (`anchored.rs`) | `fchmod` + explicit `& 0o7777` mask |

The 93 gates are not 93 independent facts — they are the Rust-level `#[cfg]` scaffolding (import gates,
function-body gates, matching `#[cfg(not(target_os = "linux"))]` fallback-error-return gates, two small
error-conversion helpers) around these nine guarantees, repeated per call site. `anchored.rs`'s 28 is the
largest count because every one of its nine public mutation functions carries a paired gate (real impl /
`unsupported_mutation()` fallback), plus ~10 import-level gates.

## 2. Which gates are genuinely Linux-specific vs. incidentally gated

**Verified against `rustix` 1.1.4's own source (GitHub tag `v1.1.4`), not inferred from docs.rs's
default-platform rendering** — the same discipline DC-41's crash-matrix evidence used ("manual
trace-through is not a substitute for running the code"), applied here as "reading a doc banner is not a
substitute for reading the `#[cfg]` attribute."

**Every `rustix::fs` primitive and flag this codebase uses is `#[cfg]`-gated against exotic/embedded
targets only (`redox`, `espidf`, `horizon`, `wasi`) — never against `apple`/macOS specifically:**

| Primitive/flag | `rustix` 1.1.4 gate (from `src/backend/libc/fs/{at,types}.rs`) |
|---|---|
| `openat` | `#[cfg(not(target_os = "redox"))]` |
| `mkdirat` | `#[cfg(not(target_os = "redox"))]` |
| `linkat` | `#[cfg(not(any(target_os = "espidf", target_os = "redox")))]` |
| `unlinkat` | `#[cfg(not(any(target_os = "espidf", target_os = "redox")))]` |
| `statat` | `#[cfg(not(any(target_os = "espidf", target_os = "redox")))]` |
| `renameat` | **no gate at all** |
| `chmodat`/`fchmod` | `#[cfg(not(any(target_os = "espidf", target_os = "wasi", target_os = "redox")))]` |
| `OFlags::NOFOLLOW`, `::DIRECTORY` | `#[cfg(not(any(target_os = "espidf", target_os = "horizon")))]` |
| `OFlags::NONBLOCK`, `::CLOEXEC`, `::EXCL`, `::CREATE` | unconditional |
| `AtFlags::SYMLINK_NOFOLLOW` | unconditional (module gated only against `espidf`/`horizon`/`redox`) |
| `Dir`/`Dir::read_from` | gated against ESP-IDF and Redox only |
| `renameat_with` (RENAME_NOREPLACE — **not currently used**, but relevant to G5) | `#[cfg(any(apple, linux_kernel, target_os = "redox"))]` — **explicitly includes `apple`** |

**Conclusion: G1 through G9, as guarantees, are not Linux-specific at all.** Every primitive
`fsutil/anchored*` calls to provide them is available, at the `rustix` API level, on macOS. The
`target_os = "linux"` gates in this codebase are **incidentally gated** — a scoping decision (DC-37
implemented and tested Linux only), not a primitive-availability boundary. This is the single most
load-bearing finding for sizing the macOS increment: **it is a porting and verification task, not a
redesign.**

**What this does *not* settle, and should not be overclaimed:** `rustix`'s `#[cfg]` gate says the
function *compiles and is callable* on macOS. It says nothing about whether macOS's kernel/filesystem
(APFS) provides the **same durability semantics** once called — in particular, `fsync()` on a directory
fd, and `fsync()` durability guarantees generally, are a documented area of divergence between Linux
(ext4) and macOS (APFS): Linux's `fsync` is well-established as forcing the specific write to stable
storage; macOS's `fsync` has documented weaker guarantees on some storage stacks (Apple recommends
`F_FULLFSYNC` via `fcntl` for a stronger guarantee on macOS, a call `rustix` does not currently wrap).
**This is a semantic question, not a compile-time one, and I cannot answer it from source inspection —
it needs empirical verification against real macOS hardware in whichever increment actually enables
macOS mutation.** Recording it here so it is not silently assumed away by "the gate relaxed cleanly."

## 3. Is DC-41's crash matrix expressible against the contract?

**Yes — its assertions are already phrased at the guarantee level, not the syscall level.** Read the
24-variant evidence table directly (`rfcs/handoffs/DC-41-integrity-evidence-campaign/crash-matrix-coverage-v1.md`)
and a sample of the cited primitive-level tests (`fsutil::tests::failed_mutable_rename_keeps_previous_authoritative_state`,
`fsutil::tests::failed_unlink_retains_file_and_cleanup_sync_reports_removed_state`): every assertion is
`std::fs::read`/`.is_file()`/`.exists()`/record-count/verify-issue-code — **portable observations of
durable state**, never a syscall name or errno. No test in the sample asserts "renameat was called" or
inspects an `Errno` variant as its pass condition.

**The `Point` enum's 24 failure-injection seams (`failpoints.rs`) are themselves plain Rust
thread-local checks** (`check_test_point`/`fail_once`/`fail_after`), not OS-specific — they intercept
calls to named functions (`mutable_rename()`, `required_directory_sync()`, …) inserted at the exact
points the nine guarantees above require a sync or a rename to happen. Nothing about the seam mechanism
is Linux-specific; only **where those seam calls are currently wired from** is Linux-specific, because
today only the Linux implementation exists to wire them into.

**So: the crash matrix is a portable specification (24 named points, each with a guarantee-level
post-failure assertion) currently exercised only through one implementation.** A macOS implementation
that (a) provides the same nine guarantees and (b) calls the same seam functions at the equivalent
points would be checked by the *same* test suite, unmodified, module-gate relaxed. This is a table-stakes
requirement to write into whatever macOS handoff follows, not evidence to produce here — §4 forbids
relaxing the gate.

## 4. Does the contract's shape force a dependency question now?

**Yes for Windows. No for macOS.**

Checked `rustix` 1.1.4's own backend selection, not assumed: its source tree has exactly two backends,
`src/backend/libc/` and `src/backend/linux_raw/`. **There is no Windows filesystem backend anywhere in
the crate** — `windows_syscalls.rs` exists only under `backend/libc/event/` and `backend/libc/io/`
(socket polling/I/O, unrelated to this contract). `rustix::fs` — every primitive in the table above —
**does not compile for `target_os = "windows"` at all.** This matches, and now sources, the handoff's
own framing ("`rustix` is Unix-only").

**Consequence:** the contract as it stands — expressed at the guarantee level (G1–G9) — is
platform-neutral prose. But *implementing* it on Windows cannot be done with `rustix`, and
`ALLOWED_THIRD_PARTY` authorizes `prikk-store` exactly `getrandom` and `rustix` — no Windows-capable
filesystem crate (e.g. a `windows`/`windows-sys` dependency for `CreateFileW`/`ReplaceFileW`/
`MoveFileExW`, or a `same-file`/`fs4`-style abstraction) is currently permitted.

**This is the owner decision the handoff asks me to stop and report, not solve:** whichever increment
eventually enables Windows mutation will need either (a) a new allowed third-party dependency
(`boundary-check`'s policy change, its own review), or (b) a hand-rolled `windows-sys`-free
implementation via `std::os::windows` primitives directly (`std::fs` alone cannot express G1's
no-follow-through-every-component or G5's hardlink-based no-clobber install; Rust's std does expose
`std::os::windows::fs::symlink_metadata`/junction handling but not an anchored-fd `openat` equivalent).
**Not decided here — flagging it now, before any macOS-sized increment is scoped, is the point of asking
early**, matching Q4's own instruction.

**Nothing above changes for macOS**, which stays entirely inside the current `getrandom`+`rustix`
dependency envelope per §2's finding.

## What I did not do

No `target_os` gate touched. No test changed. No new dependency added or proposed as a change — only
identified as a future decision point for Windows specifically. No contract document drafted — that is
design, gated on this report being read first, per §1's own instruction.

## Request

Report only, per the handoff's cleared scope. If §5's "report a weak guarantee, don't fix it" turns out
to apply once a contract document is actually drafted, that is next; nothing in this enumeration itself
surfaced a guarantee weaker than DC-37 claims — every guarantee I found already has primitive-level and,
per DC-41, several have repository-level test coverage.
