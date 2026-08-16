# DC-98 Windows Crash Injection — design v1

**RFC:** `rfcs/accepted/DC-98-WINDOWS-CRASH-INJECTION.md`. Read §0-§1 and §5's discharge note first.

Two stages, reviewed separately. **Do not start Stage 2 before Stage 1 is reviewed** — Stage 1 removes
42% of Stage 2's surface, and classifying a surface you are about to delete is wasted work.

## Stage 1 — retire `promote` and `publish_immutable`

### What comes out

Work outward from the contract, not inward from the call sites:

- `DurabilityContract::promote` and `::publish_immutable` (`fsutil/contract.rs`) — eleven methods to nine.
- Their implementations in `linux.rs`, `macos.rs`, `windows.rs`, and whatever `NoDurability` provides.
- The dispatch wrappers `promote_file_required` and `publish_immutable_file` (`fsutil/anchored.rs`).
- `fsutil/anchored/immutable.rs` — the shared helper, if nothing else uses it. **Check; do not assume.**
- **10 failpoint wrappers and their `Point` variants**: `promotion_destination_sync`, `promotion_rename`,
  `promotion_source_sync`, `wait_at_immutable_install`, `immutable_file_sync`, `immutable_install`,
  `immutable_install_error`, `immutable_install_sync`, `immutable_temp_unlink`, `immutable_cleanup_sync`.
  Also `set_immutable_install_barrier`, which exists only to feed `wait_at_immutable_install`.
- The tests that exercise them: `object_store/tests/immutable.rs`, `object_store/tests/races.rs`, and any
  `fsutil/tests.rs` cases naming either method.

### What this costs, deliberately

**G5 retires.** DC-76's nine guarantees become eight. Update, in the same commit so nothing is left
disagreeing:

- `docs/src/reference/platform-support.md` — the nine-guarantee Windows table **and** DC-97's
  per-guarantee negative-control table. G5's rows do not become "no"; they are removed, with a line
  recording that the guarantee was retired in DC-98 and why.
- Any prose that says "nine" — grep for it rather than trusting this list.

**This also resolves DC-97's deferred `races.rs` question by deletion.** I ruled then that splitting its
helper should wait for this increment; the answer turns out to be that the file goes away.

### Verify before deleting

1. **Zero production callers, re-established independently.** The DC-97 finding traced past the dispatch
   wrappers after a first-pass grep reported call sites that were only the implementors themselves.
   **Re-derive it; do not cite my number.** If you find a caller, stop and report — that finding ends the
   stage.
2. **Nothing else uses `immutable.rs`** or the `Point` variants being removed.
3. **The `Barrier` machinery**: `set_directory_create_barrier` must survive; only the immutable one goes.

### Report

Test-count delta on all three platforms, per rule 10's command, and the resulting method count on
`DurabilityContract`. A large negative delta is expected and correct here — that is dead surface leaving,
not coverage lost.

## Stage 2 — classify, then wire

**Report the classification before writing any wiring.** DC-97's report-first shape found two vacuous
controls that reading the code did not, and the same risk applies here in a sharper form: a failpoint
wired at the wrong boundary produces a test that passes for the wrong reason.

### The classification

For each remaining failpoint wrapper, one row:

1. **The wrapper**, and the Unix operation it currently injects at.
2. **The corresponding Windows operation**, or **no corresponding operation** with the reason.
3. If it maps: **the exact boundary** the call belongs at — before or after which syscall, and why that
   position is the one that makes a crash there observable.

**Expect some rows to be "no operation."** `created_directory_parent_sync`,
`observed_directory_parent_sync`, and `mutable_parent_sync` inject at directory-entry syncing, which
`platform-support.md` already records as a documented no-op on Windows because `FlushFileBuffers` has no
contract for a directory handle. There is nothing to inject at, and saying so is a passing answer.

`required_open` is called from `directory.rs`, `regular.rs`, and `read.rs` — all currently unix-gated.
Whether the Windows walk has an equivalent boundary is a question to answer, not to assume.

### Then wire, to the bar

- **At minimum, G3 demonstrated on Windows**: a crash injected at a sync boundary is propagated, not
  swallowed — and **watched to fail with the injection removed**. RFC criterion 3.
- **No production path changes shape** to accommodate an injection point. If one would have to, that is a
  finding to report. The Unix implementors manage this today; the same discipline applies.
- Re-evaluate `caller_tests`'s gate against the new state, with whatever remains gated saying why in the
  gate's own comment.

### Stop-and-report

- Wiring an injection point reveals a Windows guarantee that does not hold → **report and stop.** That is
  a DC-96-class finding, not a test-increment fix.
- A wired control cannot be made to fail with its guarantee removed → it is testing something else, the
  same trigger that fired on G1 in DC-97.

## Gates

The standing set per `EXECUTION-ORDER.md` §6 rule 9, both cross-target clippy runs, and a green
three-platform CI run before merge. Stage 1's deletion touches production code on every platform, so its
own CI run matters more than a docs stage's.
