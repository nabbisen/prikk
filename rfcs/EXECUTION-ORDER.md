# Prikk Execution Order

Single ordered view of all open work, for developers to follow in sequence.

This file does not create authority. `MILESTONES.md` remains the schedule authority, `ROADMAP.md` the
backlog narrative, `rfcs/IMPLEMENTATION-STATUS.md` the current-state snapshot, and each RFC its own scope
authority. This file answers only one question the others do not: **what do I pick up next, and what is it
waiting on?**

Last reconciled: after DC-50 closure (`4005efb`); DC-55 drafted from its replace decision and awaiting
design review.

## The two lanes

Development priority and release readiness are separate. **The release lane is `parked`** — no signer
bootstrap, hold, or release candidate exists, and `release-signers.toml` is empty and fail-closed.
Everything in §1 proceeds regardless. Nothing in §1 activates the release lane; activation requires the
three-authority commit described in `MILESTONES.md`, and neither implementation completion nor an
architect recommendation is authoritative for it.

## 1. Development lane — available now

Ordered by recommended sequence. The project owner may reorder by product value; the **Blocked by** column
is what actually constrains order.

Every handoff below is **already written**. Hand the developer the handoff, not the RFC — the RFC is scope
authority, the handoff is what they work from.

| # | Increment | State | Blocked by | **Handoff to give developers** |
|---|---|---|---|---|
| 1 | **DC-55** — first-party SHA-256 replacement | **Accepted 2026-07-28; ready for developers** | none — **cleared to start** | `handoffs/DC-55-first-party-sha256-replacement/implementation-handoff-v1.md` |
| 2 | **DC-42** — performance and maintainability gates | Proposed | design review; **DC-55** (see below) | `handoffs/DC-42-performance-maintainability-gates/implementation-handoff-v1.md` |
| 3 | **DC-52** — Python and oracle decommissioning | Proposed | design review; later-commit stability evidence | `handoffs/DC-52-python-oracle-decommissioning/implementation-handoff-v1.md` |
| 4 | **DC-43** — release security and distribution controls | Proposed | design review; security review | `handoffs/DC-43-release-security-controls/implementation-handoff-v1.md` |

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

**Why this order.** DC-55 leads because DC-50 authorized it and because of what it does to DC-42. DC-42
owns NFR-PERF-01, and DC-50 measured a ~5.8x throughput gap on the SHA-256 primitive underneath it — so
running DC-42 first would set performance requirements against a baseline DC-55 is already cleared to
invalidate, forcing a re-measure. DC-55 first means DC-42 measures once, against the primitive that will
actually ship. DC-42 then carries its real requirements decision (NFR-PERF-01 and NFR-PERF-02 must each
end implemented **or** explicitly amended). DC-52 needs its stability precondition. DC-43 needs security
review and is best consumed against a settled tooling gate.

**DC-55's design review was an author re-examination, and that is on record.** It is identity-bearing —
every ObjectId, state root, ref-name path, and signature preimage derives from the function it replaces —
and it is the category where a green test suite can mean "consistently changed" rather than "unchanged."
Design review v1 (`.git-exclude/reviewed/prikk-dc55-design-review-v1.md`) returned a blocking finding and
five notes, all resolved by the same author who wrote the design; the owner directed on 2026-07-28 that
revision proceed on that basis. The RFC's Status field records the gap rather than absorbing it into
routing convention, and the acceptance criteria were rewritten so the identity claim is reproducible by a
reviewer at **implementation** review, where independence is achievable. See also its RFC Risks section on
the `PRIKK_REGEN=1` regeneration hazard.

## 2. Blocked on a release-lane event

| Increment | Blocked by | Handoff (written, marked BLOCKED) |
|---|---|---|
| **DC-49** — portable-logic platform matrix | The M1 public portability-claim correction, which `MILESTONES.md` places inside the mandatory hold of an **activated** release. Cannot complete while the lane is parked. | `handoffs/DC-49-portable-logic-platform-matrix/implementation-handoff-v1.md` |

This is the one place where a development increment depends on a release-lane event. It was descoped from
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
    `prikk-object` 76, `prikk-replay` 4, `prikk-hash` 13, `prikk-crypto` 5, `prikk-release-policy` 57;
    180 locked packages.
11. **Submit a review request per candidate** with the diff, an evidence note, gate output, and an explicit
    statement of what did *not* change.

## 7. Posture

Production suitability, repository-format stabilization, and public-preview readiness all remain
**no-go**. The five blocking findings from the independent architecture review are closed *in
implementation* (DC-36 through DC-40) but not *in release* — they close for a shipped artifact only when
the §3 sequence completes and an adversarial release-candidate review accepts the combined state.
