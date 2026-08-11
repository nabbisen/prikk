# DC-93 — Prerequisite Investigation Ruling v1

**Reviewing:** `.git-exclude/review-request/prikk-dc-93-prerequisite-questions-v1.md`.

**Accepted, and it corrects the RFC's central premise. DC-93 shrinks from "18 files" to at most 5.**
§1 is my error. §2 rules the dispositions they surfaced rather than decided.

## 1. My error, and its exact shape

DC-93 §1 states: *"nothing invokes it — grep-confirmed across `.github/workflows/` and `scripts/`."*

**Two things are wrong with that, and the second is worse than the first.**

**`differential.rs:71-72` spawns `python3 -B release/observe-policy.py` as a subprocess** — a live,
documented code path inside the authoritative Rust tool, reachable via `prikk-release-policy
differential-check`. I grepped CI and never grepped the tool's own source, which is the one place a Rust
tool would invoke Python.

**And `scripts/` does not exist in this tree.** Half my grep targeted a path that isn't there, returned
nothing, and I read the empty result as evidence. A grep over a nonexistent directory cannot disconfirm
anything, and I did not check that it existed before relying on it.

**This is the third time this cycle my search scope has been too narrow** — DC-89's "eight occurrences"
against eleven, and "nothing there is now false" about `merge.md` while `merge-plan.md:24` was. The new
element is worse: previously I searched a real but incomplete set; here I searched a set that was partly
imaginary. **Absence of hits is not evidence unless the search space is verified to exist and to be the
right one.**

Their framing — that my claim "is accurate for exactly what it grepped, but that scope doesn't include
the Rust tool's own source" — is more generous than the error deserves.

## 2. Dispositions they surfaced and did not decide

**2.1 — `generate-manifest.py`: retain. Confirmed, as they asked.** They read my handoff's §3.1 as
settling this and asked me to confirm or overrule. **Confirmed.** It is the sole generator of
`oracle-manifest-v1.json`, the Rust tool has no generation capability (verified: `oracle/` reads and
validates, nothing writes), and the manifest is re-freezable under review per §6 rule 4 — not inert.

**Their reasoning for why "git is the rollback path" does not transfer here is correct and worth
keeping**: that argument is about falling back to a working old implementation if the new one proves
wrong. It is not about permanently losing the only means of producing a class of data nothing else can
produce. Different thing, and I would not have drawn the line as cleanly.

I verified the consequence: `generate-manifest.py` imports `policy_check.observation`, `.runner`,
`.evidence`, `.common`, and `observation`'s closure covers the rest. **The 13 retained files are a traced
dependency graph, not caution.**

**2.2 — `differential-check` and `observe-policy.py`: retire both.** They recommended it and correctly
declined to decide it under a no-deletion limit.

**Ruling: retire.** `differential-check`'s only purpose was proving Rust/Python agreement across the
DC-45 migration; the cutover completed 2026-07-21 and its post-release stability rerun was accepted
2026-08-08. Keeping a subcommand whose sole function is comparing against an implementation being retired
is keeping the reason to keep the implementation. It is a release-policy tooling subcommand — not the
product CLI, not the gate set, not the release lane — so this is an architect call; the owner may
overrule.

**It must be stated as a removal, not slipped in**: `differential-check` disappears from
`tools/release-policy/README.md` and from the tool's own help.

**2.3 — The three manifest-verification files: do not remove on "likely."** They flagged
`verify-manifest.py`, `manifest_verify.py` and `manifest_self_test.py` as probably redundant with
`oracle/verify.rs` + `oracle/self_test`, explicitly noting the line-level comparison was not done.
**Require that comparison before removal.** If any check exists only in the Python, it either moves to
Rust in its own increment or the file stays. "Likely redundant" is how a check gets lost.

## 3. My drafting error, which they caught

DC-93's status line claims it "Supersedes DC-52's obligations 3 and 4" — but §3's prerequisites never
ask about obligation 4, the eight frozen contract/evidence JSON files. They flagged the mismatch and
declined to expand scope into it.

**They are right, and the supersession line was too broad.** Obligation 4 is a disposition question about
JSON contract data — a different subject from retiring Python, with different consequences (those files
are in `boundary/package.rs`'s allowlist). Folding it in would be exactly the bundling that split DC-52
in the first place.

**Amended: DC-93 supersedes DC-52's obligation 3 only.** Obligation 4 remains open and unowned, recorded
in `FINDINGS.md`.

## 4. What they got right that I would not have asked for

**§3.4's distinction between statements that become false and a historical record's past-tense
accuracy.** They corrected `release/README.md` and `tools/release-policy/README.md` as operational, and
deliberately left `MILESTONES.md` and `rfcs/IMPLEMENTATION-STATUS.md` alone because those record what was
true in July. **That is DC-89's actual standard, applied with judgement rather than mechanically** — and
DC-89's own review said as much about documents written before their date.

**And they found a pre-existing falsehood I had not:** `tools/release-policy/README.md:4` says *"Python
remains authoritative until the separately reviewed DC-45 cutover."* The cutover completed 2026-07-21.
That line has been false for three weeks, independent of DC-93. Verified. It should be corrected in this
increment since the increment is editing that file anyway — the proximity standard from DC-87.

**§3.2's coupling analysis is the other thing I underestimated.** Removing the Python-recognition path
touches `command_scan/procedure.rs`, which §6 rule 5 designates a reviewed policy artifact rather than
refactorable code, and intersects `reference.rs`'s deliberate dual-candidate authority tests. I called it
"a whole recognition path"; it is that plus a policy-change review plus a design question about whether
the dual-candidate test pattern survives in synthetic form. **That design question is theirs to raise and
mine to answer later — do not decide it inside the removal.**

## 5. Cleared, with the scope as it now stands

**Cleared to plan the removal**, covering: `check-policy.py` unconditionally; `observe-policy.py` with
`differential-check` per §2.2; the three manifest-verification files only after §2.3's comparison; the
`command_scan.rs` recognition path with `procedure.rs` treated as a policy change; the four dead
inventory spellings; and §3.4's documentation corrections including the pre-existing `README.md:4` line.

**Exhaustive disposition still applies** — all 18 files individually removed or retained with a stated
reason, which their table already provides.

**Acceptance criterion 3 is unchanged and matters more now**: all three release-policy gates and the full
oracle case set pass at every step. With 13 files retained, "the contract does not move" is even more
literally true than when I wrote it.

## 6. Standing

- **DC-93: cleared**, at roughly a quarter of its drafted size.
- Touches no product code; an ordinary CI run suffices.
- DC-94 and DC-95's reports are ruled separately.
