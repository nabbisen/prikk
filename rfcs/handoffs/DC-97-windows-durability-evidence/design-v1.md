# DC-97 Windows Durability Evidence — design v1

**RFC:** `rfcs/accepted/DC-97-WINDOWS-DURABILITY-EVIDENCE.md`. Read §0 first; this does not restate it.

**This stage produces a report, not a patch.** §5 below is the deliverable. Do not write test code until
the classification is reviewed.

## 1. Where each guarantee's evidence lives today

`conformance.rs`'s own module doc carries the coverage map. Reproduced with the fact that matters added —
**which side of the `fsutil.rs:14-17` gate each one sits on**:

| Guarantee | Current evidence | Gated off Windows? |
|---|---|---|
| G1 root-anchored, no-follow | `tests::directory::required_directory_rejects_symlink_component` | **yes** |
| G2 atomic content replacement | `tests::mutable_atomic_write_replaces_complete_content` | **yes** |
| G3 durable-after-return | the DC-41 failpoint suite in `fsutil::tests`/`caller_tests` | **yes** |
| G4 exclusive creation | `conformance::create_exclusive_refuses_an_already_occupied_path` | **yes** |
| G5 race-safe no-clobber publication | `object_store::tests::immutable::*` | **no — outside the gate** |
| G6 regular-file validation | `tests::append_and_truncate_reject_fifo_without_blocking` | **yes** |
| G7 non-blocking opens | same as G6 | **yes** |
| G8 concurrent-safe directory creation | `tests::directory::concurrent_required_directory_creation_is_idempotent` | **yes** |
| G9 mode-bit isolation | `conformance::set_permission_bits_masks_file_type_bits_out_of_a_recorded_mode` | **yes** |

**Eight of nine sit inside the gate.** G5 is the only one whose evidence lives elsewhere — **confirm
whether it actually executes on Windows** rather than assuming it does because it is ungated; I saw one
`immutable` line in the Windows log and did not establish whether it was a result or a `Running` header.

## 2. What is already known to be undemonstrable, so you are not rediscovering it

- **G6/G7** rest on a FIFO created by `mkfifo(3)` (`test_support.rs:194-203`). Windows has no equivalent
  object. Expect a reported reason — but state *which* property goes untested, not merely that FIFOs are
  absent. G6 is "a non-regular file is rejected"; on Windows the analogous hostile input is not a FIFO, so
  say whether one exists (a named pipe? a device path?) and whether it is reachable through the same code
  path. **If a Windows analogue does exist, that is a demonstrable control, not a reason.**
- **G9** is a documented no-op on Windows (NTFS has no POSIX execute bit) *and* conformance.rs already
  records it as "not independently negative-controllable on Linux". Two distinct reasons; give both.

## 3. The standard this increment is held to, in its own words

conformance.rs's G3 row records the exact trap:

> *"An earlier draft added a test that opened a second `MutationRoot` and re-read the file to stand in for
> 'surviving a restart' — **it passed even with the `fsync` call deleted**, because without a real crash
> nothing forces the write out of the page cache. Removed once that was discovered by trying the negative
> control, not asserted."*

**That is the bar.** A Windows control counts only if you have watched it fail with the guarantee removed.
Criterion 5's "the Windows test count rises" exists so a suite that compiles without asserting anything is
visible; this is the qualitative half of the same check.

**Report the count honestly**, per rule 10's measurement command, and note that the Windows figure is
measured on the Windows runner and cannot be reproduced locally.

## 4. The gate itself

`fsutil.rs:14-17` gates `caller_tests` and `tests` together. Treat them separately: they are different
bodies of work that happen to share one attribute.

**Do not remove the gate to see what breaks.** Work out per module — and where necessary per test — what
is unix-only by nature and what is unix-only by accident. Concrete obstacles already visible in
`conformance.rs`: `:27` imports `std::os::unix::fs::PermissionsExt` unconditionally; `:30-33` import
`LinuxDurability`/`MacosDurability` with no Windows arm; each assertion helper has per-platform `#[test]`
wrappers with no Windows arm. Adding a Windows wrapper naming `WindowsDurability` is the shape the file
was designed for — its own doc says the `assert_*` bodies are the shared part precisely so a new platform
plugs in there.

**Whatever stays gated afterwards gets a comment saying why.** A gate without a stated reason is how this
one lasted through two platform increments.

## 5. Deliverable — the classification report

For each of the nine, in a table:

1. **The guarantee**, and the property its control actually removes to make the test fail.
2. **Demonstrable on Windows: yes / no.**
3. **If yes:** what the Windows control asserts, and confirmation you watched it fail with the guarantee
   removed.
4. **If no:** the specific absent primitive or property. Not "not applicable."
5. **Whether the Linux/macOS control is touched at all.** Criterion 3 says it must not be weakened; say
   explicitly where you split a shared path.

Plus: the G5 question from §1, and a separate line for `caller_tests` — is its matrix unix-specific by
nature, and which parts?

**Stop-and-report triggers**, any of which ends the stage early with a finding:

- A guarantee the table claims for Windows turns out **not to hold** when a control is finally written for
  it. RFC §4 is explicit: report and stop, do not fix here.
- A control that cannot be made to fail on Windows even with the guarantee removed — that means it is
  testing something other than what it claims, on that platform.
- Enabling a module on Windows changes a Linux or macOS result. That is a finding about the tests, not a
  merge conflict to resolve.

## 6. Gates

The standing set per `EXECUTION-ORDER.md` §6 rule 9, both cross-target clippy runs, and a green
three-platform CI run before merge. `--no-fail-fast` on the mutation jobs is permanent; with this
increment it starts to matter much more, because newly-enabled Windows tests are exactly the kind that
fail in groups.
