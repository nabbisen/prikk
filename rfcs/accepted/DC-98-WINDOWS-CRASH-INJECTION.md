# RFC (accepted) - DC-98 Windows Crash Injection

**Status.** **ACCEPTED by the project owner 2026-08-17**, on the ruling that *"correctness, stability,
robustness and security win against initial effort / cost."* Chosen over three cheaper Windows-strengthening
options because it is the only one that converts a belief into evidence rather than improving something
already safe.

**Author-review independence.** Designed and reviewed by the same agent; recorded, not elided.

## 0. Why this exists

**Crash-safety on Windows is implemented but unverified.**

Prikk cannot demonstrate durability by writing a file and reading it back — the data may be sitting in the
OS cache and would survive the read regardless. The only way to pin *"this survives a crash"* is to inject
a failure at the exact syscall boundary and assert prikk fails safe. `conformance.rs`'s own G3 row records
what happens without that discipline: a draft test *"passed even with the `fsync` call deleted."*

Prikk has that machinery — `fsutil/anchored/failpoints.rs`, a `Point` enum and a thread-local, **plain
platform-neutral Rust with nothing unix-specific in it.** The Unix implementors use it heavily:

| File | failpoint call sites |
|---|---|
| `linux.rs` | 20 |
| `macos.rs` | 20 |
| `windows.rs` | **0** |
| `windows_authority.rs` | **0** |

The shared helpers that also call it (`regular.rs`, `immutable.rs`, `directory.rs`, `read.rs`) are
themselves `#[cfg(any(linux, macos))]`, so Windows reaches none of it.

**Nobody wired it because until 0.21.0 Windows could not write at all.** DC-97 established the
consequence: G3 is *"No — unbuilt, not impossible"*, and the same missing wiring blocks **G8, all 18
`caller_tests`, half of G5, and part of `races.rs`** — one mechanism gating more than half of what remains
unproven on Windows.

## 1. The sequencing finding: the deletion is a prerequisite, not a nice-to-have

The owner's accepted strengthening order listed *"delete `promote`/`publish_immutable`"* as a separate,
cheaper item. Scoping this increment shows it must come **first**, and this RFC absorbs it as Stage 1.

**10 of the 24 failpoint wrappers exist solely for those two methods:**

- `promote` → `promotion_destination_sync`, `promotion_rename`, `promotion_source_sync` (3)
- `publish_immutable` → `wait_at_immutable_install`, `immutable_file_sync`, `immutable_install`,
  `immutable_install_error`, `immutable_install_sync`, `immutable_temp_unlink`,
  `immutable_cleanup_sync` (7)

Both methods have **zero production callers** — verified in the DC-97 cycle by tracing past the dispatch
wrappers, where a first-pass grep reported call sites that were only the implementors themselves.

**So doing the deletion second would mean wiring Windows crash injection into two methods and eleven code
paths that are then deleted.** Doing it first removes **42% of the surface** before any Windows work
begins.

## 2. The obstacle, stated as a problem

The remaining ~14 wrappers do not map one-to-one onto Windows. Several name operations Windows does not
perform:

- `created_directory_parent_sync`, `observed_directory_parent_sync`, `mutable_parent_sync` inject at
  directory-entry syncing — which `platform-support.md` already records as a **documented no-op** on
  Windows, because `FlushFileBuffers` has no contract for a directory handle. There is no operation to
  inject at.
- `required_open` is called from shared helpers that are currently unix-gated; whether the Windows walk
  has an equivalent boundary is a question, not an assumption.

**The increment is therefore a mapping exercise before it is a wiring exercise**, and DC-97 demonstrated
that doing the classification first — and reviewing it before code — is what catches the cases where an
answer is "there is nothing here to test" rather than "this is untested."

## 3. Acceptance criteria

1. **Every remaining failpoint wrapper has, for Windows, either a wired call site or a written reason
   there is no corresponding operation.** No blanks. Reasons name the absent operation, as DC-97's do.
2. **G3 is demonstrated on Windows** — at minimum one crash-injection test proving a failure at a sync
   boundary is propagated rather than swallowed, and **watched to fail with the injection removed.**
3. **The bar from DC-97 applies unchanged:** a control counts only when it has been observed to fail with
   the guarantee removed. Reasoning about a fault-injection path is not evidence about a fault-injection
   path.
4. **No Linux or macOS behaviour changes.** These are `#[cfg(test)]`-reachable injection points; if any
   production path changes shape to accommodate one, that is a finding to report, not an edit to make.
5. **`caller_tests` is re-evaluated against the new state**, and whatever remains gated says why in the
   gate's own comment — the standard DC-97 set.
6. **`platform-support.md`'s per-guarantee table is updated** for every row this moves.
7. Green three-platform CI, per the standing rule.

## 4. Non-goals

- **Changing any durability guarantee or implementation.** If wiring an injection point reveals that a
  Windows guarantee does not hold, **report and stop** — that is a finding of the same class as DC-96's,
  not something to fix inside a test increment.
- **Reaching parity with Linux's 20 call sites.** The right number is however many correspond to real
  Windows operations, and it will be smaller.
- **`prikk unlock` liveness and `FILE_ID_INFO`** — items 3 and 4 of the owner's order, still separate.
- **Building a Windows crash *simulator*.** This wires the existing injection mechanism; it does not
  attempt real process termination or power-loss emulation.

## 5. Staging

**Stage 1 — delete `promote` and `publish_immutable`**, their dispatch wrappers, their 10 failpoint
wrappers, and the `Point` variants that become unreachable. Independently worthwhile (two methods
documented "Weaker" on Windows that nothing calls), and a prerequisite per §1. Report the test-count delta
on all three platforms.

### Discharging the prior ruling that kept `publish_immutable`

**This is not a free deletion, and the record must show why it is now allowed.** `contract.rs`'s own doc
carries a standing ruling:

> *"**Ruled (design-v1.md §12.3): keep it.** Retiring a documented durability guarantee that has been
> through DC-71, DC-76, DC-81 and DC-82 is an RFC-level act, not a stage's side effect — revisit once
> Stages 4-5 (refs/trust containerization) show whether any loose-file use remains at all, not piecemeal."*

Both conditions are now met. **RFC 102 completed all six stages and shipped in 0.20.0**, so Stages 4-5
have answered the question the ruling deferred to: no loose-file use remains, and both methods have zero
production callers. And the ruling requires an **RFC-level act** — which this RFC is, rather than a stage
quietly dropping a guarantee on its way to something else.

`promote` carries no equivalent ruling; its contract entry is a plain description.

**Consequences that must be carried through, not discovered:**

- **G5 (race-safe no-clobber publication) retires as a guarantee.** DC-76's nine become eight, and
  `DurabilityContract` goes from eleven methods to nine. `platform-support.md`'s per-guarantee table and
  DC-97's classification both change.
- The tests that exercise it go with it — `object_store/tests/immutable.rs` and `races.rs`, which the
  contract doc itself names as the reason it was *"not fully dead."* **This resolves DC-97's deferred
  `races.rs` question by removing it**, rather than by splitting a helper.

**Stop-and-report:** if any production caller of either method is found, stop. The zero-caller finding is
the whole basis for this stage.

**Stage 2 — classify, then wire.** Map each remaining wrapper to its Windows operation or its reason,
**submit the classification for review before writing wiring**, then implement what the classification
supports. DC-97's report-first shape, which found two vacuous controls that reading the code did not.
