# Prikk Roadmap

This repository follows the design-first Prikk roadmap. Change history is tracked in `CHANGELOG.md`;
the corrective release sequence and criterion state are in `MILESTONES.md`; current-state architecture
and concept detail lives in the published `docs/src/reference/` and `docs/src/guide/` book pages.
`rfcs/IMPLEMENTATION-STATUS.md` is retired (see its own banner). Review findings live in the review
result that raised them; anything that must outlive a review is documented in `docs/` or `rfcs/`.

Development priority and release readiness are separate planning lanes. The project owner selects the
active development theme by product value. Release readiness remains dormant until the project owner
explicitly activates preparation for a named version; implementation completion, a listed release gate,
or the word "next" does not activate signer, hold, RC, tag, or publication work.

Activation requires a reviewed tracked commit that atomically records `active` plus the exact version in
this file, `MILESTONES.md`, and `rfcs/IMPLEMENTATION-STATUS.md`. Until that commit lands, discussion,
review recommendations, roadmap targets, and untracked messages leave the lane parked and cannot trigger
a fingerprint request. Before bootstrap begins, parking or retargeting uses the same reviewed three-file
transition; after bootstrap begins, DC-35 governance and hold rules control closure. A later target cannot
bypass an unpublished increment: the first release containing any still-unshipped accepted RFC inherits
all of that RFC's release conditions and lifecycle/status corrections. If the three authorities disagree,
the release lane is parked; see `MILESTONES.md` under Baseline and release posture.

## Future Themes

Recorded 2026-08-04 so they are findable rather than conversational. **None is scheduled**; each names its
own prerequisite. Ordering against the accepted roadmap items (node-model apply → merge execution → M4
attestation slice) is not implied.

### Sync — shipped 2026-08-22 (RFC 115/116/117, `c9c8576`); residuals remain

Recorded 2026-08-09 as M5 bundling "Sync and Quarantine." **They were separable, and sync alone was
at least three distinct questions** — bundling them under one label would have repeated the
"increment 4.4" error, where one marker covered two unrelated blockers and produced a wrong roadmap
framing. **All three are now resolved**, in the three bullets below. Quarantine, the label's other
half, was dissolved this session: nothing enters the store un-adopted, so there is no halfway state
left to quarantine.

- **Sync — criterion 1 of the status-claim criteria** (`MILESTONES.md`). **MET 2026-08-22 (`c9c8576`).**
  Recorded 2026-08-09: nothing in the tree exchanged history between repositories, so a *distributed*
  VCS could not distribute — at the time, correctly **the largest single gap between prikk and dropping
  the "early implementation" badge**. **Delivered across RFC 115/116/117**, ten increments: `prikk sync`
  negotiates via artifacts (`summary`/`compare`/`have`/`build`/`accept`/`pending`/`seal`), patch-level
  history moves in the `PEXCH002` exchange artifact, and tags travel and are adopted under the
  receiver's own key (`sync tags`/`adopt-tag`). Criterion 1's row carries the stated limits — prikk does
  not move the bytes itself, "two machines" is exercised as two repositories with no cross-host test,
  and there is no discovery or remote-tracking — read it before citing this row further.
- **Multi-parent block lineage — shipped, DC-75, 0.19.0.** `merge_execute.rs:168-171` stores both
  parents in `BlockPayload.parent_block_ids`; `:176` sets `mainline_parent_id` (`BlockPayload:63`) to
  designate which one state derivation and replay follow. **The premise this bullet used to give as
  its open question — "the patch DAG already records a merge structurally" — was refuted by DC-75
  itself**, which exists precisely because that was false: `parent_patch_ids` (a `PatchPayload` field,
  distinct from `BlockPayload.parent_block_ids`) was `Vec::new()` at every construction site and read
  nowhere — there was no patch DAG, which is why block-level parentage had to be added.
  **`patch_replay.rs`'s own "fails closed on multi-parent" doc comment was also imprecise** — it walks
  a well-formed `Merge` block fine, via its mainline parent; it refuses only a malformed one.
  **`parent_patch_ids`'s own fate is no longer open**: it was removed outright, not populated or
  repurposed, at Patch schema 2 (`0.24.0`). Product **M3**, named "Block DAG and Checkout", is
  discharged by this shipping, not still describing unbuilt scope.
- **Transport — settled by RFC 116's accepted ruling, not open.** `prikk-store` stays bytes-in/bytes-out
  and prikk stays off the network; sync-over-any-channel satisfies criterion 1, so moving a `sync`/
  `bundle` artifact between repositories is the operator's own channel, by design, not a gap awaiting a
  future increment. What remains genuinely unautomated — the operator still copies the file themselves —
  is a consequence of that ruling, not an open question about who owns it.

**Verified before shipping, and still true**: zero networking crates in `Cargo.lock`, no networked
verb in the CLI. Sync was the first capability that could have given prikk an attack surface it did
not have before — the owner's 2026-08-04 direction ("security is strongly prioritized to function;
secure by default; we should not be in a hurry") is why a threat model preceded it
(`docs/src/reference/trust-threat-model.md` covers sync substantively: Core Caveats, the
trust-gated operation list, and the tag arrival/adoption rules). The verification above is the
evidence behind that page's anonymity ruling, and for why sync raises no installer or transport
question of its own.

**Constraint, not a pending decision.** Sync shipped without an async runtime. The DC-51 boundary
that would have applied if it had needed one still stands (`placement.rs:14` permits only
`getrandom` and `rustix` for `prikk-store`) — a real limit on any future runtime addition, not
unfinished sync design.

### Merge execution — shipped in 0.19.0 (DC-74/DC-75); residuals remain

Owner-ruled 2026-08-04: **B then C** — merge execution, then the M4 attestation slice. **DC-74 shipped
`prikk merge` in 0.19.0** (`CHANGELOG.md`: *"`prikk merge` executes a merge."*), and DC-75 gave it
structural merge-block lineage the same release — a merge seals as `BlockKind::Merge` naming both
parents, a mainline pointer, and a baseline confluence proof that `prikk verify` re-derives rather than
trusts. DC-16's conservative subset and its soundness oracle were the foundation; execution is now built
on it, not the unbuilt half.

**Real residuals, not closed by 0.19.0**: merge-base discovery is manual (`--baseline-block` is
explicit; nothing computes it), rename detection does not exist (no `ConflictWitnessKind` variant names
one), and semantic merge remains out of scope (a stated non-goal since DC-16).

### Editor, IDE, and file-manager integration — blocked on model gaps, not on API work

Deferred, and the reasons are the point: **no current-branch pointer** (an IDE status bar has nothing to
show — every command resolves `--ref` explicitly), **`worktree-status` cannot run** against any repository
the CLI produces, and **there is no `diff` command**. An integration API today would expose those gaps as
the product.

`diff` itself, when scheduled: **first-party, reusing `text_span`'s authoring computation** — not a
display-only crate. The spans `plan_authored_text_span` produces are identity-bearing and signed; a
display diff computed differently would show the user something other than what gets committed, which is
the wrong failure to design into a tool whose claim is that the repository is the evidence.

### Cross-platform mutation — shipped in 0.21.0; two narrower Windows guarantees remain

**MET, 0.21.0 (2026-08-16) — criterion 6.** Linux, macOS, and Windows all mutate; the suite runs on all
three in CI, and a repository authored on Linux, mutated on Windows, and verified on Linux produces
byte-identical object ids. The cost was smaller than "three implementations": the logic is shared, and
what differed was a handful of primitives (anchored `NOFOLLOW` opens, directory fsync) — macOS was a
port, Windows a rewrite.

**Windows carries two documented narrower durability guarantees**, named rather than silent — see
`docs/src/reference/platform-support.md` for the exact gap, including the resolution race Windows
cannot close by construction (no `openat` equivalent). 0.22.0 closed two others that stood through
0.21.0 (`prikk unlock` process-liveness reporting, and the 128-bit anchor identifier).

## Corrective Program After 0.17.7

The independent architecture review of 0.17.7 found a critical ref-publication interruption state and
high-severity gaps in state-root identity, required directory durability, existing-object validation,
and signature-preimage authority. The tracked finding-to-RFC matrix, dependencies, target releases, and
completion gates are in `MILESTONES.md`.

The project remains an experimental architecture implementation. Successful routine gates do not
override reproduced negative evidence. Public-preview readiness remains no-go through M2 and requires
a new independent review after M2. Production suitability remains no-go through M3 and likewise
requires an explicit independent ruling. Repository-format stability is not granted by any scheduled
milestone; it requires a separate future stability RFC and review.

## 0.16.0 Release Task Management

These tasks are tracked here because the original task notes are local scratch space and not a durable
backlog.
They remain managed in this section until each completion condition is met.

| ID | Owner | Status | Trigger / next action | Completion condition |
|---|---|---|---|---|
| TASK-01 docs Phase-2 physical subdirectories | Designer, then architect reviewer | Done | Reviewed and committed | Accepted review and committed docs source-tree move |
| TASK-02 consolidated data-model + trust/threat-model docs | Architect + maintainer | Done | Reviewed, repaired, and committed | DC-24 docs are reviewed and committed |
| TASK-03 docs Pages workflow hardening | Maintainer | Local prep done; external deploy verification pending | After release/docs workflow changes are pushed, run GitHub Actions `workflow_dispatch` or observe the release-triggered Pages run | First Pages build/deploy succeeds, or a tracked follow-up records any GitHub-side failure |
| TASK-04 DC-23 store-unit test carry-forwards | Designer/implementer | Done | Reviewed and committed with the accepted 0.16.0 pre-release hardening bundle | Store-level cross-item test is committed |
| TASK-05 0.16.0 release finalization | Maintainer | Done | Released as tag `0.16.0` and crates published | `0.16.0` tag and crate publish are completed by the maintainer |

## Post-0.16.1 Documentation Reference Backlog

Documentation-only, current-state reference increments targeted for after **0.16.1**, following
DC-24 (data model + trust/threat). They are grounded through the tracked DC-24 baseline recap
(`rfcs/handoffs/DC-24-data-model-trust-threat-docs/baseline-recap.md`) and current released code/RFCs;
local scratch specs are not reviewer-facing authority. Each task must carry source
anchors and mandatory honest-limits caveats when it graduates. Sequencing: Tier 1 first; TASK-06 and
TASK-08 are the largest gaps; TASK-09 rides close behind the DC-24 threat model. These must not creep
into DC-24, and every one must preserve the same honest-limits discipline (unit-test-evidenced
durability, no repository-format stability, `verify` is not a global-trust proof) or it re-creates the
over-trust risk DC-24 exists to prevent.

**Documentation home.** Released **DC-26** retires the DC-24 `rfcs/fdds/` pattern for current-state
references. The durable homes below are authoritative `docs/src/reference/` or `docs/src/guide/` pages;
`rfcs/fdds/` is reserved for future gating FDDs only.

| ID | Tier | Owner | Status | Trigger / next action | Completion condition | Durable home |
|---|---:|---|---|---|---|---|
| TASK-06 durability & crash-recovery reference | 1 | Architect + maintainer | Released in 0.17.2 | Complete; use the reference as the current public durability/recovery baseline. | Reviewed durability/crash-recovery reference is committed. | `docs/src/reference/durability-recovery.md` |
| TASK-07 verify & doctor reference | 1 | Architect + maintainer | Released in 0.17.3 | Complete; use the reference as the current public verify/doctor diagnostic baseline. | Reviewed integrity/recovery docs define what `verify` and `doctor` do and do not prove. | `docs/src/reference/integrity-recovery.md` |
| TASK-08 patch algebra & merge-evidence concepts | 1 | Architect + maintainer | Released in 0.17.1 | Complete; use the reference as the current public concept baseline. | Reviewed current concept page explains commutation, evidence outcomes, and non-goals. | `docs/src/reference/patch-algebra.md` |
| TASK-09 key management & signing setup | 1 | Designer/maintainer | Released in 0.17.4 | Complete; use the guide as the current public signing setup baseline. | Reviewed operator guide is committed and links the trust/threat reference without promising key lifecycle features. | `docs/src/guide/security-setup.md` |
| TASK-10 repository layout & authority model | 2 | Architect + maintainer | Released in 0.17.5 | Complete; use the reference as the current public repository-layout/authority baseline. | Current directories and authority-vs-cache rules are reviewed and committed. | `docs/src/reference/repository-layout.md` |
| TASK-11 path & worktree safety rules | 2 | Architect + maintainer | Released in 0.17.6 | Complete; use the reference as the current public path/worktree safety baseline. | Reviewed path/worktree safety reference is committed with current gaps marked. | `docs/src/reference/path-safety.md` |
| TASK-12 concurrency & locking model | 2 | Architect + maintainer | Released in 0.17.7 | Complete; use the reference as the current public concurrency/locking baseline. | Reviewed locking/concurrency docs are committed and describe manual stale-lock limits. | `docs/src/reference/concurrency-locking.md` |
| TASK-13 release, versioning & compatibility policy | 2 | Maintainer | DC-35 policy implementation and DC-45 Rust authority cutover committed; signer set empty | Bootstrap the first signer only through its separate reviewed governance transaction, observe the public 72-hour hold, and obtain an explicit hold-lift ruling before the 0.18.0 release candidate. | Reviewed release/compatibility policy and Rust authority gate are committed; signer bootstrap evidence, elapsed hold, and hold-lift ruling are recorded before RC preparation. | `docs/src/reference/release-compatibility.md` |
| TASK-14 consolidated non-goals / deferred features | 3 | Maintainer/architect | Open | Start when deferred-feature lists begin drifting across README, ROADMAP, mdBook, and release notes. | Reviewed non-goals page is committed and links ROADMAP as the planning authority. | `docs/src/reference/non-goals.md` |
| TASK-15 roles & user-classes orientation | 3 | Designer | Open | Start when the docs need a clearer audience map after the Reference section settles. | Reviewed orientation page or index update is committed. | `docs/src/index.md` or `docs/src/guide/audience.md` |
| TASK-16 error taxonomy & diagnostics | 3 | Implementer/architect | Open | Start with TASK-07 or when diagnostics need user-facing interpretation. | Reviewed diagnostics reference is committed and grounded in `crates/prikk-error`. | `docs/src/reference/errors.md` |

Scratch detail may exist locally until each task graduates; update the **Status**, **Trigger / next
action**, **Completion condition**, and **Durable home** here as they land. This section is the durable
record, not the scratch files.

## Historical Note — PR-030

PR-030 closed the observability gap after rollback drafts are sealed by the existing `seal --allow-no-audit`
path: sealed history labels Blocks that contain rollback-marked Patch objects, and repository verification
counts sealed rollback Blocks and sealed rollback Patch references. It did not introduce rollback-specific
refs, authorize rollback, mutate the worktree, or change seal publication semantics.
