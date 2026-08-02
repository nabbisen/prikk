# Prikk Execution Order

Single ordered view of all open work, for developers to follow in sequence.

This file does not create authority. `MILESTONES.md` remains the schedule authority, `ROADMAP.md` the
backlog narrative, `rfcs/IMPLEMENTATION-STATUS.md` the current-state snapshot, and each RFC its own scope
authority. This file answers only one question the others do not: **what do I pick up next, and what is it
waiting on?**

Last reconciled: 2026-07-31, after DC-56's scope finding (`8748f00`). **Complete:** DC-59, DC-60, DC-62,
DC-63. **Implemented, awaiting architect review:** DC-58's N1 (`6f53da3`), DC-61 (`ca4c044`).
**DC-56 closes partial** — its index works, but NFR-PERF-01's dominant violator was misidentified in its
RFC and is carried to **DC-64**, which **also closes partial** (implemented; eliminates the O(operations
replayed) cost but not the O(live node count) remainder — see §1 row 4). **DC-57 held** — handoff withdrawn. Its premise is no longer unreachable: the owner **decided 2026-08-02 that multi-commit queuing is a scheduled capability**, so DC-57 is blocked on that increment rather than on a decision. **There is now no outstanding owner decision** in the development lane. The release-lane decision
point sits after the performance work per the owner's 2026-07-29 direction.

## The two lanes

Development priority and release readiness are separate. **The release lane is `parked`** — no signer
bootstrap, hold, or release candidate exists, and `release-signers.toml` is empty and fail-closed.
Everything in §1 proceeds regardless. Nothing in §1 activates the release lane; activation requires the
three-authority commit described in `MILESTONES.md`, and neither implementation completion nor an
architect recommendation is authoritative for it.

## 1. Development lane — available now

Ordered by recommended sequence. The project owner may reorder by product value; the **Blocked by** column
is what actually constrains order.

Hand the developer the handoff, not the RFC — the RFC is scope authority, the handoff is what they work
from. **DC-57's handoff is withdrawn — do not issue it.**

| # | Increment | State | Blocked by | **Handoff to give developers** |
|---|---|---|---|---|
| 1 | **DC-58** — source-structure audit | **Complete.** N1 reframing `6f53da3`, reviewed and accepted 2026-07-31 | nothing | — |
| 2 | **DC-61** — branch closure (§6.5 deletion half) | **Complete.** Implemented `ca4c044`, reviewed 2026-07-31 (accepted with one non-blocking finding), **N1 repaired `2394f1b`** | nothing | — |
| 3 | **DC-56** — commit scan + memory compliance | Implemented `8748f00`. **Closes partial** — criteria 1,2,3,6,7 met; 4 and 5 re-scoped and carried. **NFR-PERF-01 not closed** | ruling applied 2026-07-31; nothing outstanding on the developers | — |
| 4 | **DC-64** — baseline reconstruction cost (NFR-PERF-01, carried) | Implemented; **closes partial** — the O(operations replayed) full-lineage-replay cost (97.6% of the phase) is eliminated on the warm path, but `load`/`persist`/`from_replay`, each a binding condition of the trust-ladder ruling, remain O(live node count), so Axis A is not fully flat. **NFR-PERF-01 remains missed**, on a lower curve. See the design document §9 | none | — |
| 5 | **DC-65** — text-edit baseline content | **Complete at `250ad54`** — reviewed and accepted 2026-07-31, verified independently at N=6 sealed edits, one non-blocking process note (reported gate commands did not match §6 rule 9; architect re-ran the canonical set, all pass). §1's four questions answered before design (`prerequisite-questions-v1.md`); invariant stated and both `plan_edit_text` and `patch_algebra::baseline_text` conform; a fifth site (DC-64's incremental step) found and fixed once the first three unblocked it; N=4/5 sealed-edit tests at store and CLI level; `ReplaceBinary` confirmed unaffected with a regression test; coverage finding recorded | none | — |
| 6 | **DC-66** — multi-commit queuing | **Complete at `45af36f`** — reviewed and accepted 2026-08-02, all eleven criteria met, one non-blocking note (rollback-draft still rejects on a non-empty WAL, deliberately). Architect independently rebuilt a four-deep queued edit chain from sealed history and got byte-correct content | nothing | — |

Each handoff for a *proposed* RFC states at its head that implementation may not begin until that RFC is
accepted. Preparing the handoff is not authorization; it removes everything except the design gate.

**DC-41 is complete** — all four stages committed (crash matrix `fb4153c`, hash vectors `d5bd096`, hash
differential `540d4db`, property/fuzz accepted 2026-07-28). Its descoped platform matrix is DC-49 and is
not a DC-41 completion condition.

**DC-54 is complete** — accepted, implemented at `e8f780a`, post-commit review accepted 2026-07-28. It
closed the encode/decode path asymmetry found by DC-41 stage 4's campaign.

**DC-51 is complete** — accepted `d7d49c6`, implemented `d3e939b`, post-commit review accepted with one
blocking finding, repaired `4c8b7a3`. Dependency placement is now mechanically enforced.

**DC-50 is closed** — closed at `4005efb` with a **replace** decision. Its record is at
`handoffs/DC-50-first-party-sha256-roi-decision/decision-record-v1.md`. It stays in `rfcs/accepted/`
rather than `done/` because `done/` means shipped and DC-50 ships nothing; being a decision-only
increment, it will never move. DC-50 produced no code and authorized exactly one successor: DC-55.

**DC-55 is complete** — accepted `a01e628`, swap implemented `8c84bc4`, fixture repairs `083d6c0` and
`753ebab`. Implementation review v1 returned one blocking finding (a fixture depending on directories git
cannot store, which broke `cargo test --workspace --locked` on the committed tree); repaired and accepted
at re-review v1 on 2026-07-29, verified by fresh clone with a negative control. `prikk-hash::sha256` now
runs on `sha2`, with the outgoing first-party implementation retained test-only as the differential's
permanent independent reference.

**DC-42 is superseded** — archived 2026-07-29 into **DC-56** (NFR-PERF-01), **DC-57** (NFR-PERF-02), and
**DC-58** (source-structure audit). Design review found it bundled three unrelated increments against
standing rule 2. Never implemented. See `rfcs/archive/DC-42-PERFORMANCE-MAINTAINABILITY-GATES.md`.

**Why this order.** Rows 1–3 are history now: DC-58, DC-61, and DC-56 are all landed, and the reconciliation
DC-61 needed with DC-63's kind branch was carried out as part of its 18-call-site schema threading.
**DC-64 is the only live development candidate**, and it is where NFR-PERF-01 — a missed product-M1 gate,
outstanding since before 0.17.7 — actually gets closed or is reported as inherent. **Accepted 2026-07-31
and cleared to start.** Design review measured its target to the operation (~40 µs each, 97.6% of the phase
in `replay_lineage`) and eliminated a keyed-cache route that could never have hit, given the one-record WAL
cap. Reporting the cost as inherent is a permitted outcome. DC-52 and DC-43 are **not** in this lane — both are release-blocked, see §2.

**DC-56's owner decision is settled.** Ruled 2026-07-30: NFR-PERF-01 bounds **steady-state** commit cost,
not every commit. That resolves its conflict with NFR-PERF-04 — which blesses rebuildable indexes while a
strict reading would forbid building one — and selects a changed-path index. The ruling carries a binding
obligation: **DC-56 must specify cache validity** (when the index is trusted, what invalidates it, what
bounds rebuild frequency), because an unbounded cold path satisfies the letter and defeats the requirement.
DC-56 is accepted and has no remaining blockers.

**DC-56 grew a second objective.** Design review v2 found the commit path does not merely *traverse* the
worktree — `worktree_files.rs:11-14` stores `bytes: Vec<u8>` per file, so every commit reads the whole
worktree into memory, O(total worktree bytes) regardless of change size. No requirement names that; it is
recorded in `MILESTONES.md` as an untracked scalability defect. The same changed-path index fixes both
objectives, but evidencing the memory one needed a memory axis, opened as **DC-62** rather than folded back
into DC-59 — DC-59 was complete and its criteria all discharged, so adding one would have retroactively
unfinished it. **DC-62 is now complete (`07b1fc8`), so this precondition is satisfied** — including the
floor and delta column DC-56's criterion 5 compares against, so no harness work remains in DC-56's scope.

**Beware the milestone labels.** `MILESTONES.md` § "Two milestone schemes" is required reading before
resolving any `M0`–`M3` gate label in the NFR matrix. The requirements use the product scheme; this file
and `MILESTONES.md` use the corrective one. The collision already caused one architect review to conclude
that overdue work was not yet due.

**On DC-55's review independence.** Its design review was an author re-examination: this project has one
architect, so independent design review is not achievable for a design the architect wrote. That is the
defined process — the organization document's Phase 2 gate assigns design review to the high-capability
model without distinguishing the two — rather than a deviation from it. The limitation is real regardless,
and DC-55 shows both sides of it: the author review found a genuine blocking defect, *and* a second
blocking defect survived into the implementation and was caught only because acceptance criteria had been
written to be reproducible from the repository rather than trusted from a report. Keep that pattern for
identity-bearing increments.

**DC-62 is complete** — accepted `5ff2388`, implemented `963caae`, **N1 repaired at `07b1fc8`** and that
repair reviewed and accepted with no findings. Peak-memory measurement added as a separate pass, no new
dependency, no production code. **DC-56's precondition is satisfied.** The measurement confirms O(total
worktree bytes): above the measured 6,144 KB floor, memory grows **9.92x** for a 10x repository-size increase
at fixed change count, where absolute VmHWM grows only 2.58x. The report now publishes an "Above floor"
column, so that is the figure a reader sees and the one DC-56's criterion 5 compares against.

**And N1 was repaired under DC-62, correctly, against my written instruction.** I had assigned the fix to
DC-56's scope on the DC-59 → DC-62 precedent above ("adding a criterion would have retroactively unfinished
it"). That precedent governs **new scope arising later**; it does not govern **a review finding against the
increment itself**, which is repaired under that increment — as DC-55's fixture blocker and DC-51's
`reference-check` break were. The developers did it under DC-62 and were right. **Rule:** the DC-59 → DC-62
split applies to new scope, not to findings; a finding against an increment reopens that increment.

**DC-63 is complete** — accepted `c7d1691`, held briefly on two `refs.rs` blockers, cleared via handoff v2,
implemented `6b33a72`. Implementation review v1 accepted with no blocking findings; gates re-run by the
architect in a detached worktree (790 tests, 0 failures). **Requirements §6.6 is closed**, and `RefKind::Tag`
has production call sites for the first time since the ref model was built. Both blockers are fixed in the ref
system's core: kind-aware ref-name validation in `validate_coherent_publication`, and one shared
`ensure_ref_target_valid` helper resolving the tag indirection at both verification sites.

**DC-60 is complete** — accepted `994bf32`, scope amended the same day to `list` + `create`, implemented
`6c2b7a6`. Implementation review v1 accepted with no blocking findings; gates re-run by the architect in a
detached worktree (779 tests, 0 failures). Its `branch create` publishes a byte-equivalent DC-13 genesis
shape, asserted by test. Deletion is DC-61.

**DC-58 batches 1 and 2 are accepted** — `e1d0213` and `54a3037`, implementation reviews accepted with no
blocking findings. All four remaining over-500 files were resolved: three split, and `lifecycle_cache.rs`
reduced from 974 to 117 implementation lines by moving 848 lines of already-test-only trust-ladder
scaffolding into a whole-module-gated `cache_ladder.rs`. That reclassification was ruled in scope — a
non-test and a release build both still compile, proving nothing production-reachable moved behind the
gate — but must be reported separately from the three genuine splits, which is the only item outstanding.
Two permanent by-design exceptions stand: `node_authoring.rs` deferred while DC-56 is open, and
`frozen_outgoing.rs` excluded as DC-55's immutable reference.

**DC-59 is complete** — implemented `a9c2fe0`, accepted 2026-07-29. Its report measured the full-tree
scan: 4.22 ms at 10 files rising to 516 ms at 10,000, with the change set fixed at one file throughout.
The scan is now evidence rather than inference, and DC-56's precondition is satisfied.

**DC-57 is HELD** — its premise does not hold and its handoff is **withdrawn**. The active WAL is
structurally capped at one record repository-wide, so 800/1000 is unreachable and its boundary tests
cannot be constructed. NFR-PERF-02 presupposes multi-commit queued active sessions, which
`rfcs/IMPLEMENTATION-STATUS.md:464` records as not implemented. Found by the dev team stopping at handoff
Step 1 as instructed. **Blocked on an owner decision** — see `MILESTONES.md` finding
"Multi-patch active blocks not implemented".

## 2. Blocked on a release-lane event

| Increment | Blocked by | Handoff (written, marked BLOCKED) |
|---|---|---|
| **DC-49** — portable-logic platform matrix | The M1 public portability-claim correction, which `MILESTONES.md` places inside the mandatory hold of an **activated** release. Cannot complete while the lane is parked. | `handoffs/DC-49-portable-logic-platform-matrix/implementation-handoff-v1.md` |
| **DC-52** — Python and oracle decommissioning | `DC-45:419` and `:545` forbid Python deletion "before the first Rust-gated 0.19.0 release and its accepted post-release stability rerun." No release has shipped since 0.17.7. **Moved here 2026-07-30** — it was previously listed as available now, which was wrong. | `handoffs/DC-52-python-oracle-decommissioning/implementation-handoff-v1.md` |
| **DC-43** — release security and distribution controls | Its scope *is* release security and distribution, and `DC-35:255-257` hands it key custody, rotation, expiry/revocation monitoring, attestations, and SBOMs. DC-35 needs a fitness amendment, so designing DC-43 now designs against a foundation about to change. **Moved here 2026-07-30.** | `handoffs/DC-43-release-security-controls/implementation-handoff-v1.md` |

Release stabilization is deferred by owner direction 2026-07-30, so everything in this section is
dormant. Note that **three** increments sit here, not one — DC-52 and DC-43 were previously listed in §1 as
available now, which understated how much of the backlog is release-gated.

This was the one place where a development increment depends on a release-lane event. It was descoped from
DC-41 for exactly that reason. If the owner would rather unblock it sooner, the alternative is a reviewed
decision to move the documentation correction into the development lane — that is an owner decision, not
an implementation one.

## 3. Release lane — only on explicit owner activation

Not startable by a developer. Recorded so the sequence is visible.

1. Activation commit — lane `active` plus exact target version, in all three authorities, atomically.
2. DC-35 signer bootstrap as an isolated public governance transaction.
3. Mandatory public 72-hour hold.
4. During the hold: literal DC-38 stale-pointer/ahead-log reproduction; DC-37-aligned portability/
   requirements correction (this is what unblocks DC-49).
5. Explicit architect/security hold-lift ruling.
6. Combined release candidate: full gates, corrective failpoint matrix, adversarial RC review.

**Gate inheritance:** release conditions attach to accepted-but-unshipped *increments*, not to version
labels. DC-39, DC-40, and DC-41 are on `main` and unshipped, so whichever release ships first inherits the
complete M1 sequence regardless of what it is numbered.

## 4. Scheduled later

These two have **design briefs**, not implementation handoffs — their detailed design does not exist yet,
and their own RFCs defer it to design review. The brief specifies what the design stage must produce, so
design starts from a defined target. An implementation handoff follows once each design is accepted.

| Increment | Milestone | Design brief | Note |
|---|---|---|---|
| **DC-44** — migration, backup, restore evidence | M3 | `handoffs/DC-44-migration-backup-restore-evidence/design-brief-v1.md` | Owns NFR-REL-03; decides what happens to existing format-1 repositories |
| **DC-53** — repository-wide AUTHOR trust verification | Post-M2, unscheduled | `handoffs/DC-53-repository-wide-author-trust-verification/design-brief-v1.md` | Capability gap, not an evidence gap; identity-adjacent, needs a companion design document with vectors |

## 5. Unscheduled, deliberately

- **Key lifecycle** — rotation, revocation, expiration, thresholds above one, hardware signing, remote
  trust distribution. Explicitly out of scope for every current RFC. Needs its own increment before any
  publication-grade trust claim.
- **Cosmetic marker diagnostic** — unknown/malformed `.prikk/FORMAT` reports `unsupported format version:
  0`, where `0` is a sentinel rather than the offending value. Fails closed correctly. A non-blocking
  pre-RC correction candidate; not a prerequisite unless selected.

## 6. Standing rules for every increment

These apply to all work above and are not restated in each handoff.

1. **Design-first.** A proposed RFC is not implementation authority. It must move to `rfcs/accepted/`
   through its own design review first. Requirements → external design → internal design → program design
   are the architect's; implementation and testing are the developers'.
2. **One increment per candidate.** No bundling. Multi-stage increments land one stage per review.
3. **A finding is never a test expectation.** Any behaviour defect discovered opens its own corrective RFC
   with a minimized reproducer. This matters most in DC-41 stage 4, where randomized decoder input is
   where something will plausibly be found — a malformed-input panic is an NFR-SEC-04 defect, and finding
   one is a success for the campaign, not a failure of the stage.
4. **Frozen identities are verified every review.** Current baselines: `Cargo.lock`
   `601d0678b8481a750519e64bb19f66f8532301b4157d8353d8d9211261c5da31` (re-frozen at DC-41 **stage 4**,
   which added `proptest`; this supersedes stage 3's `18a8b40a…`, which itself superseded `0cd51cbd…`),
   oracle manifest `2f0c54ab…`, `release-signers.toml` `f8d56841…`, both command inventories. Any
   intentional change is a reviewed re-freeze whose new hash supersedes the old.
5. **These are review-gated policy/identity artifacts, not refactorable code.** Changing any of them is a
   policy change requiring its own review: `command_scan/procedure.rs` (accepted command productions),
   `command_scan/prefix.rs` (prefix grammar), `reference.rs` (authority descriptors), `format.rs`
   (format-2 schema allowlist), `state_root.rs` (state-root byte grammar).
6. **Never spell out the full command form in prose.** Write "release-policy `check`", not the
   full `cargo run --locked -p prikk-release-policy` invocation with the bare subcommand spelled out
   after `--` — that full form is a recognised policy invocation, so any scanned `.md` file containing
   it must be registered in the command inventory or `reference-check` fails. DC-51's own evidence note
   tripped this. `boundary-check` and `reference-check` are safe; only the bare subcommand word
   triggers it.
7. **Dependency placement is now mechanically enforced.** DC-51's `boundary-check` category
   `dependency-placement` catches a third-party crate misplaced into a product crate's
   `[dependencies]`, including under `[target.*]` and via `package =` renaming. Review-only
   verification is defense-in-depth going forward, not the primary control.
8. **Governed procedure files.** `.github/workflows/ci.yml` and any `.sh`/`.yml` under `.github`,
   `scripts`, or `release` are scanned default-closed. Every `run:` command must match an accepted
   production, or `boundary-check`/`reference-check` fail. Adding a CI command means a reviewed classifier
   amendment in the same increment.
9. **Gate set for every candidate.** `cargo fmt --all -- --check`;
   `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
   `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`; `git diff --check`;
   `cargo audit --no-fetch`; release-policy `check`, `boundary-check`, `reference-check`. Use a
   repository-local `TMPDIR` (`.git-exclude/tmp`) where `/tmp` is read-only.
10. **Report counts before and after.** Test counts per touched crate, and locked package count where
    dependencies change, so no silent loss or growth can hide. Current: `prikk-store` 543,
    `prikk-object` 76, `prikk-replay` 44, `prikk-hash` 14, `prikk-crypto` 5, `prikk-release-policy` 59;
    180 locked packages. (`prikk-replay` was previously misrecorded here as 4; it has been 44 since
    before DC-54 and nothing has touched it — corrected during DC-55's baseline check.)
11. **Submit a review request per candidate** with the diff, an evidence note, gate output, and an explicit
    statement of what did *not* change.

## 7. Posture

Production suitability, repository-format stabilization, and public-preview readiness all remain
**no-go**. The five blocking findings from the independent architecture review are closed *in
implementation* (DC-36 through DC-40) but not *in release* — they close for a shipped artifact only when
the §3 sequence completes and an adversarial release-candidate review accepts the combined state.
