# RFC (accepted) - DC-99 Windows Capability Parity

**Status.** **ACCEPTED by the project owner 2026-08-17**, as items 3 and 4 of the Windows strengthening
order. Two independent capabilities, one increment, because both add a documented Win32 call to
`prikk-ffi` and DC-90 §4.4's expensive part — a human reading each `unsafe` block against Microsoft's
documentation — is done once for both.

**Author-review independence.** Designed and reviewed by the same agent; recorded, not elided.

## 0. What these close

DC-87 and DC-96 shipped Windows mutation with named residuals. DC-97 and DC-98 proved what was already
there. **These two change what Windows can actually do**, and are the first Windows work since 0.21.0 that
a user would notice.

- **Item 3 — `prikk unlock` has no liveness signal on Windows.** `unlock.rs:90-93` returns
  `PidLiveness::Unknown` unconditionally off Linux/macOS. Since DC-87 made Windows a mutating platform, a
  Windows repository can wedge, and `prikk unlock` is the recovery path — with every stale-lock decision
  resting entirely on the operator, on the one platform where the tool can offer nothing.
- **Item 4 — the anchor identity check is weaker on ReFS.** `prikk-ffi::identity_of` uses
  `GetFileInformationByHandle`'s 64-bit file index, which Microsoft documents as **not unique on ReFS** —
  and Windows 11's Dev Drive is ReFS, the location Microsoft recommends for source repositories.

## 1. What is already settled

- **`prikk-ffi` is the exemption holder.** DC-90's `UNSAFE_EXEMPT_CRATES` names it, its manifest
  re-declares the workspace lint table minus `unsafe_code`, and `boundary-check` verifies both — and
  fails when the exemption is removed, confirmed by negative control in DC-96. **Adding calls here needs
  no new policy surface.**
- **`windows-sys` is already a direct dependency** of `prikk-ffi` and already in the graph via `rustix`
  and `tempfile`. Neither item adds a supply-chain entrant.
- **The liveness contract is advisory and asymmetric**, and this does not change: *"trusted to refuse,
  never to authorise."* `PidLiveness`'s own doc says `DoesNotAppearRunning` is **not proof it is safe to
  clear**.
- **The Unix implementation is the model to mirror, including its subtlety.** `EPERM` maps to
  `AppearsRunning`, not `Unknown`, because *"the kernel found a process to check permissions against — it
  exists, this caller simply cannot signal it."* Windows' access-denied case is the same situation and
  must reach the same answer.

## 2. The obstacles, stated as problems

**Item 3: "does this PID exist" is not one Win32 call.** `OpenProcess` failing distinguishes *no such
process* from *exists but not queryable* only via its error code, and succeeding does not by itself mean
running — a handle to a terminated process still opens until the last handle closes. `GetExitCodeProcess`
returns `STILL_ACTIVE` (259) for a live process, a value a real process may also exit with, so it is
ambiguous on its own.

**Item 4: `FILE_ID_INFO` is not universally available.** `GetFileInformationByHandleEx(FileIdInfo)` is
supported on NTFS and ReFS; behaviour on FAT/exFAT and some network filesystems is not guaranteed.
Requiring it would make prikk refuse to operate on a repository that works today.

Neither obstacle is resolved by picking an API. Both are resolved by deciding **which way an ambiguous
answer must fall**, which is §3.

## 3. The safety rule both items turn on

**Item 3 — every ambiguity resolves away from `DoesNotAppearRunning`.**

A false *not running* can authorise clearing a lock that is genuinely held, putting two writers on one
container — the outcome `prikk unlock`'s whole design exists to prevent. A false *running*, or `Unknown`,
costs an operator an inconvenience. **Only a positively established absence may return
`DoesNotAppearRunning`.** Anything unexpected returns `Unknown`.

**Item 4 — a weaker identity must never silently compare as a stronger one.**

The fallback is permitted; a *silent* fallback is not. Two identities of different provenance must not
compare equal, and the type must make a cross-form comparison impossible or false by construction rather
than by discipline.

## 4. Acceptance criteria

1. **`prikk unlock` returns a real liveness answer on Windows** for both a live PID and an absent one,
   demonstrated by test, not by reading the API contract.
2. **The `unlock` tests lose their Windows `cfg` split.** DC-87 gated
   `every_held_lock_kind_is_enumerated_with_its_own_pid_live` and
   `a_lock_recording_a_nonexistent_pid_is_reported_as_not_appearing_to_run` to assert `Unknown` off
   Linux/macOS. **Those gates come out**, and the strong assertion applies on all three platforms — that
   is what closing this item means.
3. **The access-denied case reaches `AppearsRunning`**, matching Unix's `EPERM` reasoning, with the
   parallel stated at the call site.
4. **Anchor identity uses the 128-bit form where available**, and the two DC-96 acceptance tests
   (`worktree_checks_and_writes_remain_on_retained_root`,
   `full_verification_retains_wal_objects_trust_and_recovery_diagnosis_after_root_replacement`) still pass
   **ungated**, watched to fail with identity comparison neutralised — the bar every control in DC-97 and
   DC-98 met.
5. **Cross-form identity comparison is impossible or false**, by type rather than by convention.
6. **Every new `unsafe` block carries a `SAFETY:` comment justifying handle validity, buffer bounds, and
   the failure path**, and is reviewed against Microsoft's documentation at review time — DC-90 §4.4's
   named human obligation, which no gate discharges.
7. **`platform-support.md` updated**: the `prikk unlock` liveness paragraph and the ReFS caveat both
   describe what is now true, including whatever residual the fallback leaves.
8. Green three-platform CI.

## 5. Non-goals

- **Changing `PidLiveness`'s advisory contract.** It stays trusted to refuse, never to authorise. A real
  Windows primitive makes the refusal better informed; it does not make clearing safe.
- **Solving PID reuse.** Unaddressed on every platform, and out of scope on all of them equally.
- **Requiring NTFS or ReFS.** §3's fallback exists precisely so this is not a new platform requirement.
- **Any FFI beyond these two capabilities.** The exemption is claimed for what §2 names and nothing else.

## 6. Staging

**Stage 1 — liveness.** Self-contained, and criterion 2 makes its completion visible: two `cfg` splits
disappear.

**Stage 2 — identity.** Independent of Stage 1; sequenced second only because criterion 4's negative
control is the more delicate of the two.

**Report the API semantics before wiring, per stage.** DC-96 left `MaybeUninit`-versus-`Default` open and
the investigation settled it in one round; DC-98's classification-first stage found two vacuous controls.
The same shape applies: establish what the calls actually return in each case — including the
access-denied and unsupported-filesystem cases — and report that before building on it.
