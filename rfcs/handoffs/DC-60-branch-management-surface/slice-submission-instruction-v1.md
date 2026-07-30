# DC-60 Slice Submission Instruction

**Date:** 2026-07-30
**For:** the DC-60 implementer
**Supplements:** `implementation-handoff-v1.md` (Step 3 void) and
`.git-exclude/reviewed/prikk-dc60-delete-divergence-ruling-v1.md` (the ruling, read that first)

Your report was accepted in full and both findings confirmed. DC-60's scope is amended to **`branch list` +
`branch create`**. Deletion and `--continue-log` moved to DC-61, which needs a ref-log tombstone — a format
change DC-60 excluded.

This document says exactly what to do with the code you have already written.

## 1. Strip the deletion machinery from the DC-60 commit

Your working tree contains a complete, correct-to-spec `branch delete` and `--continue-log`. **Neither may
ship in DC-60**, because DC-60's scope no longer contains deletion. Left in, they are unreferenced
production code that will likely trip `dead_code` under `-D warnings`, and they are scope creep into an RFC
that now explicitly excludes them.

Remove, by location as of your current tree:

| File | What |
|---|---|
| `crates/prikk-cli/src/branch.rs:153` | `run_delete` |
| `crates/prikk-cli/src/branch.rs:84-88` | the `continue_log` match arm and its "pass `--continue-log`" message |
| `crates/prikk-cli/src/branch.rs:264,270` | the `continue_log` field and its parse default |
| `crates/prikk-cli/src/main.rs` | the `delete` dispatch arm |
| `crates/prikk-cli/src/output/help.rs` | help text for `delete` and `--continue-log` |
| `crates/prikk-store/src/refs.rs:211` | `RefStore::delete_pointer` |
| `crates/prikk-store/src/refs/pointer.rs:38` | `remove_ref_pointer` |
| `crates/prikk-store/src/lib.rs` | the corresponding exports |
| `crates/prikk-cli/tests/dc60_branch_management.rs` | the delete tests, the `--continue-log` test, **and the deliberately-failing test** |

**Also revert one visibility widening.** You changed
`crates/prikk-store/src/active.rs:180` `require_active_ref_for_non_empty_wal` from `pub(crate)` to `pub`,
and widened its `lib.rs:70` export, for the delete guard. Nothing else in the tree uses it from outside
`prikk-store` — I checked. With delete stripped, **restore `pub(crate)` and drop the export.** Otherwise
DC-60 ships a public API widening for a feature it does not contain, which is exactly the kind of quiet
surface growth DC-51's placement gate and this project's review standard exist to catch.

Reverting it is not a criticism of the original change — widening was right *for* the delete guard, and you
reused the helper rather than restating the rule, which is what the handoff asked for.

## 2. Keep the stripped work — DC-61 will reuse most of it

Preserve it however suits you: a patch file, a local branch, a stash. Do not discard it.

`delete_pointer` and `remove_ref_pointer` are the right primitives and DC-61 keeps them. What DC-61 adds on
top is genuinely new and not yet designed: the tombstone append, the `verify` classification arm, and the
`publish` fifth arm. Your fail-closed guards (missing branch, active-WAL owner, last remaining branch) carry
over essentially unchanged.

**Do not start DC-61.** It is `proposed`, not accepted, and its §4 carries three verification obligations —
ref-log format compatibility, whether `replay_log`/`log_position` can represent a tombstone tip, and what
`doctor` does with one — any of which could change the design. Implementation authority follows its design
review, not this note.

## 3. Submit the slice

One commit containing `branch list` and `branch create` only, then a review request against **amended**
DC-60.

Read `rfcs/accepted/DC-60-BRANCH-MANAGEMENT-SURFACE.md` before submitting — its acceptance criteria went
from nine to six, and you built against the nine. The six you will be measured against:

1. `branch` lists all branches with deterministic ordering, verified against a fixture with more than one
   branch.
2. `branch create` publishes a signed ref-state object; the new branch appears in listing; `verify` passes
   afterward.
3. `branch create` fails closed on an invalid name, an existing name, an unresolvable `--from`, **and a name
   with a surviving log and no live pointer** — each tested against constructed state.
4. No identity artifact changes: `vectors/snapshot.txt`, `vectors/hard.rs`,
   `state_root/tests/vectors.rs`, `text_span/vectors.rs` all byte-identical.
5. Command help states that switching is not supported and names the reason.
6. Full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9, plus test counts before and after per rule 10.

**Criterion 3's fourth condition is the one to check you still satisfy.** DC-60 no longer deletes anything,
but a ref log can survive an interrupted publication, and creating over it would produce the corrupt state
DC-61 exists to resolve. The guard must stay, with no `--continue-log` escape. `publish` would refuse
anyway, but a clear early error beats a generic classification failure.

**Criterion 5 changed slightly:** help should now also say deletion is not yet available, not only that
switching is unsupported.

## 4. Include in the review request

- The diff.
- Test results for each fail-closed condition, constructed as real state rather than asserted in isolation.
- **The DC-13-genesis-shape equivalence assertion.** Your
  `branch_create_at_existing_target_matches_dc13_genesis_shape` — asserting `update_seq`,
  `previous_ref_state_id`, `kind`, and that no new block was created — is stronger evidence than the RFC
  asked for and is the property I was most concerned about. Keep it and cite it.
- Confirmation that the four identity artifacts are byte-identical.
- Test counts per touched crate before and after.
- An explicit statement of what did **not** change — including that `require_active_ref_for_non_empty_wal`
  is back to `pub(crate)` and no public API widened.
- Full gate set: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features
  --locked -- -D warnings`; `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`;
  `git diff --check`; `cargo audit --no-fetch`; release-policy `check`, `boundary-check`, `reference-check`.
  Use a repository-local `TMPDIR` (`.git-exclude/tmp`).

Run the gates on a **clean checkout of the commit**, not the working tree, and say so. DC-55's
implementation review found a mandatory gate that passed locally and failed on a fresh clone; that is the
standing expectation now.

## 5. Standing request

Both blocking findings against DC-60's design came from checking `rfcs/done/` and `specs/` rather than from
reading the RFC. If something in the amended scope contradicts a shipped RFC or an accepted requirement,
stop and report it — as you did here, and as DC-57's report did. Twice now that has caught a defect no
amount of implementation care would have.
