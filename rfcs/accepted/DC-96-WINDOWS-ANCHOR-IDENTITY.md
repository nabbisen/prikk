# RFC (accepted) - DC-96 Windows Anchor Identity

**Status.** **ACCEPTED by the project owner 2026-08-16**, as remedy R1 of
`.git-exclude/reviewed/DC-87-stage-2-cause-2-ruling-v1.md` §6, together with the owner's ruling that
**Windows mutation does not ship until this lands**. DC-87 Stage 2 stays open until then.

**Author-review independence.** Designed and reviewed by the same agent. Recorded rather than elided;
compensated at implementation review, where the FFI review obligations DC-90 §4.4 names are human work
that cannot be delegated to a gate.

## 0. Why this exists — the finding, by observation

DC-87 Stage 2 made Windows a mutating platform. Its own CI job then demonstrated, in run `31939169047`,
that on Windows prikk's reads **and writes** follow a substituted directory:

| | Linux (control) | Windows |
|---|---|---|
| `materialize_manifest_entries` after root swap | error (correct) | **`Ok(written_files: 1)`** |
| impostor `root/conflict.txt` | `None` | **`Some("replacement")`** |
| retained `.prikk-retained` bytes | 2 → 364 (**delta 362**) | 2 → 2 (**delta 0**) |
| impostor `.prikk` bytes | 2 → 2 (**delta 0**) | 2 → 364 (**delta 362**) |

The same 362 bytes of durability-bearing container storage reach the retained directory on Linux and the
impostor on Windows. **Zero bytes reach the retained directory.** Prikk reports success.

This is **not** the G1 gap accepted at DC-87 prerequisite time. That disclosure
(`docs/src/reference/platform-support.md`) covers a concurrent **reparse-point** substitution timed into
the window between checking one path component and opening the next, and states plainly that a passive,
already-planted reparse point is caught. **A directory rename plus `create_dir` plants no reparse point.**
A reader applying the documented statement would conclude this case is defended.

It is an **integrity** failure, not only a durability one: the existing
`full_verification_retains_wal_objects_trust_and_recovery_diagnosis_after_root_replacement` test seeds the
impostor with **corrupt-signature** objects and the retained layout reports them as repository state.

## 1. What is already settled, so this increment does not re-derive it

- **The mechanism.** `WindowsAuthority` (`crates/prikk-store/src/fsutil/anchored/directory.rs:192`) is
  `{ path: PathBuf }` — a bare path re-walked on every operation, retaining nothing. Linux and macOS hold
  `Arc<AnchoredDirectory>`, a descriptor bound to the object that was checked. `bind` already opens the
  directory and validates it, then **discards the handle**.
- **The scope.** `RepositoryLayout` (`layout.rs:132-137`) holds **two** `MutationRoot`s —
  `worktree_mutation` and `repository_mutation`. Objects, refs, and the WAL are exposed, not only the
  worktree.
- **The choke point.** Every `DurabilityContract` method reaches the filesystem through
  `resolved_existing_path` / `resolved_prepared_path` (`windows.rs:229-243`), both of which walk via
  `WindowsAuthority::{open_child, ensure_child}`. **There is one place to verify, not eleven.**
- **The unsafe question is already ruled.** DC-90 established the owner's position — `unsafe` permitted
  "under control with safety and maintainability preserved" — and built the machine-checked boundary for
  it (`tools/release-policy/src/boundary/unsafe_boundary.rs`): at most one workspace crate may omit
  `forbid(unsafe_code)`, named explicitly in `UNSAFE_EXEMPT_CRATES`, with its own third-party allowlist
  isolated from `placement.rs`'s. **That list is empty today, deliberately. This increment is the first
  legitimate claim on it.**

**Correction on the record.** The ruling that produced this RFC described R1's cost as "the first `unsafe`
in `windows.rs`." That understated it: `unsafe_code = "forbid"` is a **workspace-wide** lint
(root `Cargo.toml`, `[workspace.lints.rust]`), inherited by every member. The owner ruled R1 on the
narrower statement. The correction does not change the ruling's basis — DC-90 anticipated exactly this
situation and built for it, so R1 rests on firmer ground than the original framing conveyed, not weaker —
but it is recorded here because a decision taken on an understated cost must be re-stated accurately.

## 2. The obstacle, stated as a problem rather than a solution

Windows offers no primitive that resolves a child by name **relative to a directory handle**. There is no
`openat`. So the Linux guarantee — the next open is scoped to the handle already checked, not to a
re-walked path string — cannot be reproduced by construction on Windows.

What Windows *does* offer is **identity**: `GetFileInformationByHandle` yields a
`(dwVolumeSerialNumber, nFileIndexHigh:nFileIndexLow)` pair that distinguishes one directory object from
another on a volume. Rust's `std` exposes this only behind the unstable `windows_by_handle` feature, which
is why prikk has not used it so far.

**So the achievable guarantee is detection, not prevention.** Prikk cannot make the swap ineffective on
Windows the way a retained descriptor does on Linux. It can refuse to proceed once the anchor is no longer
the object it validated.

**This must not be over-claimed.** See §5.

## 3. Acceptance criteria

1. **The two existing tests pass on Windows, ungated.**
   `snapshot::tests::worktree_checks_and_writes_remain_on_retained_root` and
   `verify::tests::root_authority::full_verification_retains_wal_objects_trust_and_recovery_diagnosis_after_root_replacement`
   pass on Linux and macOS today and must pass on Windows **without a `cfg` gate**. A gated version of
   either is a failed increment, not a passed one — it is the question being stopped rather than answered.
2. **Both anchors covered.** A negative control for `worktree_mutation` *and* one for
   `repository_mutation`, since the second is the durability-bearing one and only the first is obvious.
3. **Fail closed, and identifiably.** The refusal carries a distinct, greppable diagnostic naming the
   path — an operator who hits this must be able to tell it from an ordinary I/O error.
4. **The exemption is the smallest it can be.** Whatever crate is named in `UNSAFE_EXEMPT_CRATES` must be
   auditable in a single sitting. Naming `prikk-store` — the crate performing every filesystem operation —
   would spend the single exemption on the largest possible blast radius.
5. **`unsafe_code = "forbid"` still holds for every other workspace member**, verified by
   `boundary-check`, not by inspection.
6. **The disclosure is corrected.** `platform-support.md`'s G1 section currently implies this case is
   defended. It must state what is now detected, what is still not, and the difference between them.
7. **Green three-platform CI**, per the standing rule for anything touching filesystem-backed state.

## 4. Non-goals

- **Closing the G1 mid-walk race.** Verifying the anchor does not make intermediate path components safe
  against a concurrent reparse-point substitution. That window stays open and stays documented.
- **Per-component identity.** Only the anchor is verified. Recording an identity for every component of
  every walk is a larger design with a real cost, and the demonstrated failure is anchor replacement.
- **Matching Linux's guarantee.** Detection is not prevention. §5 of the design states the residual.
- **Any new `unsafe` beyond the one call.** The exemption is claimed for `GetFileInformationByHandle` and
  nothing else.
- **Windows `OpenProcess`/`GetExitCodeProcess` liveness** (DC-87's separate follow-up, still owner scope).
  It is unrelated and must not be folded in because both happen to be Win32 FFI.

## 5. Staging

One stage. The change is small, the acceptance test already exists and already fails, and splitting it
would create an intermediate state in which the exemption is claimed but nothing uses it.

**Design:** `rfcs/handoffs/DC-96-windows-anchor-identity/design-v1.md`.
