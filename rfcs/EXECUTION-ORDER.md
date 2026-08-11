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
from. **DC-57's handoff v1 is withdrawn — issue v2 only.**

| # | Increment | State | Blocked by | **Handoff to give developers** |
|---|---|---|---|---|
| 1 | **DC-58** — source-structure audit | **Complete.** N1 reframing `6f53da3`, reviewed and accepted 2026-07-31 | nothing | — |
| 2 | **DC-61** — branch closure (§6.5 deletion half) | **Complete.** Implemented `ca4c044`, reviewed 2026-07-31 (accepted with one non-blocking finding), **N1 repaired `2394f1b`** | nothing | — |
| 3 | **DC-56** — commit scan + memory compliance | Implemented `8748f00`. **Closes partial** — criteria 1,2,3,6,7 met; 4 and 5 re-scoped and carried. **NFR-PERF-01 not closed** | ruling applied 2026-07-31; nothing outstanding on the developers | — |
| 4 | **DC-64** — baseline reconstruction cost (NFR-PERF-01, carried) | Implemented; **closes partial** — the O(operations replayed) full-lineage-replay cost (97.6% of the phase) is eliminated on the warm path, but `load`/`persist`/`from_replay`, each a binding condition of the trust-ladder ruling, remain O(live node count), so Axis A is not fully flat. **NFR-PERF-01 remains missed**, on a lower curve. See the design document §9 | none | — |
| 5 | **DC-65** — text-edit baseline content | **Complete at `250ad54`** — reviewed and accepted 2026-07-31, verified independently at N=6 sealed edits, one non-blocking process note (reported gate commands did not match §6 rule 9; architect re-ran the canonical set, all pass). §1's four questions answered before design (`prerequisite-questions-v1.md`); invariant stated and both `plan_edit_text` and `patch_algebra::baseline_text` conform; a fifth site (DC-64's incremental step) found and fixed once the first three unblocked it; N=4/5 sealed-edit tests at store and CLI level; `ReplaceBinary` confirmed unaffected with a regression test; coverage finding recorded | none | — |
| 6 | **DC-66** — multi-commit queuing | **Complete at `45af36f`** — reviewed and accepted 2026-08-02, all eleven criteria met, one non-blocking note (rollback-draft still rejects on a non-empty WAL, deliberately). Architect independently rebuilt a four-deep queued edit chain from sealed history and got byte-correct content | nothing | — |
| 7 | **DC-57** — active-patch thresholds (NFR-PERF-02) | **Complete at `caa2fc2`** — reviewed and accepted 2026-08-02, no findings. Hard block fires before any write; `>=` semantics documented; config fails closed on all four bad inputs. **NFR-PERF-02 implemented** | nothing | — |
| 8 | **DC-67** — ordinary-use conformance suite | **Complete at `d87a542`** — reviewed and accepted 2026-08-02. **The prediction held**: two findings reported, neither fixed here — (1) `checkout --patch-materialize` cannot replay `ReplaceBinary`/`ChangePerm`, (2) no working-directory branch-switch. Both now unowned | nothing | — |
| 9 | **DC-69** — lifecycle-state retention | **Complete at `9279c08`** — reviewed and accepted 2026-08-03. **Route (c): prikk does not forget, established and measured**, not assumed. `seen_ids` and `latest_tombstone_by_id` answered separately (the latter self-prunes on restoration). §3.3 declined with reasons: a horizon needs two decisions this increment does not own — bounding `rollback-draft`'s reach, and redefining what full replay trusts. No production code | nothing | — |
| 10 | **DC-70** — prebuilt binary distribution | **Complete-partial at `2ca02f9`** — reviewed 2026-08-03; B1 (an unsound `inert_head` widening covering `tar`/`rustc`/`gh`) repaired and re-reviewed. Release workflow, checksums, `binstall` metadata and README install surface delivered. **Criterion 3 carried**: release evidence models one archive, not N, and extending it is blocked behind DC-45's frozen baseline until the 0.19.0 cutover | nothing | — |
| 11 | **DC-71** — non-Linux build conformance | **Complete at `7c898f8`** — reviewed and accepted 2026-08-04. Read-only commands now **execute** on macOS and Windows, CI-verified continuously; mutation stays Linux-only per DC-37. Three blocking findings, all real, three of four found by the CI job this increment created. Closes the release-claim-mismatch row standing since the original architecture review | nothing | — |
| 12 | **DC-72** — path-safety conformance (NFR-SEC-03) | **Complete at `40fdc7e`** — reviewed and accepted 2026-08-04. The §2 table found the recorded gap was wider than described and surfaced a fourth surface (maintainer trust key ids, where a collision could silently reduce a maintainer threshold). Collisions now rejected at creation on three surfaces through one shared fold; reserved names rejected for trust key ids. Architect's §3.5 placement ruling was **wrong in detail and corrected** — the named validators are pure and cannot enumerate | nothing | — |
| 13 | **DC-73** — node-model operation apply | **Complete at `18c5df3`** — reviewed and accepted 2026-08-04, no findings. `ReplaceBinary`/`ChangePerm` now materialize and invert; `CreateFile`'s mode threaded through checkout, closing a silent defect where every file rebuilt as `0600`. Wire format untouched. `RenamePath`/`CreateSymlink` left unimplemented with markers corrected to name the real blocker (no authoring path). **Unblocks three tests in DC-65/DC-67 that worked around the old gap** | nothing | — |
| 14 | **DC-74** — merge execution | **COMPLETE at `3464e2a`** — reviewed and accepted 2026-08-08. Adopts patches verbatim (byte-identity asserted, non-vacuously); refuses cleanly on five live-constructed conflict classes. Architect re-ran all nine gates independently (880/880 both toolchains, 180 packages) and constructed two scenarios the submission did not: an **over-old baseline**, which fails closed by construction, and a **negative control** showing 4 of 5 refusal tests do not pin the confluence gate — benign, since `derive_next_state_root` independently refuses with zero object writes. Two non-blocking findings. **Still release-conditioned**: does not ship until DC-75 discharges the structural-record condition | nothing | — |
| 15 | **DC-75** — merge block lineage / structural merge record | **COMPLETE, merged 2026-08-08** at `c79c421`. Merges seal as `BlockKind::Merge` recording both parents, a mainline pointer, and the baseline; `verify_merge_baseline` **re-derives** rather than trusts it. **Discharged DC-74's release condition**, clearing 0.19.0. Existing object ids proved byte-stable — no hash literal moved | nothing | — |
| 16 | **DC-76** — filesystem durability contract | **COMPLETE at `d568438`** — reviewed and accepted 2026-08-09. Nine guarantees stated as one `DurabilityContract` trait with guarantee-named methods; `LinuxDurability` sole implementor; no behaviour change (892 tests unchanged). **Nine negative controls, two of which could not be cleanly demonstrated and were reported as findings rather than dropped** — architect independently confirmed the G9 kernel behaviour and re-ran G1. **One blocking finding (B1): the first submission broke non-Linux CI; the gate gap that allowed it was the architect's, and rule 9 was amended.** Repaired at `d568438` with a *conditional* dead-code allowance, not a blanket one | nothing | — |
| 17 | **DC-77** — docs Mermaid rendering | **COMPLETE, merged 2026-08-08** at `7e90776`. Diagrams render as pictures; one **exact** `procedure.rs` allowlist entry, proved narrow by the architect against four rejected variants. Vendored assets, offline, no CDN | nothing | — |
| 18 | **DC-78** — history exchange | **STAGE 1 COMPLETE, merged 2026-08-09** at `cba0459` after a **green macOS CI run**. Trust store is an adopted-key **set**, TOFU enforced on conflicting re-add, and adopting a second key no longer invalidates the receiver's own history. **Merged once prematurely and reverted**: the architect merged on local gates alone, and macOS CI then found a security regression — APFS case-folding let a colliding key id skip the DC-72 collision check. Fixed by running that check unconditionally; **verified on macOS, where the bug lived**. **STAGE 2 (§D3) COMPLETE, merged 2026-08-09** at `edbc94c` — `verify` now reports `sealed-block <id>: <key_id>` per block, discharging §D3's reporting gap; architect's negative control confirms the test pins per-block attribution, not mere presence. **Merged after a green macOS run — the standing rule applied before the merge for the first time.** **Remaining: D4/D6 + ruling 4 together** | nothing | `handoffs/DC-78-history-exchange/implementation-handoff-v2.md` + addenda 1-4 |
| 19 | **DC-79** — sha2 + getrandom upgrade | **COMPLETE, merged 2026-08-09** at `7ad0af3`. MSRV holds at 1.85; **no digest moved** — DC-41 vectors and the randomized frozen pre-DC-55 differential both pass unchanged, no hash literal edited. `getrandom` → `fill` rename at three sites, ruled in scope. **Lock 180 → 187**: six transient `digest`-stack duplicates that DC-80 collapses, plus `hybrid-array` permanent. Architect's "cosmetic" framing of that was understated and is corrected | nothing | — |
| 20 | **DC-83** — test temp-dir uniqueness | **COMPLETE at `76c8d18`** — accepted 2026-08-09. **The developer correctly refused the architect's instruction**: `monotonic_suffix` is a wall-clock timestamp despite its name, so "mirror `unique_temp_dir` exactly" would have left the bug in place. Fixed with an `AtomicU64`; 214 collisions → 0 across 128,000 samples | nothing | — |
| 21 | **DC-84** — test helper uniqueness sweep | **COMPLETE at `3825867`** — accepted 2026-08-09. Fifteen helpers (a **recount** against the architect's thirteen), one shared `unique_suffix` for prikk-cli plus an independent fix for prikk-store across a genuine crate boundary, `monotonic_suffix` renamed. 896 tests, two zero-collision demonstrations. **First increment committed to a branch rather than local main** | nothing | — |
| 22 | **DC-86** — bundle decoder hardening | **COMPLETE, merged 2026-08-09** after a green macOS run. Proptest coverage on both untrusted-input decoders; import bounded on object count and total bytes, **refused before any write**. Count check precedes allocation and uses `Vec::new()`, not `with_capacity(count)` — no allocation bomb. Architect disabled both bounds: exactly the three bound tests failed. **Honest scope statement** distinguishing the committed 1,024 cases from a one-off 800,000 campaign | nothing | — |
| 23 | **DC-80** — ed25519-dalek 2→3 | **COMPLETE, merged 2026-08-10** after a green macOS run. `verify_strict` differs by one line (`&[message]`, multipart API); **no verification semantics changed** — architect independently reproduced the cross-version probe, including `S+L` malleability with `high-3-bits-set=false`, rejected by both. Mixed-version history (2 blocks under 2.x, 1 under 3.x) verifies clean. Lock **187 → 179**. **The architect's six-package collapse figure was wrong; measured, only `const-oid` collapses** | nothing | — |
| 24 | **DC-85** — merge from a received ref | **COMPLETE, merged 2026-08-10** at `596edfc` after a green macOS run on the reviewed commit `a8f3f61`. `merge --from remotes/<name>` works, gated by §3A.1's mandatory trust check: every adopted Block must carry a **currently**-trusted MAINTAINER signature, checked over the same candidate-block walk `candidate_patch_ids` already does and before `into_ref` advances. `validate_local_branch_ref` not relaxed; `into_ref` stays local-only. **Architect's negative control disabling the gate: the merge did not merely proceed, it completed** — `heads/main` advanced to a Merge block adopting content sealed by a never-trusted key, so DC-78 Stage 3's gap was fully exploitable. Induction exempting local merges re-derived independently across every `RefStore::publish` call site. **The architect's review claimed no existing doc statement had become false; `merge-plan.md:24` had, and the developer found and corrected it** | nothing | — |
| 25 | **DC-81** — macOS mutation | **COMPLETE, merged 2026-08-09** after a green macOS CI run. `MacosDurability` implements the contract; **G3 uses `fcntl_fullfsync`** — measured **180x slower than `fsync`** on the runner, recorded in `FINDINGS.md`. Four platform differences found by running, all in **test fixtures** rather than production primitives. **Two architect errors this cycle** — a premature merge to main, and a docs commit landing on the developer's branch; both corrected | nothing | — |
| 26 | **DC-82** — mutation dispatch collapse | **COMPLETE, merged 2026-08-09** after green macOS CI. `NoDurability` as a third implementor; one gated `ACTIVE_DURABILITY` constant; **all eleven mutation call sites unconditional**; `anchored.rs` 43 → 14 gate attributes, each now scaling with **platform count** rather than call-site count. **Criterion 3 (whole-tree single digits) not met — the target was the architect's to miscalibrate**; the sub-contract layer is per-platform types and primitives, deferred to the Windows increment | nothing | — |
| 27 | **DC-87** — Windows mutation | **ACCEPTED 2026-08-10.** The other half of the owner's cross-platform mutation priority, and the increment **DC-82's unmet criterion 3 deferred by name**. Windows resolves to `NoDurability` today; read-only is already CI-gated, and **DC-72 already rejects the Windows-hostile path forms cross-platform**, so what remains is genuinely the durability backend. The obstacle is that `MutationRoot` holds a `rustix` fd and the whole primitive layer is Unix-shaped — `openat` has no Win32 equivalent, and G1 is defined in terms of it. **Two stages** (seam-neutral refactor, then the backend), per DC-82's own split rationale. **Six blocking prerequisites precede design**, the sharpest being whether `durable_directory_entry` (G3) is implementable on NTFS at all and what that does to DC-38's crash-recovery reasoning. **Round 1 answered and ruled 2026-08-10.** `FlushFileBuffers` does not apply to directories and `REPLACEFILE_WRITE_THROUGH` is documented "not supported", so **DC-38's "format-2 publication never permits an ahead log" does not carry to Windows** — though DC-38's own format-1 clause already defines a bounded recovery for that state, which is a security question, not a portability one. **Three architect corrections**: `ReplaceFileW` is not atomic (three documented partial-completion error codes); `MOVEFILE_WRITE_THROUGH` was unexamined and is now the decisive open question; and G9's framing was wrong — prikk records only the execute bit, and `read.rs`'s `mode = 0` sentinel would silently drop it from sealed history. **`#![forbid(unsafe_code)]` blocks every raw Windows syscall from `prikk-store`** — whether prikk gains its first `unsafe` surface is **escalated to the owner**. **Mode-carrying fix MERGED 2026-08-10** at `d2fcac0` after a green three-platform CI run on `1e10a09` — the `read.rs` `mode = 0` sentinel is gone, `RootFileStat`/`WorktreeFileMeta` carry `Option<u32>`, and an unobservable mode now declines to plan `ChangePerm` instead of silently stripping the executable bit from sealed history; architect's negative control (`unwrap_or(REGULAR_FILE_MODE)`) failed exactly the one test that models the hazard. **Stage 2 BLOCKED 2026-08-11 on an owner scope decision, and Stage 1 held with it.** The transition-durability investigation confirmed DC-38's invariant cannot hold on Windows: step 6's log append is an existing-file content append and *is* achievable, while step 5's pointer promotion needs transition durability and is *not* — the asymmetry reproduces the exact ahead-log state DC-38 exists to eliminate. **The sequencing call paid off**: had Stage 1 gone first it would have built a seam for an implementor that cannot exist. Three options escalated (restructure publication / ship a weaker invariant / do not ship Windows mutation), with **DC-91** proposed to ask the question that decides between them. **Sequencing ruled 2026-08-11: the Stage 2 transition-durability investigation goes first, Stage 1's seam after it** — the seam exists only to house a Windows authority type, so building it before knowing whether Windows can satisfy `atomic_replace`/`promote`/`durable_append` is speculative. Same principle the architect misapplied against DC-88 (which changed no type) and now applies to the question that genuinely can change what the type must do **Narrow round 2 ruled 2026-08-10:** `MOVEFILE_WRITE_THROUGH` could not be settled from primary sources after three corroboration attempts, and **the architect withdrew the question as the wrong blocker** — `durable_directory_entry` is the one `DurabilityContract` method named after its primitive rather than its requirement, so "Windows cannot fsync a directory" never entailed "Windows cannot publish a ref durably." Stop-and-report fired; **Stage 2 now blocks on DC-88, not on a Windows API fact**. Mode-carrying shape accepted (sentinel `0` becomes `Option`), plus a doc defect the architect found at `commit_index.rs:4` claiming mode is part of a cache-trust condition the code does not check | owner decision on the `unsafe` surface | `handoffs/DC-87-windows-mutation/narrow-round-ruling-v1.md` |
| 29 | **DC-89** — platform claim documentation accuracy | **ACCEPTED 2026-08-10 — small, mechanical, cleared to implement directly.** Since DC-81 merged, the reference docs have told readers mutation is Linux-only, in two distinct false forms — a capability claim (`architecture.md:106`/`:132`, `durability-recovery.md:19`) and an evidence claim ("exercised by project gates on Linux only", six more pages), the latter also false because DC-81 added the macOS mutation CI job. **Most of the incorrect text is the architect's own.** Arises from the DC-87 mode-shape review, where **the architect's accept condition named one file and the claim turned out to live in eight places across seven more** — the second time this cycle the architect checked the file in front of them and generalized. **Criterion 3 requires the affected set be derived independently, not worked from the architect's list** — and it worked: the developer found a twelfth site (`durability-recovery.md:91`) the RFC missed, and flagged that the RFC's prose count ("eight") contradicted its own table (eleven). Seven-file correction **accepted at `b0a66ea`**; **criterion 1 amended by the architect** to cover all user-facing documentation, because it had wrongly excluded `README.md` — the third narrow-scoping error in this chain. **COMPLETE, merged 2026-08-10** at `f8b812d` after green CI. README amendment delivered at `35b965f` (`:105`/`:138` left alone as true prebuilt-binary claims, per criterion 5). **Two stale `ci.yml` comments (`:48`, `:92`) reported and deliberately left** — no risk, so no `FINDINGS.md` row; they go to DC-87 Stage 2, which must touch that file anyway | nothing | `handoffs/DC-89-platform-claim-docs-accuracy/implementation-review-v1-amendment-accepted.md` |
| 31 | **DC-91** — publication record shape | **PROPOSED 2026-08-11 — an evaluation, not a commitment.** One question: does a fixed-name, slot-based durable publication record have **independent value on POSIX** (fewer reachable crash states, less dependence on directory-sync ordering, self-describing recovery), or is it purely a Windows tax? If the former, DC-87's option 1 stops being a platform cost and becomes an improvement that also unblocks one; if the latter, Windows mutation is a separate and cheaper decision. **A "no" is a complete, successful outcome** and settles an open question either way. Touches the most safety-critical machinery in the product, so §4.4 asks for blast radius in terms of what would have to be re-proved | owner acceptance | — |
| 30 | **DC-90** — unsafe code boundary and gate | **ACCEPTED 2026-08-10.** Turns the owner's ruling (*"`unsafe` is allowed under control with safety and maintainability preserved"*) into a checked property rather than an intention, in the shape DC-51's placement gate already established: one crate may omit `forbid(unsafe_code)`, named in an allowlist, unable to depend on any product crate, with its own third-party allowlist so the exception is not a side-door around DC-51. **Must land before the first `unsafe` line** — a boundary added afterwards documents what happened instead of constraining it, the same ordering argument DC-82 used against DC-81. Does not block DC-88 and is not blocked by it. **Architect's §4.2 baseline measurement, with its own false start recorded**: a first grep for `lints.workspace` returned zero and read as "the lint table is inert" — wrong; the real form is `[lints]` + `workspace = true`, and **all eight members carry it**. One real asymmetry surfaced: `prikk-crypto/src/lib.rs` has no source-level `#![forbid]` and is covered by manifest inheritance alone, so manifest and source disagree in presentation while agreeing in effect — which of the two the gate treats as authoritative is a genuine design decision. **Prerequisites answered and ruled 2026-08-10.** Headline: the expensive half needs no building — `clippy::undocumented_unsafe_blocks` already enforces the SAFETY-comment rule, found by testing rather than reasoning. **Architect's correction**: the gate set runs clippy but **not this lint** — it is `restriction`-group, allow-by-default, and `-D warnings` does not reach it (probed independently, zero matches). **Consequence nobody had drawn**: enabling it in the exception crate's own manifest would let the one crate permitted to write `unsafe` switch off its own guard with a one-line edit. Ruled: the lint goes in the root `[workspace.lints.clippy]` table, and **dropping workspace lint inheritance is itself a gate failure** — that is what makes the arrangement self-guarding. Manifest is authoritative; `prikk-crypto`'s missing source attribute needs no change. **Implemented at `baa4b38` and reviewed 2026-08-10 — accepted on one condition**: the lint was set to `"deny"`, which a source-level `#![allow(...)]` silently overrides, so the exempt crate could satisfy the new gate and then switch the guard off in its own `lib.rs` with every gate still passing. **Architect built the bypass and its fix**: manifest `deny` + source `allow` → lint never fires, clippy exits 0; manifest `forbid` + source `allow` → `error[E0453] ... incompatible with previous forbid`. The increment already knew the principle — it is why `unsafe_code` is `forbid` — and applied it to the guarded thing but not the guard. **Fixed at `f358353` and accepted**: the escape route is now a hard `E0453` compile error, which the architect verified **end to end on a real member crate** rather than on the isolated probe, and reverting the constant to `"deny"` fails eight tests. **COMPLETE, merged 2026-08-11** at `2bad097` after green CI. The escape route is a hard `E0453` on merged `main`, verified end to end against a real member crate | nothing | `handoffs/DC-90-unsafe-code-boundary-gate/implementation-review-v1-condition-accepted.md` |
| 28 | **DC-88** — durability contract requirement shape | **ACCEPTED 2026-08-10, with the scope trade taken as stated — DC-87 Stage 2 waits for this.** Does `durable_directory_entry` state a requirement or a primitive? DC-76's own thesis is "guarantee, not syscall"; this is the one method that misses its own bar, and Windows is where it bites. **Arises from the architect's own error** in blocking DC-87 Stage 2 on an unanswerable Windows API question instead of on prikk's contract. Accepting it blocks DC-87 Stage 2 until it lands; the alternative was shipping Windows with a documented weaker crash invariant; the owner accepted the slower, stronger path. **Prerequisites answered and ruled 2026-08-10, and the result collapsed the increment**: `durable_directory_entry` has exactly two callers (both `worktree.rs`, both wanting per-entry confirmation), and **DC-38 never calls it** — its three durability-bearing steps go to `atomic_replace`/`promote`/`durable_append`, each of which bundles its own directory sync. **COMPLETE, merged 2026-08-11** at `fe6d5e0` after a green three-platform run on `ed04c21` — a parameter restatement plus two one-line caller edits; architect's negative control (reverting the implementor) fails **five** tests, four of them pre-existing caller-level ones, and a repository-root-file probe confirmed the `required_parent` change is behaviour-identical to the old `unwrap_or(Path::new(""))`. **Two architect corrections follow**: the scope trade sold to the owner ("this blocks Stage 2") was **mispriced on a false premise** — the two are orthogonal, so Stage 2 is no longer blocked on this and **Stage 1's seam is released from hold**; and the Windows blocker is real but **located in the three transition methods, not here** — with the contract already permitting any implementation of them, so no amendment is needed to attempt it. §3's two-slot sketch reassigned to Stage 2 | nothing — **cleared to implement** | `handoffs/DC-88-durability-contract-requirement-shape/prerequisite-ruling-v1.md` |

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
| **DC-43** — release security and distribution controls | Its scope *is* release security and distribution, and `DC-35:255-257` hands it key custody, rotation, expiry/revocation monitoring, attestations, and SBOMs. DC-35 needs a fitness amendment, so designing DC-43 now designs against a foundation about to change. **Moved here 2026-07-30.** | `handoffs/DC-43-release-security-controls/implementation-handoff-v1.md` |

**DC-52 left this section 2026-08-08** — `DC-45:419`'s condition is discharged by 0.19.0 and its
accepted stability rerun. It is available now; deletion is still a separate architect-reviewed change.

**DC-74 does not belong here and must not be moved here.** It carries a *release condition* — buildable
and mergeable now, not releasable until a named condition holds. That is the inverse of this section,
which lists increments blocked *from being built*. See `MILESTONES.md`, "Attached release conditions".

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

   **Additionally, for any increment touching `#[cfg(target_os)]`-gated code** — added 2026-08-09 after
   DC-76 broke the non-Linux CI job while passing all nine gates above:
   `cargo clippy --workspace --all-targets --all-features --locked --target x86_64-pc-windows-gnu -- -D warnings`
   and the same with `--target x86_64-apple-darwin`. CI's `non-linux build` job runs this natively on both
   platforms, so an increment that skips it can pass every canonical gate and still turn CI red. The nine
   above cannot see platform-conditional dead code, which is the exact failure DC-71 exists to prevent.
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
