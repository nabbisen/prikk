# RFC (proposed) - DC-87 Windows Mutation

**Status.** **PROPOSED** — needs the project owner's acceptance before design begins.
**Independence.** Author-reviewed — the standing ceiling.
**Arises from.** The owner's direction of 2026-08-10: mutation expansion on Windows and macOS as soon
as possible, *with clean architecture and a safe process*. macOS landed as DC-81; this is the other
half. Also from **DC-82's own criterion 3**, which was reported **not met** and deferred here by name:
"the sub-contract layer is per-platform types and primitives, deferred to the Windows increment."
**Target.** 0.20.0. **Status-claim criterion 6.**

## 1. What is already settled, so this increment does not re-derive it

- **The contract exists and is guarantee-named, not syscall-named** (DC-76). Eleven methods, nine
  guarantees. A Windows backend implements the trait; it does not negotiate the trait.
- **Dispatch is already collapsed** (DC-82). `ACTIVE_DURABILITY` in `fsutil/anchored.rs:50-55` is the
  single gated constant; all eleven call sites are unconditional. A third real implementor is one more
  arm there, not eleven more arms.
- **Windows already builds and already runs read-only.** It resolves to `NoDurability` today, and CI
  gates `non-linux build (windows-latest)` and `non-linux read-only conformance (windows-latest)` on
  every push. This increment converts Windows from the fallback implementor to a real one; it does not
  introduce Windows support from nothing.
- **Path policy is already Windows-hardened, cross-platform** (DC-72). The validator rejects
  backslashes, colons, non-ASCII, control bytes, components ending in a space or dot, and the Windows
  reserved device stems — **on every host, including Linux**. Alternate data streams and drive-relative
  forms are unreachable because `:` is rejected outright. **This is the single biggest de-risking fact
  in this RFC:** the usual Windows-port hazard is path policy, and prikk paid for it two increments
  early. What is left is genuinely the durability backend.
- **DC-51's placement gate covers target-specific dependency tables.** I verified
  `boundary/placement.rs:51-68`: `dependency_entries` collects `[target.*.dependencies]` and
  `[target.*.build-dependencies]` as well as the plain tables. A Windows-only crate added under
  `[target.'cfg(windows)'.dependencies]` therefore **cannot** slip past `ALLOWED_THIRD_PARTY`. That is
  the friction working as designed, and §3.5 keeps it.

## 2. The obstacle, stated as a problem rather than a solution

DC-82 collapsed the *dispatch*. It did not, and did not claim to, make the layer beneath it
platform-neutral. That layer is Unix-shaped in its types, not merely in its calls:

- `MutationRoot` (`fsutil/anchored/directory.rs:22-27`) holds `Arc<AnchoredDirectory>` wrapping a
  `rustix::fd::OwnedFd` — and holds it only under `#[cfg(any(target_os = "linux", target_os = "macos"))]`.
  Off those platforms the struct silently loses its authority field and keeps only a `PathBuf`.
- `directory.rs`, `regular.rs`, `immutable.rs`, and `read.rs` are shared primitives that `LinuxDurability`
  and `MacosDurability` both build on. Every one of them is gated the same way. Windows shares none of it.
- Consequently `MutationRoot::open` succeeds on Windows and returns a root with **no anchoring at all**,
  and `fallback_path` (`directory.rs:104-107`) simply joins the relative path onto a stored `PathBuf`.

So Linux and macOS differ by a handful of primitives inside one shared shape. **Windows does not fit the
shape.** `openat` has no Win32 equivalent, and the fd-anchored, component-at-a-time, no-follow walk that
G1 is defined in terms of is the load-bearing assumption of the entire layer.

This is not an argument for a redesign. It is the reason §3 asks before §4 designs.

## 3. Blocking prerequisites — report before designing

The architect's design assertions on platform work have needed correction repeatedly this cycle. These
are questions, not rulings. **Answer them against the code and the platform documentation, and report;
do not begin design on any of them.**

**3.1 — Can G1 be held on Windows, and at what cost?** G1 is root-anchored resolution, one component at
a time, no-follow on every component including the last, such that a symlink or junction swapped in
mid-path cannot escape the root. Identify what is actually achievable: `CreateFileW` with
`FILE_FLAG_BACKUP_SEMANTICS` and `FILE_FLAG_OPEN_REPARSE_POINT` per component (Win32, but report whether
a directory handle can serve as a *resolution root* for the next component at all), versus
`NtCreateFile` with `OBJECT_ATTRIBUTES.RootDirectory` (native, `unsafe`, and a stability question of its
own). **If G1 cannot be held on Windows to the standard Linux holds it to, say so plainly.** A documented
platform difference — or a decision that Windows mutation is refused until it can be held — is an
acceptable answer. An approximation presented as G1 is not.

**3.2 — Is `durable_directory_entry` implementable on NTFS?** This is G3 in its most direct form, and the
question I most expect to yield a finding. `FlushFileBuffers` on a directory handle is not documented to
flush the directory's entry list the way `fsync` on a directory fd does. If the guarantee cannot be
obtained: state the honest weaker guarantee, and then answer the consequence — **does DC-38's ref-publication
crash-recovery reasoning still hold under it?** That reasoning was written against a durability primitive
Windows may not have. Report the impact; do not absorb it.

**3.3 — What does G9 mean where there is no POSIX mode?** `set_permission_bits` takes a recorded mode
carrying file-type bits. Windows has `FILE_ATTRIBUTE_READONLY` and ACLs, and no mode word. Report the
options — map the owner-write bit only, refuse, or no-op — and, for each, what happens to `ChangePerm`
replay fidelity for a repository authored on Unix and checked out on Windows. This is a correctness
question about cross-platform history, not a convenience question.

**3.4 — Do `remove_if_present` and `promote` hold their stated guarantees?** Windows refuses deletion
and rename of files held open without `FILE_SHARE_DELETE`, and its rename-over-existing semantics
(`MoveFileExW` with `MOVEFILE_REPLACE_EXISTING`, or `ReplaceFileW`) differ from `renameat`. Report
whether each method's doc comment remains true on Windows, or becomes true only under conditions the
doc comment does not state.

**3.5 — What dependency, if any?** Prefer `std` where `std` suffices. If a crate is needed, report the
crate, the exact feature set, its transitive package count, and its effect on `Cargo.lock` **before
adding it**. It goes in root `[workspace.dependencies]` with `{ workspace = true }` in the member
manifest — never a literal version — and `ALLOWED_THIRD_PARTY` (`boundary/placement.rs:5-13`) needs a
deliberate amendment, which is the architect's to rule on, not the increment's to make quietly. Note
that `rustix` is currently declared **unconditionally** in `prikk-store/Cargo.toml:20`; report whether
this increment should target-gate it, and what that does to the lock.

**3.6 — A read-path question that is not this increment's to fix.** `read.rs`'s non-Unix branch
(lines 79-82, 130-133, 189-192, 245-248) uses `std::fs::read`, `symlink_metadata`, and `read_dir` on a
joined path — resolving the whole path in one call rather than component-by-component with no-follow.
If that is right, **Windows holds weaker path guarantees than Linux today, for read-only operation,
independent of mutation.** Confirm or refute it and **report it as a finding**. Do not repair it inside
this increment: a read-path security fix and a new-platform mutation backend have different proofs, and
bundling them would make a failure unattributable — which is the whole reason DC-82 was split out of
DC-81.

## 4. Staging

DC-82's lesson, in its own words: "a behaviour-preserving refactor and a new-platform backend have
entirely different proofs. Bundled, a reviewer cannot tell which half a failure came from." That applies
here with more force than it did there, because the refactor is larger.

**Stage 1 — make the layer platform-neutral at its seams. Linux and macOS only. No behaviour change.**
Give `MutationRoot` a per-platform authority type behind one interface, so the struct is no longer a
different struct on different platforms, and so the Windows backend in Stage 2 is *written against* the
final shape rather than migrated into it. Proof: every existing test passes **unchanged** on Linux and
on the macOS CI job; production `target_os` count in `fsutil/` reported before and after; DC-76's nine
negative controls still fail when their guarantee is removed; DC-71 still demonstrated.

**Stage 2 — `WindowsDurability`.** Proof is the conformance suite actually running on Windows, not a
successful compile.

Each stage merges only after a green CI run on **all three** platforms. The standing rule applies to
both stages; both touch filesystem-backed state throughout.

**If Stage 1's answers from §3 show the seam cannot be drawn without changing `DurabilityContract`'s
method set or the nine guarantees — stop and report.** That is a design question that comes back here,
not a decision to take inside an increment.

## 5. Acceptance criteria

1. §3's six questions answered and reported **before** design.
2. **Every one of the nine guarantees is either held on Windows or documented as weaker in
   `docs/src/reference/platform-support.md`, with the reason and the operational consequence.** No
   guarantee may be silently downgraded, and "the method returns `Ok`" is not evidence that the
   guarantee behind it holds.
3. A new **`Windows mutation test suite`** CI job, mirroring `macOS mutation test suite`
   (`ci.yml:74-90`, including the `cargo fetch --locked` step DC-81's addendum-2 B1 found necessary),
   running the DC-76 conformance suite on `windows-latest` and green.
4. **DC-76's nine negative controls demonstrated on Windows**, or a reported reason each one cannot be
   — following DC-76's own precedent, where two could not be cleanly demonstrated and were reported as
   findings rather than dropped.
5. **DC-71 preserved, demonstrated not asserted:** `prikk-store` still compiles for a target with no
   implementor, mutation there still fails at **runtime** rather than at build time, and read-only
   commands still work.
6. **Answer the owner's question explicitly, with a number.** The owner asked on 2026-08-10 whether
   `#[cfg(any(target_os = "linux", target_os = "macos"))]` can be removed once Windows mutation lands.
   Report the final production `target_os` count in `fsutil/`, the before/after delta, and — for each
   gate that remains — why it is irreducible. DC-82's single-digit target was the architect's to
   miscalibrate once already; this increment reports the honest number rather than being held to a
   figure set from outside the code.
7. **No cross-platform history divergence.** A repository authored on Linux, mutated on Windows, and
   verified on Linux must produce identical object ids and a clean `verify`. Demonstrated end to end,
   not reasoned about.
8. Gate set per `EXECUTION-ORDER.md` §6 rule 9 **as amended** — the canonical nine plus macOS and
   Windows cross-target clippy.

## 6. Non-goals

- **Any change to the nine guarantees or `DurabilityContract`'s method set.** If the port appears to
  require one, stop and report (§4).
- **Any path-policy change.** DC-72 already rejects the Windows-hostile forms cross-platform. Adding
  `COM0`/`LPT0`, or anything else to that validator, is a separate increment.
- **Repairing §3.6's read-path gap.** Report it; do not fix it here.
- **arm64 Windows CI.** `windows-latest` is x86_64 and stays so, matching DC-71's recorded position.
- **Any new command surface.** No user-visible command changes in either stage.
- **Performance work.** DC-81 measured `fcntl_fullfsync` at 180x `fsync` and recorded it rather than
  acting on it. Windows measurements are recorded the same way; NFR-PERF-01 remains its own increment.
