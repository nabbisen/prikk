# Prikk Corrective Milestones

This file schedules the corrective program opened after the independent architecture review of the
released 0.17.7 tree. `ROADMAP.md` remains the concise project backlog, individual RFCs own design, and
`rfcs/IMPLEMENTATION-STATUS.md` remains the current implementation snapshot.

**Authority.** This file is the project **schedule**: release posture, release-lane state, attached
release conditions, and milestone definitions. It is **owner-retained** — major milestones,
major-version timing, significant risk acceptance, and release approval are the project owner's
(`.git-exclude/roles/high-capability-model-operating-instructions.md:9`), and roadmap and milestones
are defined jointly (line 18). The architect edits this file only where an owner ruling, an RFC
acceptance criterion, or a direct instruction names what to change; the developer does not edit it at
all. **Risk and review findings are not recorded here** — a finding lives in the review result that
raised it, and anything that must outlive that review is documented in `docs/` or `rfcs/`.

## Baseline and release posture

The reviewed 0.17.7 tree remains suitable for architecture experimentation and corrective development.
It is not approved for production use, repository-format stabilization, or a public-preview readiness
claim. Successful existing unit and integration gates do not close reproduced crash-state defects.

The corrective implementation freeze ended when DC-35 through DC-40 implementation and evidence were
accepted. Development priority and release readiness now use separate lanes: product/assurance work is
selected by project value, while release work remains dormant until the project owner explicitly
activates preparation for a named version. A completed implementation, target version, or listed next
gate does not itself activate signer bootstrap, a hold, RC preparation, tagging, or publication.

When release preparation is activated, all gates for that target remain binding and release assets must
be current before publication. Accepted RFCs are not individually treated as release readiness.

### Durable release-lane transition

Release activation is an atomic reviewed planning/status commit, not a conversational instruction. That
commit must update all three durable authorities with the same values:

- `ROADMAP.md`, under **Release Candidate Increment**: lane state `active` and exact target version;
- this file, under **Baseline and release posture**: lane state `active` and exact target version;
- `rfcs/IMPLEMENTATION-STATUS.md`: `Current release activation: active` and
  `Current activated release target: <exact version>`.

The transition must land before any fingerprint is requested, bootstrap candidate is prepared, or other
release-lane work begins. Discussion, implementation completion, target-version prose, review
recommendations, and untracked messages are non-authoritative. While no bootstrap transaction or hold
has begun, parking or retargeting uses the same reviewed three-authority transition. Once bootstrap
begins, DC-35's governance, containment, hold, and lift rules govern closure; a planning edit cannot
erase that state.

**Version targets, ruled by the owner 2026-08-08.** `0.19.0` is the **next ordinary minor release** —
DC-74's merge execution is a new CLI verb, so the next release is a minor, not a patch. It is gated on
**DC-75 alone**, which discharges DC-74's release condition. The **M2 assurance-and-distribution
release is retargeted to `0.20.0`** (DC-43, DC-52). Six records previously named `0.19.0` for two
different releases — the next minor and the M2 assurance release — which are far apart in effort;
`DC-45:419`'s Python-retirement gate follows DC-52 to `0.20.0` with it.

Release conditions attach to unshipped accepted increments, not only to a version label. The first
release that contains an accepted but unshipped increment inherits every release condition and
lifecycle/status correction attached to that increment. Therefore, activating 0.19.0 while 0.18.0
remains unpublished would carry forward all M1 gates in addition to applicable M2 gates. Retargeting
also updates this file, `ROADMAP.md`, and every affected RFC target/status statement in the same reviewed
change.

### Status-claim criteria — the "early implementation" badge

`README.md` carries `status-early-implementation`, and immediately below it: *"Do not use Prikk as the
sole store for important project history yet. The repository format and command surface are still
evolving, and future releases may require migration."* **The badge and that sentence stand or fall
together** — removing one while the other holds would be incoherent, so these criteria govern both.

Recorded 2026-08-09 at the owner's instruction, so the question is settled by evidence rather than by
judgement at the time, and so each gap has a visible owner.

| # | Criterion | Why it gates the claim | State today |
|---|---|---|---|
| 1 | **Sync exists** — two machines can exchange sealed history, and both verify it afterward | A *distributed* VCS that cannot distribute. **Corrected 2026-08-19 by RFC 115's investigation: the second half of this row was wrong.** Exchange between repositories does exist — `bundle export`/`import` move a genesis-complete closure, imported history lands in the `remotes/` received namespace without advancing any local ref (DC-78, by design), `merge` accepts a received ref as one side (DC-85), and since DC-53 Stage 2 a receiver cryptographically verifies imported authorship. **All of it is tested end to end** | **OPEN, and exactly one gap remains: transport.** **RFC 115 is implementation-complete and merged 2026-08-20** — patch-level exchange as the owner ruled it (§2.2), across five increments, each reviewed and merged at the exact reviewed commit: the patch-set digest (`bb5424c`), the signed recognition claim (`a56535a`), the `PEXCH001` exchange artifact and accept path (`0128c91`), the D6 amendment making the claim carry block order verbatim (`1e72235`), and seal-from-accepted (`07d8a47`). **A receiver can now accept foreign patches, verify their AUTHOR signatures against transported key material, store them, see what it holds unsealed, and seal it under its own maintainer key in the order a signed claim supplies.** Trust never expands on receipt: no artifact can cause a maintainer key to be adopted. Divergence — an accepted patch not applying to the receiver's tip — is classified and reported **as divergence, not corruption**. `verify` also gained a received-namespace stage, closing the gap where a dangling received ref was invisible on both sides. Ten security refusals in Stage 3 and eleven in Stage 4, each with an observed-failing negative control. **Owner ruled 2026-08-20 (RFC 116, accepted):** the next increment is **negotiation-as-artifacts**, not a network protocol — negotiation is a prerequisite of any protocol, so a protocol without it could only send whole histories. And **this row permits sync-over-any-channel**: its load-bearing clause is *"and both verify it afterward"*, which holds identically however the bytes arrive, so `prikk-store` stays bytes-in/bytes-out and **prikk stays off the network** — every RFC 115 refusal already treats the transport as untrusted, so moving the bytes itself would add a large attack surface and no verification strength. **Stated limit:** confidentiality becomes the user's choice of channel rather than a prikk guarantee (prikk offers none today either). **The architect proposed this reading and disclosed that it also favours the architect's own recommended path before the owner ruled.** **What is left: negotiation, then optionally transport.** There is still no network code in the workspace; RFC 115 §3 surveyed four options and **none is chosen** — an owner decision, not an increment waiting to start. **The reading that gates this row was stated before the work, not after** (RFC 111 §8's precedent): "exchange sealed history" requires that the *receiver* ends holding sealed history, which is why seal-from-accepted was required and why it was scheduled ahead of transport. **§5.1's test and security discipline remains binding on the transport work** |
| 2 | **The format-stability question is answered, and its answer honoured** — either canonical encoding is frozen with a stated compatibility promise, or a migration guarantee exists that a repository written by one release is readable by the next | The badge's own sentence says migration may be required. DC-75 added two `BlockPayload` fields on 2026-08-08 | **MET, 2026-08-19** — RFC 114, merged `984c8f1`. **Answered:** verification-bearing bytes are frozen forever (the object-id preimage, each shipped `(object_type, schema_version)` pair's canonical encoding, the signature preimage, the algorithm identifiers); representation may change behind a documented, tested migration path (repository format, containers, index, WAL, bundle). **Honoured by gates, not by intention:** literal identity vectors per shipped pair that fail if the frozen surface moves, a completeness guard so an admitted pair cannot start being written without one, and a tripwire making a `CURRENT_FORMAT_VERSION` bump **unable to pass CI without migration coverage** — each observed failing before its fix. **Freezing is not "never add a field":** `schema_version` sits inside the id preimage, so a new field is a new version and nothing already written moves. **Scope, per the owner's ruling:** the contract binds **format 6 onward**; formats 1-5 are **not supported** (prikk has never been in production), and their refusal messages now say so rather than offering a migration the product cannot honour. **Residual:** the format-7 carry-forward operation does not yet exist — the tripwire guarantees someone is *stopped*, not that the operation is ready (RFC 114 §5.2) |
| 3 | **`verify` is not superlinear in history length** | The central claim is offline verifiability by anyone. At roughly O(N³) — 34 s at 160 blocks — that stops being practical at a few hundred commits | **MET, 2026-08-18** (DC-92 `b718623`, then RFC 111 `13f7a4b`/`ffaab08`). `verify` is linear: **27.04 ms at N=160, per-doubling ratio 1.97**, against 167.85 ms and ×3.51 before RFC 111. **Held by a gate, not a measurement** — `rfc111_index_decode_cost_gate.rs` fails if `verify`'s full-index-decode count ever grows with repository size again, and was observed failing before its fix. **Read as written, and the reading is stated rather than assumed** (RFC 111 §8): this criterion names `verify`, and its own rationale is *offline verifiability by anyone* — a reader's cost. `seal`'s cost is authoring throughput and does not gate that claim, so it is **not** counted here. **What is therefore not covered:** `seal` still performs O(N) reads per call (`derive_next_state_root` walks the lineage with a deliberately per-call memo), so building N commits remains O(N²) in total reads — 46.96 ms at N=160 on disk after RFC 111, down from 93.86 ms, flat-ratioed but not eliminated. That residual is real, is **owned by no increment**, and is not what this criterion measures |
| 4 | **The signer bootstrap has occurred** — DC-35's authority transaction, two distinct natural persons | Release authority has never been established for any release to date | **`release-signers.toml` empty and fail-closed.** `docs/src/reference/release-compatibility.md:16` already states no release satisfies the gate |
| 5 | **`verify` checks author signatures repository-wide** — **Corrected 2026-08-09 by DC-78's investigation: NOT a prerequisite of criterion 1.** Exchange is ruled to claim only "sealed by a Maintainer key you adopted", which `trust.rs` can already verify — so a receiver verifies exactly as much as a local user does, and criterion 1 is reachable without this one. **But shipping exchange makes this criterion more important, not less**, since other people's history then arrives with authorship unchecked | The positioning is that every change is signed by its author *and verifiable by anyone*. **Delivered 2026-08-18** across DC-53 Stage 1 (`970bc27`), Stage 2 Step 1 (`27088c9`) and Step 2 (`89036bf`). `verify` cryptographically checks every reachable Patch's AUTHOR signature; a forged signature over recorded material fails `verify`'s exit status; one `key_id` binds to one public key, enforced at record time and again at verify time; and author key material now travels with a bundle (`PBNDL002`), so history received from another party verifies instead of reading *Unverifiable* forever | **MET, with a stated limit — read the limit before citing this row.** What is delivered: authorship is checked everywhere, including received history, and a contradiction fails closed. **What is not: this is trust-on-first-use.** Transported key material is supplied by the sender, so a bundle that verifies proves the signature and the key agree *with each other* — **not** that the key belongs to the named author. Pinning buys **continuity of authorship** (the same `key_id` always carries the same public key, and a later contradiction is refused), not authenticity of first contact. **The claim this row supports is therefore "prikk verified this is the same author as last time", not "prikk verified who this author is"** — `docs/src/reference/trust-threat-model.md` states it where a reader will meet it. **The reading was chosen before the outcome was known** and not because it let the row be marked met (RFC 111 §8's precedent). Residuals: no key rotation, revocation or expiration (rotation is refused, indistinguishably from impersonation); a conflicted `key_id` has no remedy; `doctor` surfaces none of these outcomes, which is an open scope question |
| 6 | **Mutation works wherever the project claims support** | A tool that can read but not write on two of three supported platforms is not past "early" | **MET, 0.21.0 (2026-08-16).** Linux, macOS, and Windows all mutate; the suite runs on all three in CI, and a repository authored on Linux, mutated on Windows, and verified on Linux is required to produce byte-identical object ids. Windows carries four documented narrower guarantees (`platform-support.md`) — named, not silent |

**These are necessary, not certified sufficient.** Whether the list is complete is the owner's call when
the last one closes; the architect's record of prediction this cycle does not justify claiming otherwise.

**Deliberately not criteria, with reasons**, so their absence is a decision rather than an oversight:

- **Conflict resolution.** Conflicts are detected and refused cleanly, with no partial state. Refusing
  well is a defensible posture for a tool that is otherwise mature; resolution is a capability, and a
  resolution is itself a patch somebody must sign.
- **Merge-base discovery.** Manual `--baseline-block` is a usability limit, and a wrong baseline is
  refused rather than mis-merged.
- **Unbounded lifecycle state.** Real, and it decides ten-year viability — but it degrades gradually
  rather than making the tool unfit today.

### Attached release conditions

| Increment | Condition | Ruled |
|---|---|---|
| **DC-74** — merge execution (**implemented `3464e2a`; condition DISCHARGED 2026-08-08 by DC-75 at `c79c421`**) | ~~**Merge execution does not ship until sealed history structurally records a merge**~~ — a later verifier must be able to re-check the baseline and both sides. DC-74 is buildable and mergeable now; it is not releasable until this holds. **DISCHARGED by the architect 2026-08-08**, per DC-75's amended criterion 5 (the developer reports the technical content; the architect records the discharge, since a release condition is the object of implementation review and cannot be self-certified). Merges now seal as `BlockKind::Merge` recording both parents, a mainline pointer, and the baseline; `verify_merge_baseline` **re-derives** rather than trusts the recorded baseline, walking both parents' ancestries. Verified independently at review (`.git-exclude/reviewed/DC-75-implementation-review-v1.md`). **Scope of the discharge:** `verify` confirms the baseline is *a* genuine common ancestor of both parents, not the *lowest*, and does not re-run confluence — per the accepted ruling that a merge is trusted on the maintainer signature as every other sealed decision is. **This condition no longer blocks a release; it does not authorize one.** | Owner, 2026-08-08; discharged 2026-08-08 |

**Why this condition exists, and why it cannot be deferred.** A merge under DC-74's adoption model is
sound only if the two sides were confluent from a common baseline. `parent_patch_ids` is inert — set to
`Vec::new()` at every construction site including the authoring path, and read nowhere — so with
single-parent blocks nothing in sealed history records what the confluence was checked against. **The
irreversible event is a user sealing a merge**, which cannot occur before a release exposes the command.
History is immutable, so a merge sealed under-recorded stays under-recorded permanently; no later
increment can repair it without rewriting sealed history. This is the only condition on the board that
is a one-way door.

If the three authorities disagree, the release lane is parked. No release-lane work may begin until a
reviewed commit restores agreement.

**Current release lane:** `parked`.

**Current activated release target:** none.

**0.22.1 released 2026-08-17** — tag `0.22.1` signed with the owner's key
(RSA `25757DA6CBF7022C4E14CCAC1B3066B87DB99A34`) on `df0a951`, 12/12 CI green at that commit, and **the
first release cut through RFC 107's new workflow**: four build targets, twelve assets, and a release page
assembled from this version's own changelog entry. **Every published checksum was verified by hand with
`sha256sum -c` after the cut** — all four `OK`, including the Windows `.zip`, which discharges RFC 107's
criterion-7 residual with a real artifact rather than an argument. All eight crates published to
crates.io. The lane was parked the same day.

**0.22.1 activated 2026-08-17 by the project owner.** RFC 107, the release distribution surface, from two
defects the owner found directly: prebuilt binaries were Linux-only although macOS and Windows have been
supported since 0.21.0, and a release page told a visitor nothing about the release it belonged to. **A
third was found while fixing them**: the static notes template claimed *"repository mutation is Linux-only
project-wide (DC-37)"* — true when written for 0.20.0, false on the 0.21.0 and 0.22.0 pages, i.e. the two
releases whose entire content was making mutation work on Windows.

**User-facing content**: prebuilt binaries for macOS (`aarch64-apple-darwin`) and Windows
(`x86_64-pc-windows-msvc`) alongside the two Linux targets, and release pages that carry the version's own
changelog entry. No product code changed; this is the distribution surface only.

**0.22.0 released 2026-08-17** — tag `0.22.0` signed with the owner's key
(RSA `25757DA6CBF7022C4E14CCAC1B3066B87DB99A34`) on `16313fe`, 12/12 CI green at that commit, GitHub
release published with six assets, and all eight crates published to crates.io. The lane was parked the
same day.

**0.22.0 activated 2026-08-17 by the project owner.** Windows capability parity and the verification
work behind it. **User-facing content is two Windows capabilities**: `prikk unlock` now returns a real
liveness answer there (it previously reported `unknown` unconditionally, leaving every stale-lock
decision to the operator), and anchor identity uses the 128-bit form where the filesystem supports it,
which matters on ReFS — Windows 11's Dev Drive. **A Linux or macOS user receives no observable change**;
everything else since 0.21.0 is Windows-specific or internal, and the release notes must say so rather
than imply otherwise.

Behind those: DC-97 classified all nine `DurabilityContract` guarantees on Windows, DC-98 demonstrated
crash-safety there with nine controls each watched to fail and retired the orphaned
`promote`/`publish_immutable` surface, RFC 105 turned RFC 100's naming rule into a `boundary-check`
control, and RFC 106 proved the anchor identity guard that 936 tests had not depended on. Windows tests
**909 → 956**.

**0.21.0 released 2026-08-16** — tag `0.21.0` signed with the owner's key
(RSA `25757DA6CBF7022C4E14CCAC1B3066B87DB99A34`) on `4a33b49`, 12/12 CI green at that commit, GitHub
release published with six assets, and **all eight crates published to crates.io** — the first release
to publish `prikk-ffi`. The lane was parked the same day.

**0.21.0 activated 2026-08-16 by the project owner.** The lane was parked from 0.20.0's release until
activation. 0.21.0 carries DC-87 (Windows mutation, Stages 1 and 2) and DC-96 (Windows anchor identity) — 23
commits since 0.20.0, all of one theme. **Windows becomes a mutating platform**, the first change to
prikk's platform posture since the project began. `docs/src/reference/platform-support.md` states four
residual properties a Windows operator meets: DC-87 criterion 4's nine `DurabilityContract` negative
controls are not run on Windows; `prikk unlock` returns no positive liveness signal there; the 64-bit
file index is unreliable on ReFS (Windows 11 Dev Drive); and a repository directory cannot be renamed
while a prikk command holds it open. All four are accepted and documented, not discovered late.

**0.20.0 released 2026-08-16**; the lane was parked the same day. It is activated only while a release
is being prepared, so a shipped target does not leave it standing open.

**Activated 2026-08-16 by the project owner.** 0.20.0 carries RFC 102 complete — container-based
durability across six stages, repository format 6, compaction — plus `prikk compact`, `prikk unlock`,
`prikk trust maintainer remove`, and the dead-surface consolidation. **0.20.0 does not deliver Windows
mutation** — at the time of this release Windows remained read-only, unchanged from 0.19.0; holding a
finished body of work for unstarted work is what had produced a 393-commit gap since 0.19.0.

**DC-87 (Windows mutation) completed 2026-08-16**, Stage 1 and Stage 2, together with DC-96 (Windows
Anchor Identity — a security-hardening remedy Stage 2's own CI job found necessary before Windows
mutation could ship: renaming a repository's directory tree and creating a fresh one at the same path
redirected both reads and writes into the impostor, silently). Twelve of twelve CI jobs green across
Linux, macOS, and Windows on branch `dc87-stage2-windows`, not yet merged to `main`. Targets 0.21.0.

**0.19.0 released 2026-08-08** — seven crates published, tag `0.19.0` signed with the owner's key
(RSA `25757DA6CBF7022C4E14CCAC1B3066B87DB99A34`), CI and Release workflows green, six assets published.
**Activated at `abef69b` before any preparation began**, per the transition rule — the second release to
follow it. Verified after publication: `prikk 0.19.0` resolves all six internal crates at 0.19.0 with no
0.18.x leakage (the 0.18.0 defect, confirmed absent), the published Linux x86_64 binary's checksum
verifies, reports `prikk 0.19.0`, and carries the `merge` verb. Lane parked immediately on completion. **Post-release stability rerun accepted 2026-08-08** from `664488f` (`.git-exclude/reviewed/prikk-0.19.0-post-release-stability-rerun-v1.md`): Python and Rust agree, the deliberate-disagreement self-test fires, all repository gates pass. **0.19.0 is closed**, and `DC-45:419`'s Python-retention condition is discharged.

Activated 2026-08-08 by the architect under the owner's delegation of minor/patch release scheduling,
with the owner's explicit approval to proceed, and **before any preparation began**.
**Why now:** DC-74 merge execution and DC-75 merge block lineage are both accepted, and DC-74's release
condition was **discharged at `c0f29b5`** — 0.19.0's sole gate. **Why minor:** `prikk merge` is a new CLI
verb and `BlockPayload` gains two optional fields; no existing object id moves (proved — no hash literal
changed in `c79c421`, DC-41's vectors unchanged), so this is additive, not breaking.

**0.18.4 released 2026-08-04** — seven crates published, tag signed, release workflow green with six
assets. **This is the first release since 0.18.0 to follow the transition rule**: activated at `4378643`
before any preparation began, and parked here on completion. The two releases that skipped it are
recorded below.

**Process finding, recorded 2026-08-04 by the architect, about the architect.** 0.18.2 and 0.18.3 were
both prepared, published, and tagged while this lane read `parked`. **No three-authority activation
commit was made for either.** Only 0.18.0 got one (`dae292e`); the lane was parked after 0.18.1 and never
reactivated.

Owner authorization existed for both releases, so no release was unauthorized. What was bypassed is the
**durable record**: the rule above exists so release work is a reviewed, recorded decision rather than a
conversational one, and it states plainly that discussion and untracked messages are non-authoritative.
Two releases now rest on exactly that.

**The next release activates properly before any preparation begins** — version bump, changelog, publish,
and tag all count as release-lane work under this section's own wording.

**0.18.1 released 2026-08-03** — seven crates published to crates.io, tag `0.18.1` signed and pushed. The
lane returns to parked on completion; the next activation is a fresh three-authority transition.

Recorded: 0.18.0 was tagged but **never published** — a `cargo publish --dry-run` pass found internal
crate dependencies declared `version = "0"`, which `^0` resolves, so a published `prikk 0.18.0` would have
accepted `prikk-store` 0.17.7. Pinned to exact versions and shipped as 0.18.1. **Publish is irreversible;
a version number is not** — spending one was the cheaper error.

0.18.1 does **not** pass the DC-35 signer-authority audit and says so in its own release notes:
`release-signers.toml` remains empty, no authority transaction was performed, and `main` was unprotected.
Signature presence on the tag is not signer authority.

Milestones below are dependency-ordered, not calendar promises. Target versions identify the intended
release boundary and may change only through an update to this file, `ROADMAP.md`, and affected RFCs.

## Two milestone schemes — resolve gate labels here first

**This file's `M0`–`M3` are the corrective scheme. Requirement gate labels in
`specs/prikk-non-functional-requirements-v1.1.md` are NOT.** They belong to the original product scheme in
`specs/prikk-roadmap-milestones-v1.1.md`, which reuses the labels `M0`–`M3` with different meanings and
continues to `M7`.

Recorded 2026-07-29 by DC-42 design review v2 (finding B4). The collision had already caused a careful
architect review to misread a requirement gate and conclude that overdue work was not yet due. Every one
of the 38 NFR IDs carries such a label, so the error is available in both directions — it can make overdue
work look scheduled, or scheduled work look overdue.

| Label | Product scheme (what NFR gates mean) | Delivered? | Corrective scheme (this file) |
|---|---|---|---|
| M0 | Design Lock and Safe Scaffolding | yes | Architecture ratification |
| M1 | Core Storage and Identity | capability yes — **but NFR-PERF-01 unmet** | Corrective storage and identity baseline |
| M2 | Minimal Patch Engine | yes | Assurance and distribution baseline |
| M3 | Block DAG and Checkout | **partially** — checkout and block DAG shipped; multi-patch active blocks implemented (DC-66); **NFR-PERF-02 now met** (DC-57's 800/1000 thresholds, configurable, enforced); **NFR-PERF-03 remains unowned** (merge-scope bounding is a DC-57 non-goal) | Migration and recoverable backup |
| M4 | WASM Plugin and Audit | no | — |
| M5 | Sync and Quarantine | no | — |
| M6 | Alpha Hardening | no | — |
| M7 | Public Preview Readiness | no | — |

The corrective scheme is a remediation track laid over the product scheme after the independent
architecture review; it does not replace it. A requirement gated at product M1 is **overdue today**
regardless of where the corrective track has reached.

When citing a gate, name the scheme: "product M3" or "corrective M2", never a bare "M3".

## Finding ownership

**Moved, then removed.** A separate finding-and-risk register was split out of this file on
2026-08-08 so that authorship followed the file. **It was removed on 2026-08-16 by owner decision**: it
had grown to 105 rows and 87KB, roughly half of them duplicating milestones this file already schedules
and the rest an unscheduled backlog nobody read, so it cost the reviewee more to search than it returned.

**What replaces it.** A review finding lives in the review result that raised it. Anything that must
outlive that review is documented where it belongs — `docs/` for behaviour and limitations users or
operators need, `rfcs/` for decisions and scope. Nothing accumulates in a third place. Records written
before 2026-08-16 that refer to the register are accurate as of their own date and are deliberately left
unrewritten.

## M0 - Architecture ratification

**Release target:** none.

**RFC:** DC-34 Publication and Identity Authority.

**Status:** Complete; architect re-review accepted DC-34 on 2026-07-14 and it is tracked in
`rfcs/accepted/`.

M0 selects the ref publication commit point, valid interrupted states, retry/doctor authority, the
literal version-1 signature preimage, and the RefUpdate no-clock sentinel. DC-38 through DC-40 may not
begin identity-bearing implementation before DC-34 is accepted by architect review.

**Completion condition:** Satisfied. DC-34 was reviewed, repaired, re-reviewed, and moved to
`rfcs/accepted/` with roadmap/index/status links updated.

## M1 - Corrective storage and identity baseline

**Release target:** 0.18.0.

**RFCs:**

1. DC-35 Release Compatibility and Status Correction.
2. DC-36 Existing-Object Publication Integrity.
3. DC-37 Required Filesystem Durability.
4. DC-38 Ref Publication Crash Recovery.
5. DC-39 Signature and Envelope Authority.
6. DC-40 State Merkle Root and Format Transition.

DC-36 and DC-37 designs were accepted on 2026-07-15. DC-37 implementation was accepted and committed,
and DC-36 immutable object publication implementation was subsequently accepted. DC-38 ref publication
recovery implementation was accepted and committed after repair re-review on 2026-07-15. DC-35's
repository-governed multi-signer and break-glass amendment was accepted after architect design re-review
v3 on 2026-07-15. Architect repair re-review v3 accepted its policy implementation on 2026-07-16 after
byte/object, canonical-governance, tag-shape, and attempt-growth repairs. No signer is admitted; bootstrap
remains a separate prerequisite. DC-45 design acceptance and the later Rust authority cutover are
complete. Bootstrap therefore uses the accepted Rust release-policy gate under the separately reviewed
DC-35 governance transaction. Neither DC-45 acceptance nor cutover authorizes bootstrap. DC-39
architect review v1
required authority over the public canonical serializer and strict Ed25519 signature shape. The
design repair adds those rules, the invalid-predecessor `add_signature` invariant, and deterministic
diagnostic multiplicity/order while retaining the DC-34 preimage vector, canonical envelope tuple,
format-1 diagnostics, writer inventory, and companion RefUpdate no-clock erratum. Architect design
re-review v1 accepted the repaired design on 2026-07-22. Implementation repair re-review v2 accepted
the bounded candidate, committed as `8f565f2`; post-commit evidence review v1 accepted independent
no-hardlink checkout and deterministic-archive evidence on 2026-07-22. DC-39 implementation is
complete. This closure does not authorize release activity.
DC-38 and DC-40 designs, including the DC-40 companion state-root/format FDD, were accepted on
2026-07-14. DC-40 architect implementation review v1 required four repairs covering strict format-2
read admission, anchored mutation authority, exact legacy cleanup authority, and the end-to-end
format-1 command matrix. Architect repair re-review v1 accepted the repaired candidate, committed as
`70c3902`; post-commit evidence review v1 accepted independent checkout/archive identity and focused
plus full regression evidence on 2026-07-23. DC-40 implementation delivery is complete. Release
activation is parked: no signer bootstrap, hold, or RC has started. If the project owner explicitly
activates 0.18.0 preparation, the first release-lane action is the separately reviewed initial DC-35
release-signer bootstrap governance transaction.

**Conditional M1 first-shipping-release activation order (currently labeled 0.18.0):**

1. Prepare the initial DC-35 signer-bootstrap transaction as an isolated public governance change with
   the required non-secret proof, two distinct accountable approvals, and branch-governance evidence.
2. After that transaction becomes public, keep release publication blocked for at least 72 hours.
3. During the hold, re-run the literal older-review stale-pointer/ahead-log crash reproduction against
   the accepted DC-38 state machine and record the exact result as the B1 release-condition check.
4. During the hold, align README and other tracked public portability claims with DC-37's accepted
   0.18.0 support matrix: mutation is experimental on qualifying Linux local filesystems; macOS and
   Windows remain read-only/diagnostic unless the required primitives and crash evidence are reviewed
   before RC. Record in durable tracked authority that the excluded NFR-PORT-01 source is historical
   input, DC-37 is current 0.18.0 authority, and broader cross-platform mutation remains a deferred
   design target rather than current support or a silently waived goal.
5. After the minimum interval and required containment/classification checks, obtain the explicit
   architect/security hold-lift ruling required by DC-35.
6. Only then prepare the combined 0.18.0 RC, refresh all release assets before publication, run the full
   relevant gate and corrective failpoint matrix, and request adversarial RC review.

This sequence is dormant until the reviewed tracked activation transition. It carries forward unchanged
if a later target first ships the unshipped M1 increments. The bootstrap transaction, hold-lift ruling,
and RC are separate authority decisions. Evidence work and documentation correction during the hold do
not shorten it. The cosmetic unknown/malformed-marker diagnostic remains optional and does not block the
ordered sequence unless separately selected.

**Release condition:** all five blocking findings are closed by accepted implementation review; the
reproduced ref failure no longer succeeds; the state-root and signature vectors are pinned; format-1/
format-2 behavior is explicit; release/status documentation is current; the full relevant gate set and
corrective failpoint matrix have observed passing evidence; and an adversarial 0.18.0 release-candidate
review accepts the combined state. No production or public-preview claim follows automatically.

## M2 - Assurance and distribution baseline

**Development status:** active at DC-41 design review against the accepted corrective baseline.

**Eventual release target:** 0.19.0, subject to explicit release activation and all applicable release
gates.

**RFCs:**

1. DC-41 Integrity Evidence Campaign.
2. DC-59 Commit Benchmark Harness (**implemented `a9c2fe0`**), DC-56 Commit Full-Tree Scan Compliance,
   DC-57 Active-Patch Thresholds (**held** — see the capability gap above), DC-58 Source-Structure Audit
   (batch 1 at `e1d0213`). DC-56/57/58 supersede DC-42, archived 2026-07-29; DC-59 was split from DC-56 at
   design review and produces NFR-PERF-01's named evidence artifact.
3. DC-43 Release Security and Distribution Controls.
4. DC-45 Release Policy Tooling Consolidation.
5. DC-46 Workspace Rust 1.85 Compatibility.
6. DC-47 Stable Clippy Gate Alignment (complete at `ea95e92`; post-commit evidence accepted).
7. DC-48 Legacy Clippy Production Retirement (complete at `383e503`; post-commit evidence accepted).
8. DC-49 Portable-Logic Platform Matrix (blocked on the M1 portability-claim correction).
9. DC-50 First-Party SHA-256 ROI Decision (closed at `4005efb` with a **replace** decision; produced no
   code and authorized exactly one successor, DC-55).
10. DC-51 Product Dependency Placement Gate.
11. DC-52 Python and Oracle Decommissioning.
12. DC-55 First-Party SHA-256 Replacement (the implementation DC-50 authorized; identity-bearing;
    accepted 2026-07-28, **implementation complete at `753ebab`**, implementation re-review accepted
    2026-07-29).

DC-49 through DC-52 were added on 2026-07-28 to give an owner to obligations that previously existed only
in architect review prose. DC-55 was added the same day for the same reason — DC-50's replace decision
would otherwise have been an authorization living only in a decision record. Of these five, DC-50 is
accepted and closed, DC-51 is accepted and implemented at `d3e939b`, and **DC-55 is accepted and
implemented at `753ebab`**; DC-49 and DC-52 remain proposed and each requires individual design
acceptance.
DC-49 is the platform matrix descoped from DC-41 and is the only development increment blocked on a
release-lane event. DC-53 (repository-wide AUTHOR trust verification) is recorded as a post-M2 capability
gap and is deliberately not part of M2. The recommended sequence across all open work is in
`rfcs/EXECUTION-ORDER.md`.

DC-45 through DC-48 are preparatory M2 tooling and compatibility increments that landed before M1
release. They are not the remaining post-0.18.0 execution sequence. DC-45's design was accepted after
architect repair re-review v1 on
2026-07-16. Profile hardening and the observation adapter were committed, and architect implementation
repair re-review v1 accepted the exact-byte oracle semantics on 2026-07-17. Project-owner acceptance was
initially withheld pending a compact tracked representation that avoided the candidate's 237 per-case
vector files. Architect footprint QA conditionally approved three strict suite packs, and architect design
amendment re-review v1 accepted the pack, location, closure, and archive contract on 2026-07-17. Compact
implementation was then prepared without staging for implementation re-review. Owner acceptance,
isolated commit, and source-archive evidence were required to precede Rust implementation. Compact
implementation review v1 found one blocking dot-segment grammar defect. Architect repair re-review v1 accepted its
narrow repair on 2026-07-17. Architect design repair re-review v1 accepted the
explicit retirement schedule on 2026-07-17, satisfying the lifecycle-design condition for that separate
owner decision. Five Python oracle authoring/verification files remain through the first Rust-gated
0.19.0 release. The first later release-candidate increment is blocked until an architect accepts the
later-commit stability rerun; the following release-candidate increment is blocked until the exhaustive
five-file decommissioning review removes each file or records an individual owner-approved, event-bound
exception. The accepted Rust implementation was required to replace the complete manifest verifier and
self-test matrix, and later did so. The other eight frozen evidence/contract files remain until a later
equivalence-backed replacement/consolidation
review or an explicit final-retirement review closes migration and rollback needs. These blockers
remain durably tracked if DC-45 moves to `done/` before their completion. The project owner committed
the exact 13-file oracle with the reviewed design/status update as stage-1 freeze commit `47aec9c` on
2026-07-17. Deterministic archive, checkout/extracted verification, direct-dependency/identity, and
seven-product-package exclusion evidence was accepted after architect post-commit evidence review v1
on 2026-07-17. Stage-2 Rust implementation was accepted after architect repair re-review v11 and
committed as `6a65a35` on 2026-07-21. Its deterministic archive, isolated checkout/extraction,
Python/Rust engine, differential, boundary, reference, identity, and seven-product-package exclusion
evidence was accepted after architect post-commit evidence review v1 on 2026-07-21. Preparation of an
isolated authoritative-command cutover candidate and disposable rollback rehearsal was then authorized.
Preparation found that the accepted stale-reference gate hardcodes Python live authority and cannot
validate an inventory/documentation-only Rust switch without a Rust-source transition repair. Focused
architect QA v1 accepted a separate exact two-state transition repair before cutover implementation
resumed. Architect implementation review v1 accepted the Python-primary repair, and it was committed
as `2bfb7cc` on 2026-07-21. Post-commit preservation evidence was accepted after architect review v1
on 2026-07-21. The exact four-file inventory/live-reference cutover was committed as `6a8e365`;
deterministic archive, clean checkout/extraction, full gate, and committed-identity rollback evidence
was accepted after final architect ruling v1 on 2026-07-21. The Rust command is governance-
authoritative. Python and the frozen oracle remain required through the first Rust-gated 0.19.0 release
and an accepted later-commit stability rerun.
The remaining M2 development order is DC-41, then DC-59/DC-56/DC-57/DC-58, then DC-43. This is
program sequencing, not implementation authority: each remains proposed until its individual design
review is accepted. DC-41's former DC-36-through-DC-40 implementation dependency is satisfied; it
may start now against the accepted committed baseline. Release-specific reproduction and gate evidence
must be rerun when an RC is explicitly selected; development evidence does not silently become RC
evidence. DC-56 follows that evidence baseline; read-only measurement before then does not authorize
optimization or broad source moves. DC-43 follows them, requires security/architect design review, and
must consume the stable Rust release-policy gate rather than extend the retained Python oracle. DC-43
completion remains required before any public-preview reconsideration. Separately authorized read-only
DC-56 measurement or credential-free DC-43 policy drafting may be prepared earlier, but proposed RFCs
never authorize implementation.
DC-46 design selected restoration of the declared Rust 1.85 locked-workspace contract through three
bounded source rewrites, focused trust regressions, and pinned locked CI gates. Architect design
rereview v1 accepted it on 2026-07-21. Architect command-grammar amendment QA v1 then authorized five
exact ordinary-Cargo vectors and existing scanner tests after the prepared candidate exposed a DC-45
classifier conflict. Architect implementation review v1 accepted the complete candidate on 2026-07-21;
it was committed as `0d221af`, and architect post-commit evidence review v1 accepted its clean
checkout/archive evidence. DC-46 and the Rust 1.85 compatibility blocker are complete; DC-45 does not
silently absorb this resolved product-workspace mismatch.
DC-47 was the accepted pre-0.19.0 release-candidate correction for the then-remaining Clippy command
divergence: DC-35 public release guidance selects `--all-features`, while current stable CI and the
DC-45 governed classifier selected the no-all-features vector. Its design preserved the stronger
release gate and added one exact non-authority classifier production. Architect design review v1
accepted the bounded design on 2026-07-21. Architect legacy-vector test-contract QA v1 resolved the
retained-vector contradiction and authorized bounded implementation. Architect implementation review
v1 accepted the candidate on 2026-07-21, committed as `ea95e92`. Architect post-commit evidence review
v1 accepted its clean checkout/archive evidence on 2026-07-21, completing DC-47. DC-48 then separately
retired both unconsumed legacy Clippy productions required before the 0.19.0 release candidate; its design
was accepted after architect review v1 on 2026-07-22 with exact bare/prefixed A/B rejection evidence
required. Architect implementation review v1 accepted the bounded candidate, committed as `383e503`.
Architect post-commit evidence review v1 accepted its clean checkout/archive evidence on 2026-07-22,
completing DC-48 and the legacy-Clippy-production blocker.

**Current DC-45 through DC-48 disposition:** Rust release-policy authority, Rust 1.85 compatibility,
stable all-features Clippy alignment, and legacy Clippy production retirement are complete. Only DC-45's
event-bound obligations remain: retain Python and the frozen oracle through the first Rust-gated 0.19.0
release, obtain accepted later-commit stability evidence, and complete the separately reviewed Python/
oracle retirement or consolidation events. These obligations do not alter the DC-41 through DC-43
program order.

**Completion condition:** reproducible crash/fuzz/hash/platform evidence is available, performance and
source-structure gates are enforced or carry reviewed exceptions, and release artifacts have reviewed
security reporting, dependency policy, SBOM, digest, and provenance controls. The release-policy tool
is consolidated behind the reviewed Rust command with the public schema, product publication graph,
and differential oracle evidence preserved.

After M2, request a new independent architecture review. Public-preview readiness, repository-format
stability, and production suitability remain separate decisions and are not milestone completion
side effects.

## M3 - Migration and recoverable backup

**Release target:** not assigned; scheduled after M2.

**RFC:** DC-44 Migration, Backup, and Restore Evidence.

M3 owns NFR-REL-03 and the migration/restore exercises intentionally excluded from the 0.18.0 format
transition. It defines verifiable export/restore and either exercises format migration or records an
explicit superseding recovery contract. This work is not implied by DC-41's broad evidence campaign.

**Completion condition:** reviewed manifest/version authority, offline backup verification, restore and
retry fixtures, at least one migration rehearsal, and independent architecture acceptance. Production
suitability remains no-go before M3 or a superseding reviewed decision; public-preview consideration
after M2 remains a separate narrower ruling.

## Deferred until selected

- Branch lifecycle expansion, remotes/sync, rollback publication, plugins/audit, and key lifecycle
  features are no longer frozen solely by release status. They remain unselected and require
  design-first prioritization. Merge execution is no longer in this bucket — selected and implemented
  as DC-74; see this file's "Attached release conditions" for what still gates its release.
- TASK-14 through TASK-16 documentation themes remain queued. TASK-13 is the narrow exception because
  compatibility and release rules are required for the corrective format transition.
- Any newly discovered correctness or identity defect interrupts this sequence and receives its own
  RFC or an explicit amendment to the owning proposed RFC before implementation.
