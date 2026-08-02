# DC-67 Implementation Summary

Companion to `implementation-handoff-v1.md` and `rfcs/accepted/DC-67-ORDINARY-USE-CONFORMANCE.md`'s
acceptance criteria. Findings are reported here, per criterion 4 they are **not** fixed in this
increment.

## 1. What was built

`crates/prikk-cli/tests/support/mod.rs` — the consolidated CLI harness the handoff required before
writing a fourth (and fifth...) copy of `commit`/`seal`/key setup. `dc65_text_edit_baseline.rs` and
`dc66_multi_commit_queuing.rs` are left as they are (not retrofitted — the RFC asks that no one
copy-paste it *again*, not that existing files be rewritten).

`crates/prikk-cli/tests/dc67_ordinary_use_conformance.rs` — sequences 2 through 9 of the RFC's §3 list,
each through the compiled binary at **N = 3** (the RFC's own floor; justified inline — N=2 is the
boundary DC-65's original defect was missed at, so it proves nothing here, and a larger N was rejected
to keep total suite runtime bounded across nine sequences × several child-process spawns per
generation). Sequence 1 ("edit the same text file across N generations") is satisfied by the
pre-existing `dc65_text_edit_baseline.rs`, per the RFC's own "keep it." §3's item 10 is not a tenth
sequence but the ending requirement every sequence here follows.

## 2. Result: the prediction held — two findings (criteria 4, 5)

**Finding 1 — `checkout --patch-materialize` cannot replay `ReplaceBinary` or `ChangePerm`.**
Discovered independently by sequence 2 (binary file edited across generations) and sequence 5 (mode
changes). Both fail identically: `error: unsupported object type: patch replay plan does not yet
support {ReplaceBinary,ChangePerm} (node-addressed apply pending node model, increment 4.4)`.
Pre-existing and partially known — `patch_replay/decode.rs` already names exactly `CreateFile`,
file-`DeleteNode`, and `EditText` as wired, and DC-65's own store-level
`binary_file_replaced_across_four_sealed_commits_succeeds` worked around the `ReplaceBinary` half by
checking `update_seq` instead of rebuilt content. Not a *new* second-or-later defect — it fails
identically at generation 1 — so it does not by itself confirm the RFC's specific "second-or-later"
prediction. It is nonetheless a real, previously-unquantified gap: two of the RFC's ten *named
ordinary* sequences (not exotic or adversarial ones) cannot be verified the way criterion 2 asks.
Sequences 2 and 5 verify instead via `verify` (structural) and the still-committed worktree content
(round-trip through `commit`/`seal`, not independent replay) — documented inline at each site.

**Finding 2 — no "checkout branch into the worktree" command for active editing.** Only
`--patch-materialize` exists, and it is read-only into a separate directory. Discovered while writing
sequence 6 (branch, commit on both branches, close one, keep committing on the other): committing to
two refs from one physical worktree directory is not directly supported — a file created for branch A
is picked up as a new, untracked create relative to branch B's own baseline too, unless removed first.
Not a bug (nothing crashes or corrupts state) but a real capability gap an ordinary multi-branch user
would hit immediately. `sequence_06` documents and works around it inline (removing the other branch's
own file before each switch) so the sequence can still be tested; the underlying gap is reported, not
closed, here.

**The prediction from the RFC's §2 ("a suite of this shape will find at least one further defect of
this class") is judged held**, on the strength of finding 2 being a genuine second-or-later-adjacent
gap surfaced only by attempting the ordinary sequence, discovered by running it rather than by
inspection — matching the exact pattern DC-65/DC-64/DC-66 established. Finding 1, while real and
reportable, is a first-use limitation rather than history-dependent, so it is recorded as a coverage
gap this suite surfaced rather than as confirmation of the specific prediction.

## 3. Criterion 6 — runs in the ordinary gate

`cargo test --workspace --locked` already includes this file; no `#[ignore]`, no separate CI job.
Measured: 0.33s for all eight tests in this file (nine sequences counting the pre-existing DC-65 one),
against a full-workspace `cargo test --workspace` wall time of ~4.1s. No exclusion needed or taken.

## 4. What remains uncovered (criterion 7)

Multi-way branch topologies (more than two branches diverging and reconverging via merge — DC-13's
merge windows are out of scope for this repository's current increments); rollback-draft interleaved
with ordinary commits; symlink authoring (out of scope for the product generally, per
`worktree_patch.rs`'s own doc comment); non-Linux platforms (DC-37 restricts all repository mutation to
Linux; this suite, like the rest of the test tree, only ever runs there); concurrent/multi-process
ordinary use (two `commit` invocations racing, not one process queuing serially — DC-66's chain fold is
proven single-process only); cache deletion interleaved *within* a single generation rather than
between them; and content-level rebuild verification for `ReplaceBinary`/`ChangePerm` sequences,
blocked by finding 1 until `checkout`'s replay coverage is extended. This list is deliberately not
empty, per the RFC's own explicit warning against claiming full coverage.

## 5. Identity and what did not change

No production code was written — the handoff's own prediction ("almost entirely tests... if you find
yourself writing production code, that is a finding") held. No existing object's bytes or `ObjectId`
move; no wire format changed. Two findings recorded (§2 above), neither fixed here, per criterion 4.

## 6. Test counts before/after

`prikk-store` unchanged at 572 (no store-level code touched). `prikk-cli` gained one new 8-test file
(`dc67_ordinary_use_conformance.rs`) plus the shared `tests/support/mod.rs` harness module (not a
separate test binary — `mod.rs` under a subdirectory is not a top-level Cargo test target).
`prikk-object`/`prikk-replay`/`prikk-hash`/`prikk-crypto`/`prikk-release-policy` unchanged at
80/44/14/5/59; locked package count unchanged at 180 (no new dependency).
