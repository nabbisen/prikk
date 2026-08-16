# Platform Support

This page is the authoritative current-state reference for which platforms Prikk runs on and,
concretely, which commands are read-only versus repository mutation. It exists because that
boundary had never been enumerated anywhere — DC-71 traced it once, here, so it does not have to be
re-derived from source on demand and cannot drift silently again (a CI job builds every listed
non-Linux target on every change; see [Non-Linux CI conformance](#non-linux-ci-conformance) below).

## The boundary

**Repository *mutation* requires Linux, macOS, or Windows.** `crates/prikk-store`'s anchored
filesystem primitives use no-follow, nonblocking, atomic-rename, and no-clobber-install capabilities
([durability and crash recovery](./durability-recovery.md)) with a reviewed implementation on each of
those three platforms — `LinuxDurability`, `MacosDurability` (DC-81/DC-82; G3 uses
`fcntl_fullfsync` in place of `fsync`, measured ~180x slower on the GitHub macOS runner and recorded
in `FINDINGS.md`), and `WindowsDurability` (DC-87 Stage 2) — and no reviewed equivalent on any other
platform yet
([DC-37](https://github.com/nabbisen/prikk/blob/main/rfcs/accepted/DC-37-REQUIRED-FILESYSTEM-DURABILITY.md),
superseded for Linux/macOS/Windows by DC-87).
Every mutation function's *signature* compiles on every platform; only its *body* has a real
implementor on Linux, macOS, and Windows, and a caller on any other platform receives a clean runtime
error rather than a build failure or a silent no-op.

**What Windows actually guarantees and does not, for path resolution (G1).** This is stated here
rather than left to be discovered, because it is a real difference and not a coverage gap. Anchored
resolution on Linux and macOS opens each path component with `openat(dirfd, name, O_NOFOLLOW)`, so the
handle for a component is bound to the object that was checked — the next open is scoped to that
handle, not to a re-walked path string. **Windows has no equivalent**: no Win32 primitive takes a
directory handle as a resolution root for opening a child by name, so the walk itself is always a
re-walked path string on Windows, by construction.

Windows' actual implementation (`crates/prikk-store/src/fsutil/anchored/windows.rs`) refuses a reparse
point at each component as it is opened (`FILE_FLAG_OPEN_REPARSE_POINT` plus a post-open attribute
check), which defeats a symlink or junction that is already in place. **It does not close the window
between checking a component and opening the next one.** So a concurrent local process that
substitutes a reparse point mid-walk, timed into that window, is not provably defeated on Windows,
while it is on Linux and macOS. A passive, already-planted reparse point is caught on every platform.
**This mid-walk window is unchanged by anything below** — DC-96 verifies the anchor a walk starts
from, not each intermediate component of the walk itself.

Prikk does not claim otherwise. This gap was accepted, once, on the condition that it be stated rather
than elided (`prerequisite-ruling-v1.md` §4.1) — this section is that statement.

**Anchor replacement (DC-96 Windows Anchor Identity).** DC-87 Stage 2's own CI job demonstrated a
second, wider gap: renaming a repository's root (or `.prikk` specifically) aside and creating a fresh
directory at that path redirected both reads and writes — including objects, refs, and the WAL, not
only the worktree — into the impostor, silently, with prikk reporting success. This was not the G1
mid-walk race above; it needed no reparse point at all, and the disclosure as it stood would have led a
reader to conclude it was already defended. It was not.

**Fixed, as prevention, not merely detection.** An earlier version of this fix stored only a path
string plus an identity value and refused on mismatch — detection, and wrong: it could satisfy only
half of each acceptance test, since the tests require operations to keep working correctly against
the *retained* directory after a replacement, not merely refuse
(`.git-exclude/reviewed/DC-96-implementation-ruling-v1.md` §2-§4). **`WindowsAuthority`
(`crates/prikk-store/src/fsutil/anchored/windows_authority.rs`) instead retains the directory handle
it was bound to.** Windows has no `openat`-equivalent to resolve a child by name against that handle,
but a retained handle still follows its object across a rename — `GetFinalPathNameByHandle` returns
its *current* path. Every walk re-derives that current path from the retained handle first, confirms
via identity (`GetFileInformationByHandle`'s `(volume serial number, file index)` pair) that the
object found there is still the one that was bound, and only then walks forward from it. Both Win32
calls go through `prikk-ffi` — `crates/prikk-ffi`, the one workspace crate permitted `unsafe` per
DC-90. Three residual properties, stated precisely rather than left to be inferred:

- **Anchor replacement *between* operations: prevented while a repository is open, in two different
  ways.** For `.prikk` (`repository_mutation`), the retained handle follows a rename that does
  succeed — the gap the CI job demonstrated, now closed by continuing correctly against the retained
  directory rather than merely refusing. For the worktree root (`worktree_mutation`), the outcome is
  stronger still: **NTFS refuses the rename outright**, because `RepositoryLayout` retains a nested
  handle on `.prikk` inside it — see the next paragraph for what this means operationally.
- **Anchor replacement racing a *single* operation** — swapped between the post-open identity check
  and the open that immediately follows it — **is still possible.** The window is narrowed from "any
  time before the next operation" to one check-then-open pair; it is not closed, because Windows still
  offers no `openat`-equivalent to close it by construction the way Linux and macOS do.
- **Intermediate path components are unchanged** — this is exactly the G1 mid-walk window above, and
  DC-96 does not touch it.

**A fourth property, user-facing rather than adversarial: while a prikk command holds a repository
open on Windows, that repository's directory — and any directory containing it — cannot be renamed or
moved by any process, prikk included.** NTFS refuses to rename a directory that contains an open
handle anywhere within it, unconditionally, and prikk retains one on `.prikk` for the duration of a
command (`RepositoryLayout::init`/`open`, `crates/prikk-store/src/fsutil/anchored/
windows_authority.rs`). This is not a bug report waiting to happen; it is the mechanism above,
observed from the other side. It is bounded to a single command's execution — `prikk` has no daemon —
so the window is as long as one invocation takes, not a whole working session.

**The 64-bit file index is not reliable on every filesystem — identity is the secondary check, not
the sole mechanism, which is why this does not weaken the fix.** Per Microsoft's own documentation
for `BY_HANDLE_FILE_INFORMATION` (`nFileIndexHigh`/`nFileIndexLow`): *"The ReFS file system... includes
128-bit file identifiers... The 64-bit identifier [`nFileIndexHigh`/`nFileIndexLow`] is not guaranteed
to be unique on ReFS"* — ReFS callers needing a reliable id are directed to `GetFileInformationByHandleEx`
with `FileIdInfo` instead. Windows 11's Dev Drive, Microsoft's own recommended location for source
repositories, is ReFS. This matters less than it would have under the detection-only design: the
primary mechanism here is the retained handle following the renamed object via
`GetFinalPathNameByHandle`, which does not depend on the file index at all; identity is only the
confirmation that what was found at the re-derived path is the same object, not what determines where
the walk goes. A coincidental file-index collision on ReFS would need to land on the object the walk
already, independently, arrived at correctly — not redirect it. `FILE_ID_INFO` is not used here; if a
future increment needs a stronger per-filesystem guarantee, that is its own design question.

### The nine `DurabilityContract` guarantees on Windows

| Method | Windows guarantee |
|---|---|
| `durable_append` | **Held.** Content durability on an existing name is what Windows provides. |
| `durable_truncate` / `durable_truncate_to_empty` | **Held.** |
| `create_exclusive` | **Held at `init` only.** The new directory entry it creates is not itself durably confirmed — see the `init`-time exemption below. |
| `ensure_directory` | **Held at `init` only**, same caveat. |
| `remove_if_present` | **Held**, conditional on every open in the Windows backend requesting `FILE_SHARE_DELETE` — enforced in one place ([`open_no_follow`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/fsutil/anchored/windows.rs)), not per call site. |
| `atomic_replace` | **Weaker.** `std::fs::rename` over the destination, with no durability lever asserted for the rename itself (`MOVEFILE_WRITE_THROUGH`'s same-volume guarantee was investigated to three independent primary sources and found genuinely undeterminable). Acceptable only because its remaining callers are two rebuildable caches. |
| `promote` | **Weaker**, same rename caveat, and unreachable — zero production callers. |
| `publish_immutable` | **Weaker**, `std::fs::hard_link`-based no-clobber install with the same rename-adjacent caveat, and unreachable — zero production callers (the standing G5 orphan finding). |
| `set_permission_bits` | **Vacuous — a documented no-op.** NTFS has no POSIX execute bit; prikk's own recorded mode is never derived from the filesystem, so a round-trip checkout on Linux restores the node's recorded mode faithfully regardless of what this method does on Windows. |
| `durable_directory_entry` | **Vacuous — a documented no-op.** `FlushFileBuffers`'s own documentation covers file, communications-device, named-pipe, and volume handles and says nothing about a directory handle — there is no contract to implement against. Safe because both production callers sit inside the worktree unclean-shutdown marker's bracket (`worktree_marker.rs`): a crash between this call and the entry becoming durable leaves the marker dirty, and commit-authoring refuses to infer deletion until the worktree is re-verified. |

**The `init`-time exemption.** `create_exclusive` and `ensure_directory` create names, and Windows
cannot make a new directory entry durable. Both are reachable only during `init`. This is tolerated
because an interrupted `init` has nothing to lose — no user history exists yet, and `FORMAT` is written
last — so an incomplete `init` is detectable and a re-run completes it idempotently. That argument
depends on ordering, not on a durability primitive, so it holds on Windows unchanged.

**DC-76's nine negative controls, per guarantee, on Windows (DC-97).** Stage 2 shipped with none of
the nine demonstrated there; DC-97 classified each individually rather than leaving one blanket
statement, since the honest answer differs guarantee by guarantee:

| Guarantee | Windows control | Why |
|---|---|---|
| G1 (root-anchored, no-follow) | **Yes** — `windows::tests::a_reparse_point_substituted_for_a_directory_component_is_refused` | The one control this stage's own CI job proved was not silently skipping: an earlier version returned `Ok` without asserting anything if it lacked the privilege to create a test reparse point; confirmed via a loud failure that never fired that `windows-latest` grants the privilege and the refusal assertion genuinely runs |
| G2 (atomic content replacement) | **Yes** — `conformance::create_exclusive_refuses_an_already_occupied_path`'s sibling shape, `atomic_replace_overwrites_existing_content` | Same shared-assertion shape Linux/macOS use for the exclusive-creation case; the replace case is `windows::tests`' own test |
| G3 (durable-after-return) | **No — unbuilt, not impossible.** | The failpoint injection mechanism itself is plain, platform-neutral Rust; what's missing is call sites inside `windows.rs`'s own `DurabilityContract` implementation invoking it, the same as every Unix implementor already does. Building that wiring is its own increment, not a Windows-specific technical barrier |
| G4 (exclusive creation) | **Yes** — `conformance::create_exclusive_refuses_an_already_occupied_path`, `&WindowsDurability` | Same shared assertion body Linux/macOS use — no Windows-specific test needed, the file's own architecture already covers a new platform |
| G5 (race-safe no-clobber publication) | **Partial.** `object_store::tests::immutable::{same_object_id_with_different_signature_transport_is_rejected, malformed_wrong_id_and_wrong_type_existing_files_are_rejected}` — **yes**. The concurrent-race variants (`object_store::tests::races`) — **no**, same reason as G3: the thread-synchronized race needs the same unbuilt failpoint call site, and the cross-process race variants share a test helper with the failpoint-only tests in a way this stage did not attempt to split under time pressure | `object_store.rs`'s own test gate was Linux/macOS-only for a stale reason (predated Windows mutation); narrowed, and what remains gated is gated for the reasons above, not by inertia |
| G6 (regular-file validation) | **No — no Windows analogue exists**, not merely unbuilt | Linux/macOS evidence uses a FIFO, an ordinary-path filesystem object with no Windows equivalent reachable the same way: Windows named pipes live in a separate `\\.\pipe\` namespace, not placeable inside an anchored directory tree. Windows' own reserved-device-name special files (`CON`, `NUL`, …) are already refused one layer up, at `RepoPath::parse`, before ever reaching this guarantee's own code path |
| G7 (non-blocking opens) | **No**, same reason as G6 | |
| G8 (concurrent-safe directory creation) | **No — unbuilt, not impossible**, and the existing Windows test is weaker than the guarantee | `windows::tests::ensure_directory_is_idempotent_under_a_concurrent_creator_shape` calls the same operation twice sequentially in one thread — idempotency, not a proven race. Linux's real control needs `set_directory_create_barrier_for_test`, whose wait point (`wait_at_directory_create`) is called only from the Unix `AnchoredDirectory::ensure_child` — never from `windows_authority.rs`. The underlying code already tolerates a concurrent `AlreadyExists` winner (read, not yet proven under a synchronized race) |
| G9 (mode-bit isolation) | **Yes, as a documented no-op** — `windows::tests::set_permission_bits_is_a_documented_noop` | Two independent reasons this is not negatively controllable further, not one: NTFS has no execute bit to mask (Windows), and `fchmod` already masks non-permission bits at the kernel level regardless of what this code does (Linux) — `conformance.rs`'s own shared assertion function reads back POSIX mode bits and so was never given a Windows wrapper; the no-op's own, differently-shaped test is the right coverage instead |

Reported rather than silently left implicit, per DC-76's own precedent (two of its original nine also
could not be cleanly demonstrated on the platforms it shipped on, and were reported rather than
dropped).

**The same gap exists on the read path today, in the shipped read-only configuration.** All four non-Unix
fallback read functions resolve a whole path in one operating-system call, so reparse points at
intermediate components are followed — there is no component-by-component walk on that path at all. One
of them, `read_file_if_exists`, additionally does not refuse a symlink at the *final* component, unlike
its three siblings in the same module, which use a no-follow stat. That last one is an asymmetry inside
one file rather than a platform limitation, and it is stated here rather than left implicit because the
guarantee is otherwise described per-function.

**`prikk unlock`'s PID liveness check is held on Linux/macOS and documented-weaker on Windows** — the
same shape as `set_permission_bits`/`durable_directory_entry` above, outside the `DurabilityContract`
table because it lives in a different module (`crates/prikk-store/src/unlock.rs`). Linux/macOS have a
real `kill(pid, 0)` primitive via `rustix::process::test_kill_process`; every other platform, Windows
included, has always stubbed `check_pid_liveness` to an unconditional `PidLiveness::Unknown`
(`unlock.rs:90-93`) — no `OpenProcess`/`GetExitCodeProcess` equivalent has been written. This is safe:
`Unknown` is the conservative outcome and never authorizes clearing a lock, the same as a negative
result on Linux/macOS. **The operational consequence is what changed with Stage 2, not the stub
itself.** Before Windows could mutate, it could never hold a lock, so the stub was unreachable in
practice. Stage 2 makes Windows a mutating platform: a Windows repository can now wedge, and on
Windows, `prikk unlock` returns no positive liveness signal at all — every stale-lock decision there
rests entirely on the operator, with no automated "this process is still running, don't clear it"
refusal available. A real Windows primitive is tracked as follow-up scope, not part of Stage 2.

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

**`macos-mutation` and `windows-mutation`** (DC-81, DC-87 Stage 2) run the full workspace test suite
natively on `macos-latest` and `windows-latest`, since neither developer nor architect can run either
platform locally as part of this project's own environment — the CI job existing and being green *is*
the verification for each backend, not a supplement to one done elsewhere.

**`windows-mutate` → `linux-mutate-reference` → `verify-cross-platform-history`** (DC-87 Stage 2
criterion 7) close the one property none of the jobs above can: that repository *authored on Linux,
mutated on Windows, and verified on Linux* produces identical object ids and a clean `verify`. The
Linux-built fixture is mutated identically on both platforms with the same deterministic signing seeds
`fixture` already uses; the Windows-mutated repository is then handed to a Linux job, which runs
`prikk verify` against it directly and diffs its recorded object ids against the independently-computed
Linux reference. Every other job in this workflow is one platform verifying itself — this is the only
one where a different platform checks Windows' output.

## What is not covered here

- **Prebuilt non-Linux binaries** are not published. Building from source (`cargo build`/
  `cargo install`) is the only non-Linux install path today; see the [README's install
  section](https://github.com/nabbisen/prikk#install).
- **DC-76's nine negative controls are not demonstrated on Windows** — see "The nine
  `DurabilityContract` guarantees on Windows" above. The failpoint mechanism they need is
  Linux/macOS-only.
- **`macos-latest` is Apple Silicon (`aarch64-apple-darwin`), not x86_64** — GitHub's default since
  the macOS 14 runner image. `windows-latest` is x86_64. Neither the x86_64 macOS nor the arm64
  Windows variant is separately CI-gated, and Windows arm64 is untested entirely; nothing in the
  Windows backend is architecture-specific (it is `#[cfg(target_os = ...)]`, not
  target-triple-specific), so this is a coverage gap in CI breadth, not a known or suspected
  difference in behavior.
- **File mode / executable-bit authoring on Windows, or any platform with no observable POSIX mode**
  (DC-87 §3.3/§4.3): worktree authoring never derives a node's recorded mode from such a platform's
  filesystem — an existing node's already-recorded mode is always carried forward untouched, and a
  brand-new file is created non-executable by default, since there is no existing recorded mode to
  inherit and no observed signal to use. `set_permission_bits` is correspondingly a documented no-op
  on Windows (see the guarantee table above) — this is a missing capability (an executable file's
  initial creation cannot be authored from such a worktree), not data loss — a previously-recorded
  executable bit is never silently dropped from sealed history by this platform difference.
