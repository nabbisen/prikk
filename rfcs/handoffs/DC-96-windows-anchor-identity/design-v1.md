# DC-96 Windows Anchor Identity — design v1

**RFC:** `rfcs/accepted/DC-96-WINDOWS-ANCHOR-IDENTITY.md`. Read §0-§2 first; this document does not
restate the finding.

## 1. Shape — three candidates, and the ruling

The remedy needs `GetFileInformationByHandle`. Where the `unsafe` lives is the design question.

| | Shape | Cost |
|---|---|---|
| **A** | Name **`prikk-store`** in `UNSAFE_EXEMPT_CRATES`, call `windows-sys` inline | Spends the single exemption on the crate performing every filesystem operation — ~750 tests, the whole durability surface. Largest possible blast radius. |
| **B** | Add the **`same-file`** crate; no exemption needed, `forbid` holds everywhere | New third-party crate (+`winapi-util`) in the integrity-critical path. Its SAFETY reasoning is outside every review obligation DC-90 §4.4 places on us. |
| **C** | **A new minimal crate**, first entry in `UNSAFE_EXEMPT_CRATES`, calling `windows-sys` | One new workspace member, with release-lane consequences (§7). |

**Ruled: C.**

DC-90 built "at most one exempt crate, named explicitly" precisely so that `unsafe` is contained in the
smallest auditable surface, and left the list empty as a meaningfully-checked state. C is the shape that
machinery was designed for.

**B deserves its steelman and loses on a specific point.** `same-file` is mature and exists for exactly
this question. But `windows-sys` is **already in our dependency graph** (via `rustix`→`errno`, and
`tempfile`), so C adds *no new supply-chain entrant at all* — only a direct edge to a crate we already
build. And because `windows-sys` supplies the `extern "system"` declaration, our `unsafe` is not
hand-written ABI: it is "call a Microsoft-maintained declaration and read one out-parameter." That is
close to the most auditable FFI can be, and it keeps the SAFETY reasoning inside the review obligation
rather than outside it. B trades auditable code we own for opaque code we do not, while *also* adding
crates.

**A is rejected on acceptance criterion 4.** It is the cheapest diff and the worst outcome.

## 2. The new crate

**Name: `prikk-ffi`.** It is *the* FFI crate, so a future FFI need lands in the already-exempt crate
rather than motivating a second exemption — which DC-90 forbids anyway. **Risk, stated:** a general name
invites accumulation. It is bounded by DC-90's per-block review obligation and by this design's non-goal
that the exemption is claimed for one call and nothing else. If a second call is ever proposed, it is a
design question, not an implementation one.

**Location:** `crates/prikk-ffi/`. Workspace member; on non-Windows it compiles to nothing.

**Its entire public surface:**

```rust
/// Identity of an open filesystem object on Windows: the pair that distinguishes one
/// directory object from another on a volume.
#[cfg(windows)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FileIdentity { /* volume serial + file index, both private */ }

/// Read the identity of an already-open handle. The caller owns opening the handle,
/// including share flags and reparse-point policy.
#[cfg(windows)]
pub fn identity_of(file: &std::fs::File) -> std::io::Result<FileIdentity>;
```

Taking `&File` rather than a path is deliberate: opening — `FILE_SHARE_DELETE`, `FILE_FLAG_BACKUP_SEMANTICS`,
the reparse-point refusal — stays in `windows.rs` where that policy already lives and is already reviewed.
`prikk-ffi` does one thing.

**Fields private, `PartialEq` derived.** Callers compare identities; they never inspect or persist them.
An identity is meaningful only within one boot on one volume and must never reach an object id, a
container, or any on-disk artifact.

### The lint table — a requirement the gate does not enforce

DC-90's check requires the exempt crate to re-declare `undocumented_unsafe_blocks = "forbid"` locally.
**That is the minimum, not the target.** Opting out of `[lints] workspace = true` silently drops
*everything else* in the workspace table too — `missing_docs`, every workspace clippy lint.

**Re-declare the full workspace lint table verbatim, minus `unsafe_code`.** Diff it against the root
`Cargo.toml` and state in the review request that you did. `boundary-check` cannot see this; a human must.

## 3. `WindowsAuthority`

```rust
pub(super) struct WindowsAuthority {
    path: PathBuf,
    identity: FileIdentity,   // captured at bind/ensure_child/open_child
}
```

**Store the identity value, not the handle.** A retained `File` would look like the Linux design and buy
nothing — Windows cannot resolve a child through it — and an inert retained resource invites the belief
that it is providing a guarantee it is not. Say so in the doc comment; the next reader will ask.

- **`bind`** already opens and validates the directory. Capture the identity from **that same handle**,
  before dropping it — do not re-open, which would introduce a race inside the constructor.
- **`ensure_child` / `open_child`** — **call `verify_anchor()` first**, before walking. Then capture the
  identity of the final component for the returned authority, from the handle the walk already opened.
- **`same_as`** — currently `Arc::ptr_eq` on the path with a comment that there is nothing else to compare.
  There is now. Compare `(path, identity)`. Delete the stale comment; leaving it would be worse than never
  having written it.

**`verify_anchor`**: re-open `self.path` no-follow, read its identity, compare with `self.identity`.
Mismatch → fail closed.

### Correction, 2026-08-16 — the placement above was wrong, and the correction changes the shape

**This section originally said verification in `open_child`/`ensure_child` was sufficient, on the grounds
that "every `DurabilityContract` method funnels through the two resolvers."** That claim is true and was
verified against all eleven methods. **It is also scoped to the write side, and I did not check the read
side.** The preflight investigation found the gap
(`.git-exclude/review-request/prikk-dc96-preflight-findings-v1.md`), and I confirmed it:

`open_existing_windows_directory_for_read` (`directory.rs:253-265`) — the resolver behind all four
`WindowsReader` methods — does `root.authority.path.clone()` and walks with `stat_directory_no_follow`.
**It never calls `open_child` or `ensure_child`.** Verification placed as originally written would have
left every read exposed, and
`full_verification_retains_wal_objects_trust_and_recovery_diagnosis_after_root_replacement` — a read
through a stale layout, and the test that most directly demonstrates the finding — **would still have
failed on Windows**, contradicting acceptance criterion 1.

**The lesson is the shape, not the miss.** This gap exists because Stage 1 added a second walker and a
later design reasoned only about the first. Adding a second verification call site leaves exactly that
trap set for a third. So the answer is not "check in two places."

### Ruling: no code can obtain a base path from an authority without verifying it

**Move `WindowsAuthority` into its own module** (`fsutil/anchored/windows_authority.rs`, `cfg(windows)`),
so its fields become genuinely private — today the struct and the read walker share `directory.rs`, and
field privacy binds at module granularity, so privacy buys nothing where it stands.

**The authority exposes no path accessor at all.** Its entire surface is walks that verify first:

```rust
pub(super) fn resolve_existing(&self, relative: &Path) -> Result<PathBuf>;
pub(super) fn resolve_prepared(&self, relative: &Path) -> Result<PathBuf>;
pub(super) fn resolve_existing_for_read(&self, relative: &Path) -> Result<Option<PathBuf>>;
```

Each verifies `self`'s identity, then walks. `directory.rs`'s three free resolver functions become thin
wrappers over these. `PlatformAuthority::{ensure_child, open_child}` stay for
`MutationRoot::{ensure_root, open_root}` and verify identically.

**`resolve_existing_for_read` keeps the tolerant contract exactly** — `Ok(None)` as soon as a component is
absent, which every `WindowsReader` signature depends on to distinguish absence from I/O error. It moves
module, not meaning. **Do not unify it with the required variant**; the read/write contract split was a
considered Stage 1 decision and is not this increment's to revisit.

**Why this over a second call site.** It is the same size of diff and a different class of guarantee: a
future fourth walker cannot compile without going through a verifying method, because there is no way to
get the base path out. That is the distinction DC-90's own module doc draws between a control and a
convention, applied here.

**The empty-relative case is the one that matters most and is easiest to miss.** A file directly in the
anchor (`conflict.txt` in the worktree root — the failing test's exact case) walks *zero* components, so
verification must happen **before** the component loop, not inside it. A loop-body check would leave the
demonstrated failure unfixed while every test that uses a nested path passed. This applies to all three
methods above.

### Cost, and what not to do about it

Every read now pays one extra directory open plus one FFI call. `verify_repository` on a large repository
performs many reads, so this is measurable in principle. Windows already walks the full path per
operation, so one more open is proportionally small — but **measure rather than assume**, and if a
regression appears, **report it rather than weakening the check** (caching a verification across
operations reintroduces exactly the staleness window this increment exists to close).

## 4. Failure mode

Fail closed, with a distinct diagnostic naming the path and saying the anchor was replaced — not a bare
I/O error. An operator who hits this needs to distinguish "someone swapped my repository directory" from
"disk error." Follow whatever error taxonomy `windows.rs` already uses (`io_error` /
`PrikkError::Integrity`); do not invent a new variant without raising it.

## 5. The residual — state it, do not let it be inferred

**Detection, not prevention.** Prikk cannot make the swap ineffective on Windows the way a retained
descriptor does on Linux. After this change:

- **Anchor replacement between operations: detected, refused.** The demonstrated failure.
- **Anchor replacement racing a single operation** (swapped between `verify_anchor` and the open that
  follows): **still possible.** The window is narrowed, not closed.
- **Intermediate path components: unchanged.** G1's documented mid-walk window stays exactly as it is.

`platform-support.md` must say all three. Its current G1 text implies the first case is already defended,
which is what let this ship — **correcting that sentence is part of this increment, not follow-up.**

## 6. Tests

1. **The two existing tests, ungated on Windows.** Acceptance criterion 1. Do not touch their assertions.
2. **A `repository_mutation` negative control**, distinct from the worktree one — a durability-bearing
   write after `.prikk` replacement must be refused. The probe on
   `dc87-stage2-windows-cause2-probe` (`d691625`) is the scenario; convert it into a real assertion.
3. **A positive control that can fail.** Normal operation with no swap must still succeed — otherwise
   "refuse everything" passes criterion 1. This is the check that stops the fix being worse than the bug.
4. **Identity distinguishes, and matches.** Two different directories compare unequal; the same directory
   opened twice compares equal. Without the second, a `FileIdentity` that never equals itself would pass
   every other test here.

Tests 3 and 4 are the negative controls. **Confirm each fails when the fix is reverted**, and say so.

## 7. Policy and release surface — investigate, report, do not decide alone

Adding a workspace member touches machinery I have not traced end to end. Expect at minimum:

- `UNSAFE_EXEMPT_CRATES` — the first entry ever.
- Its **own** third-party allowlist in `unsafe_boundary.rs`, separate from `placement.rs`'s by design.
- `placement.rs`'s `ALLOWED_THIRD_PARTY` — a fixed-size `[(&str, &[&str]); 7]`; the arity changes.
  Note its test `disallowed_third_party_under_target_dependencies_fails`: the dependency will be
  `[target.'cfg(windows)'.dependencies]` and **is** scanned.
- Root `Cargo.toml` — `members`, `default-members`, `[workspace.dependencies]`.
- `windows-sys` **pinned to the version already resolved** (0.61.2 in `Cargo.lock` at time of writing) so
  the graph does not gain a second copy. Verify rather than trust this number.

**Unresolved, and yours to investigate before implementing:** whether the release lane requires a new
published crate to be registered anywhere beyond the manifests — versioning, `check`'s package set,
reference-check. **If `check` or `boundary-check` objects in a way this design does not anticipate, stop
and report rather than adjusting the policy to fit.** A policy edited to accommodate an increment is the
increment marking its own homework.

## 8. Verify before implementing, and report if any is wrong

Design assumptions, each cheap to check and load-bearing:

1. All eleven contract methods funnel through the two resolvers (§3).
2. `windows-sys` exposes `GetFileInformationByHandle` and `BY_HANDLE_FILE_INFORMATION` at a stable path in
   the resolved version, and whether the struct can be zero-initialised **safely** (`Default`) or needs
   `MaybeUninit`. **I have not compiled this.** Use whichever is sound and say which — do not add a second
   `unsafe` block for initialisation if a safe one exists.
3. A crate cannot have `[lints] workspace = true` *and* a local override (DC-90 says this was confirmed
   empirically; confirm it still holds).
4. Holding no directory handle means this change cannot block a user renaming their own repository. The
   existing `FILE_SHARE_DELETE` whole-backend rule should already make that true either way — confirm.

## 9. Gates

The standing set, plus both cross-target clippy runs, plus a **green three-platform CI run** before merge.
`--no-fail-fast` on the mutation jobs is permanent and stays.

Cause 1's fix (`ff98d9e`) is still local on `dc87-stage2-windows` and goes out with this work in one run.
