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
  not move the bytes itself, sealed history is exchanged Linux-to-Windows in CI (`74be0ab`), with the receiver's sealed-block ids byte-identical to the sender's,
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
- **Transport — deferred by RFC 116's accepted ruling, not settled shut.** RFC 116 itself: `prikk-store`
  stays bytes-in/bytes-out and prikk stays off the network *"in this increment"*; *"if a protocol is
  later wanted, it belongs in its own crate or its own binary"*; transport is *"deferred and kept outside
  the verification core."* **What is settled, and survives this correction**: sync-over-any-channel
  already satisfies criterion 1 today, so moving a `sync`/`bundle` artifact between repositories is the
  operator's own channel right now, by design — not an open question about who owns that copy in the
  meantime. **What is not settled**: whether a protocol ever gets built is left open by RFC 116's own
  words, not ruled out by them.

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

### Universal installer and uninstaller — unscheduled; a signer-authority question sits inside it

**Owner's, 2026-08-28:** installing Prikk is harder than it should be for newcomers, and an easier
path would widen adoption — a shell installer of the kind `rustup` and `bun` ship.

**The goal is right and the entry cost is real.** Today a newcomer needs either a Rust toolchain
(`cargo install`) or a manual download-verify-extract-PATH sequence. Both are more work than one
command.

**The constraint, which the design has to answer rather than route around:**
`release-signers.toml` still reads `authorized_primary_fingerprints = []` and is fail-closed, so **no
release yet passes the DC-35 signer-authority audit** — that is criterion 4, open by the owner's
ruling. A `curl | sh` installer is a stronger trust request than a download page: it asks a user to
execute an unreviewed script that fetches a binary whose authority is, by the project's own
statement, not yet established. **Shipping one now would teach users to trust a channel Prikk itself
does not vouch for.**

**A path that does not wait on criterion 4 exists, and should be weighed first:** submission to
package managers — Homebrew, Scoop, the AUR, nixpkgs — where the distribution channel supplies its own
signing and review, and the user's trust decision is one they have already made. That gets
`brew install prikk` without Prikk asserting an authority it does not have.

**Prerequisite:** either criterion 4 closing, or a design that states plainly what the installer does
and does not prove. **Uninstall is the easy half** and should not be an afterthought — whatever
installs must remove cleanly, including `PATH` edits it made.

### Beginner's help — unscheduled; the guide is complete and the on-ramp is missing

**Owner's, 2026-08-28:** starting a project with Prikk is hard for newcomers; guides, tutorials, FAQs
and troubleshooting are needed.

**The diagnosis is confirmed by the shape of what exists.** `docs/src/guide/` has **twenty
feature-organised pages** — one per command or capability — and **no tutorial, no FAQ, and no
troubleshooting page**. There is no narrative "first ten minutes" path, and the guide's second entry
is *Security and Signing Setup*, which a reader meets before they have created anything.

**A newcomer's first questions are not command questions.** *What do I run first? Why did `commit`
refuse? What is sealing and must I do it? Why does this need keys at all?* Those are answered today
only by reading several reference pages and inferring.

**Prerequisite: none — this is writable now.** It is unscheduled by priority, not by blocker. **The
`Post-0.16.1 Documentation Reference Backlog` table below already carries `TASK-15` (roles and
user-classes orientation), which overlaps the audience half of this theme**; whoever takes this should
reconcile the two rather than start a third parallel effort.

### BSD mutation (FreeBSD, OpenBSD) — unscheduled; the blocker is CI evidence, not the port

**Recorded 2026-08-28 after the owner observed that git, Mercurial and — closest to home — Pijul
support FreeBSD, and git supports OpenBSD.**

**The current boundary is an allowlist of reviewed platforms, not a capability limit.** The POSIX
mutation path uses `rustix::fs::openat`, `OFlags`, `fchmod` and `io::dup`, all of which FreeBSD has.
Measured, in a throwaway worktree: widening every `target_os = "linux"` gate to include `freebsd`
produces **exactly two compile errors**, both because FreeBSD's `mode_t` is 16-bit where Linux's is
32-bit (`linux.rs:114`, `read.rs:186`). With two casts, `x86_64-unknown-freebsd` compiles clean and
Linux is unaffected.

**OpenBSD is less certain.** `rustup` has no prebuilt std for it (tier 3), so nothing could be
verified here, and its `mode_t` is 32-bit unlike FreeBSD's — so even the two casts may differ. A
native build using OpenBSD's ports Rust is the only way to find out.

**Prerequisite: a CI mechanism that runs a real BSD kernel — and it is the same prerequisite for
both.** GitHub Actions has no native runner for either; `vmactions/freebsd-vm` and
`vmactions/openbsd-vm` both exist, so this is one third-party action and one policy decision (CI
policy requires actions pinned to a reviewed immutable revision), not two separate problems. The code
asymmetry between the two BSDs is real; the CI asymmetry is not.

**Why the cfg must not be widened before that exists.** Compiling proves nothing about `fsync`
semantics, rename atomicity, or `O_NOFOLLOW` behaviour on a filesystem nobody has reviewed. DC-71 is
the precedent: `prikk-store` once failed to compile off Linux and nobody noticed until CI was taught
to look. **A platform that compiles and silently corrupts is worse than one that refuses cleanly**,
and refusing cleanly is what happens today.

## Open-Work Index

RFC 120: every file in `rfcs/proposed/` is named below, or the gate that checks this section
fails. **This is an inventory, not a priority order** — presence means an item exists and is open,
nothing about when it should be worked or how it compares to anything else. **Not a claim that the
index is true**, only that it is complete with respect to `rfcs/proposed/`; for a theme's current
state, read the RFC itself, or this file's own Future Themes prose above. **Not a replacement for
review records** — `.git-exclude/reviewed/` stays where reasoning lives; this carries only that an
item is open.

<!-- open-work-index:start -->
- [`109-agent-native-interface.md`](rfcs/proposed/109-agent-native-interface.md) — RFC 109, Agent-native interface
- [`110-agent-safety-and-provenance.md`](rfcs/proposed/110-agent-safety-and-provenance.md) — RFC 110, Agent safety and code provenance
- [`113-history-import-foundations.md`](rfcs/proposed/113-history-import-foundations.md) — RFC 113, History import foundations (Git, Subversion, CVS)
- [`DC-43-RELEASE-SECURITY-CONTROLS.md`](rfcs/proposed/DC-43-RELEASE-SECURITY-CONTROLS.md) — DC-43, Release Security and Distribution Controls (schedule position stale — cited predecessor superseded and implemented; see the RFC's own status update)
- [`DC-44-MIGRATION-BACKUP-RESTORE-EVIDENCE.md`](rfcs/proposed/DC-44-MIGRATION-BACKUP-RESTORE-EVIDENCE.md) — DC-44, Migration, Backup, and Restore Evidence (superseded in part by RFC 114's format-refusal ruling; remainder unverified as complete — see the RFC's own status update)
- [`DC-49-PORTABLE-LOGIC-PLATFORM-MATRIX.md`](rfcs/proposed/DC-49-PORTABLE-LOGIC-PLATFORM-MATRIX.md) — DC-49, Portable-Logic Platform Matrix (unblocked — its own blocking wording has been corrected since DC-87 Stage 2; see the RFC's own status update)
<!-- open-work-index:end -->

**Two backlog tables elsewhere in this file carry live open rows of their own, referenced here
rather than absorbed** — the richer `Trigger / next action`/`Completion condition` columns are
what make those rows actionable, not a nag, and flattening them into this thin index would lose
that (RFC 120 §6 Q1). The `0.16.0 Release Task Management` table and the `Post-0.16.1
Documentation Reference Backlog` table both appear later in this file; the latter's `TASK-14`,
`TASK-15`, and `TASK-16` rows are still `Open`.

**Deliberately excluded, by ruling, not oversight** (RFC 120 §6 Q2/Q3): `MILESTONES.md`'s
milestone rows — free prose, no marker a gate can read without interpreting it — and
`rfcs/accepted/`, thirteen files dominated by finished work; an index that lists eight finished
RFCs teaches readers to ignore it.

### Findings without a file

Seeded from review notes that exist nowhere else in this repository — `.git-exclude/reviewed/` is
git-excluded, so a finding recorded only there is invisible to a fresh clone. **The gate cannot
enforce that this section stays current** (RFC 120 §4): its emptiness would not mean no findings
exist, only that none were written down here.

1. `sync build` reports *"already in sync"* when the sender's ref is unsealed — RFC 116 §4's
   deliberate behaviour (a test pins it), but the wording names a state the user is not in. A
   question about wording on a ruled surface, not a defect.
2. No local reproduction path exists for the cross-host CI jobs (`.github/workflows/ci.yml`'s
   `receiver-prepare`/`sender-build`/`receiver-accept`) — verifying the flow before writing the
   jobs required a hand-built, role-by-role proxy that exists in no tracked file.
3. `MILESTONES.md:334`'s `M5` row still reads `| M5 | Sync and Quarantine | no | — |`, while
   criterion 1 (line 159) records sync **MET** and quarantine was dissolved this session.

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
| TASK-14 consolidated non-goals / deferred features | 3 | Maintainer/architect | Complete, committed `7babdb4` | Complete; use the page as the current public refused-vs-deferred baseline. | Reviewed non-goals page is committed and links ROADMAP as the planning authority. | `docs/src/reference/non-goals.md` |
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
