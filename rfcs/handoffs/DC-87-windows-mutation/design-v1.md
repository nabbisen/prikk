# DC-87 Windows Mutation — Design v1

**Author.** Architect. **Independence.** Author-reviewed — the standing ceiling.
**Inputs.** DC-87 §1–§6; §3's six prerequisites, all answered and ruled 2026-08-16
(`.git-exclude/reviewed/DC-87-section-3-2-to-3-6-ruling-v1.md`, and §3.1 in the withdrawn RFC 104's
report); RFC 102's completion, recorded as DC-87 §0.
**Status.** Design for review. **No implementation authorized by this document.**
**Target.** 0.21.0.

---

## 1. What this design has to decide, and what it must not

DC-87 §4 already fixes the staging: **Stage 1** makes the seam platform-neutral on Linux and macOS with
no behaviour change; **Stage 2** is `WindowsDurability`. §5 fixes the acceptance criteria and §6 the
non-goals. **None of that is reopened here.**

What is left, and what this document settles: **the shape of the Windows authority type**, **what each
of the eleven contract methods does on Windows**, and **which of the nine guarantees hold, weaken, or
become vacuous** — because §5 criterion 2 requires each one held or documented as weaker with its
operational consequence, and that is a design statement, not an implementation detail.

**The hard constraint from §6, restated because it is the one most likely to be quietly broken:** no
change to the nine guarantees or to `DurabilityContract`'s method set. If the design below appears to
require one, that is a stop-and-report back to DC-87, not a decision to take while implementing.

## 2. The authority type — `WindowsMutationRoot`

**The problem, from §3.1's answer.** `MutationRoot` holds an `AnchoredDirectory` behind a Linux/macOS
`cfg`: a retained directory handle giving `openat(dirfd, name, O_NOFOLLOW)` resolution, so the handle for
a component is bound to the object that was checked. **Windows has no equivalent** — no Win32 primitive
takes a directory handle as a resolution root for opening a child by name, and the identity check that
would substitute for it (`file_index`/`volume_serial_number`) is behind an unstable Rust API.

**The design.** A `WindowsMutationRoot` that holds the accumulated root path and performs a
**component-at-a-time walk on stable `std`**:

1. Extend the accumulated path by one component.
2. Open it with `OpenOptionsExt::custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)`
   — the first so a directory can be opened as a handle at all, the second so the open lands on a reparse
   point rather than transparently following it.
3. **Refuse if the opened component is a reparse point**, checked on the handle just opened, not inferred
   from the whole path.
4. Descend only after that check passes.

**No new dependency. No `unsafe`.** §3.5's answer holds for everything this design specifies.

**What it guarantees, stated as the guarantee and not as the mechanism:** a reparse point substituted for
a plain directory or file at any component is **detected and refused**, provided it is in place when that
component is opened.

**What it does not guarantee, stated because §5 criterion 2 requires it:** the walk is **not
handle-anchored between steps**. Component *N*'s handle does not scope component *N+1*'s open. A
concurrent local process that replaces component *N* **after** its check and **before** the next open is
not detected. Closing that needs either handle-relative resolution (absent at the Win32 layer) or
identity verification (unstable). **This is one precise window, not a general weakening**, and it
requires a concurrent local attacker — a passive, already-planted reparse point is caught unconditionally.

**This gap is already documented** in `docs/src/reference/platform-support.md` (2026-08-16), which is
where §5 criterion 2 requires it, and it was accepted once before on the condition that it be stated
rather than elided.

**Stage 1's seam must therefore express "anchored authority" without presuming a handle-relative open.**
That is the real content of Stage 1: an interface whose Linux/macOS implementor uses `openat` and whose
Windows implementor uses the walk above, with the *guarantee* named at the interface and the *mechanism*
left to the implementor. If Stage 1 cannot draw that seam without changing the method set, §4's
stop-and-report applies.

## 3. The eleven methods on Windows

| Method | Windows | Guarantee |
|---|---|---|
| `durable_append` | open existing, `WriteFile`, `FlushFileBuffers` | **Held.** Content durability on an existing name is what Windows provides |
| `durable_truncate` | open existing, `SetEndOfFile`, flush | **Held** |
| `durable_truncate_to_empty` | as above, length 0 | **Held.** Strict since RFC 102 — must refuse an absent file |
| `create_exclusive` | `create_new(true)`, write, flush | **Held at `init` only.** The new name it creates is not durably recorded; see §4 |
| `remove_if_present` | `std::fs::remove_file`, absent is `Ok(false)` | **Held, conditionally** — see §3.1 below |
| `ensure_directory` | `create_dir_all` | **Held at `init` only**, same caveat as `create_exclusive` |
| `atomic_replace` | `std::fs::rename` over destination | **Weaker** — see §3.2 |
| `promote` | `std::fs::rename` | **Weaker**, and unreachable — see §3.2 |
| `publish_immutable` | no-clobber install | **Weaker**, and unreachable — see §3.2 |
| `set_permission_bits` | **documented no-op** | **Vacuous on Windows** — see §3.3 |
| `durable_directory_entry` | **documented no-op** | **Vacuous on Windows** — see §3.4 |

### 3.1 `remove_if_present` — the `FILE_SHARE_DELETE` discipline is a whole-backend rule

Windows refuses to delete a file another handle holds open without `FILE_SHARE_DELETE`. POSIX `unlink`
never fails for that reason, so the doc comment is true on Linux and not automatically true on Windows.

**Every open this backend performs — for any purpose, including reads — must request
`FILE_SHARE_DELETE`.** This is not local to `remove_if_present`; it is a property of the backend that
`remove_if_present`'s guarantee depends on. State it once at the module level and hold it everywhere,
because a single open that omits it makes a deletion elsewhere fail with no visible connection to the
cause.

### 3.2 The three rename-shaped methods

`atomic_replace`, `promote` and `publish_immutable` all rest on rename semantics that Windows does not
provide equivalently: `ReplaceFileW` has three documented partial-completion codes and no durability
lever, and `MOVEFILE_WRITE_THROUGH`'s same-volume guarantee was investigated to three sources and found
undeterminable.

**Two are unreachable.** `promote` and `publish_immutable` have **zero production callers** (DC-87 §0).
`atomic_replace`'s only remaining callers are the two rebuildable caches, whose absence or corruption
changes no result.

**So the design is: implement all three honestly and document each weaker guarantee**, rather than
refusing or approximating. **Their doc comments are what changes, not the reader's expectations** —
`prerequisite-ruling-v1.md` §4.4's standard. A copied Linux doc comment on a Windows implementation is
the failure mode here, and it is invisible to every gate.

**`atomic_replace`'s weaker guarantee is acceptable only because of where it is used.** If a future caller
puts durability-bearing state behind it, that changes this ruling. Say so in the doc comment, so the next
person adding a caller reads it.

### 3.3 `set_permission_bits` — documented no-op

**Ruled 2026-08-16.** NTFS has no POSIX execute bit; executability is determined by extension and
association. Prikk records mode internally and never derives it from the filesystem, so a round-trip
reads the node's recorded mode, not the OS's belief. NTFS ACLs could carry an execute permission but
require `SetNamedSecurityInfo`-class APIs — a materially larger, security-sensitive surface for a property
nothing reads back.

**The no-op must be documented, not silent.** The implementation states that prikk's recorded mode is
unaffected, that Windows determines executability otherwise, and that a later checkout on Linux restores
the recorded mode faithfully. **A `set_permission_bits` returning `Ok(())` with no comment is
indistinguishable from one that forgot.**

### 3.4 `durable_directory_entry` — documented no-op, and the marker is why it is safe

**`FlushFileBuffers`' documentation covers file, communications-device, named-pipe and volume handles and
says nothing about a directory handle** — there is no contract to implement against. A production
key-value store hit the same wall from the implementation side: `ERROR_INVALID_FUNCTION` on SMB, silent
success on local NTFS.

**It has exactly two callers, both on the worktree** (`worktree.rs:151`, `:199`), and both sit inside the
unclean-shutdown marker's bracket. A crash between the call and the entry becoming durable leaves the
marker dirty, so commit-authoring refuses to infer deletion until the worktree is re-verified. **Worst
case is a spurious refusal, never a silent wrong inference.**

**The implementation's doc comment names the marker as what makes the no-op safe**, and Stage 2 proves it
with a crash-mid-materialization test rather than asserting it.

## 4. The `init`-time exemption, stated once

`create_exclusive` and `ensure_directory` create names, and Windows cannot make a new directory entry
durable. **Both are reachable only during `init`.**

RFC 102's model tolerates this because **an interrupted `init` has nothing to lose**: no user history
exists yet, and `FORMAT` is written last (RFC 102 §14.2), so an incomplete `init` is detectable and a
re-run completes it idempotently. That argument holds on Windows unchanged — it depends on ordering, not
on a durability primitive.

**Stage 2 must demonstrate it rather than inherit it:** interrupt `init` on Windows, observe that the
repository is refused or completed rather than silently half-formed.

## 5. What Stage 2 must not do

- **Not repair §3.6's read-path gaps.** They are documented in `platform-support.md` and are their own
  work. A mutation backend and a read-path security fix have different proofs.
- **Not widen `hex_prefix` or `ref_name_storage_key`**, or add a dependency, for any of the above. Nothing
  in this design needs it.
- **Not present any of §3's weaker guarantees as held.** §5 criterion 2 is explicit: *"the method returns
  `Ok`" is not evidence that the guarantee behind it holds.*

## 6. Open items Stage 1 must resolve before Stage 2

1. **The seam's exact shape** (§2's last paragraph) — can "anchored authority" be expressed without
   presuming handle-relative resolution, keeping the method set unchanged? This is the one question that
   can still send DC-87 back.
2. **Where `FILE_SHARE_DELETE` is enforced** so it cannot be omitted per-call-site — a single open helper,
   not a convention.
3. **The `target_os` count** §5 criterion 6 asks for, measured before Stage 1 so the delta is real.
