# Amendment to `changelog-history-gate-handoff-v1.md` — the §5 ruling, and one self-guard

**v1 stands in full. `1edde4c` stays as it is — nothing in it is reverted, and it is pushed with this
amendment.**
**Architect review of `.git-exclude/review-request/changelog-history-gate-report-v1.md`, 2026-09-01.**

---

## 1. What I verified independently

**Everything held, and I re-derived rather than read.**

- **The restoration.** `## 0.23.0 — 2026-08-23` sits immediately above its own body, and I diffed the
  tag's own lines 4–110 (`git show b6cd309:CHANGELOG.md`) against today's corresponding range:
  **byte-identical**. Nothing beneath the heading moved.
- **The gate fails on the real defect.** In a detached worktree at `1edde4c` I removed the restored
  heading and ran `boundary-check`:
  `"0.23.0: 0 \`## 0.23.0 — DATE\` heading(s) in CHANGELOG.md (expected exactly 1)"`. Restored it;
  clean. **Your control, re-run by me rather than trusted.**
- **The CI job selection is exactly right.** I enumerated every job in `ci.yml` and cross-checked
  which run `cargo test --workspace` or a `release-policy` subcommand: `stable`, `msrv-1.85.0`,
  `macos-mutation`, `windows-mutation` — **four, all four given `fetch-tags: true`, and no other job
  touched.** `fetch-tags: true` over `fetch-depth: 0` is the right call and the reasoning is at the
  site.
- Gates re-run here: fmt clean, clippy single invocation exit 0 / 0 warnings, **1442/1442**,
  `git diff --check` clean, 57/57, boundary and reference `valid: true`.

**And control 4 was answered correctly.** You cannot produce CI results from a session that never
pushes, and you said so plainly instead of implying otherwise — then did the closest real thing
(`git clone --no-tags`, confirming the empty-tag precondition CI actually has). **v1's control 4 was
my error, not a gap in your work**: CI verification is the architect's control, because only the
architect pushes. I have corrected that in my own practice.

## 2. §5 — ruled: **A, the exemption list, with one addition**

**Your escalation was right and the finding is mine to own.** RFC 127 and v1 both said "every other
released tag has exactly one heading." **I checked eight of forty-four.** Confirmed here: 44 tags;
`0.0.1` has no heading of any shape; `0.1.1` reads `## 0.1.1 Housekeeping`. **A range-bound claim
stated as exhaustive is the error this project has caught twice in sweeps this week**, and stopping
rather than quietly picking B or C was the correct call.

**Ruling: keep the exemption list.** RFC 127 §3.3 now carries the reasoning; in short — the gate
exists to catch a regression *against* the convention, and these two tags were never in its shape, so
there is nothing to regress; **A is this project's idiom three times over** (`UNSAFE_EXEMPT_CRATES`,
`DECLARED_UNDOCUMENTED`, `RFC114_ADMITTED_BUT_UNWRITTEN`), and **RFC 130 §4.1 ruled the same shape
for the coupling gate two hours earlier**; B writes a date nobody recorded and deletes a word from a
released heading; C asserts a cutoff the RFC never established.

## 3. Required — make the exemption self-guarding

**One change, and it is the only work in this amendment.**

`LEGACY_TAGS_WITHOUT_DATED_HEADINGS` currently means *"do not check these two."* That goes stale
silently: **if `0.0.1` ever gained a conforming heading, the gate would keep skipping it and the entry
would quietly mean nothing.**

**Verify that each exempt tag genuinely lacks a conforming heading, and fail if one does not.** An
exemption that has become untrue must break the build until someone deletes it — this project's own
standard, stated at `unsafe_boundary.rs`: *"a control the controlled party can silently remove is a
convention, not a control."*

The error text should say what it means — that the tag now has a heading and its exemption should be
removed — rather than reusing the missing-heading wording, since the two situations call for opposite
fixes.

**Keep the existing pin test** (`legacy_exemption_list_is_exactly_the_two_named_pre_convention_tags`);
add one for the new direction, so the self-guard is itself shown to fire.

**Update the constant's doc comment.** It currently says the list "should be emptied once one is
chosen" — a ruling has now been made *for* the list, so that sentence is stale. It should say the
list is the ruled outcome (RFC 127 §3.3), that entries are pre-convention tags only, and that the
gate verifies each entry is still true.

## 4. Not required, recorded

**`0.1.2`'s heading is `## 0.1.2 — DC-09 Phase 4.3 / 4.4 internal node-model groundwork`.** It
satisfies the `## X.Y.Z — ` shape and passes, but the suffix is prose, not a date. **Do not "fix"
this.** The gate deliberately matches `release_notes::changelog_section`'s own pattern and the two
must agree about what a heading is; tightening one alone would be worse than the imprecision. Noted
in RFC 127 §3.3 so nobody later assumes the suffix parses as a date.

Your §5 said the convention "starts holding consistently around `0.1.2`" — shape-wise yes,
semantically not until later. Immaterial to the ruling.

## 5. Controls

1. **The self-guard shown failing**: give an exempt tag a conforming heading in a scratch tree, run
   the gate, show it refuse and name the stale entry; remove the heading, show it pass.
2. **The pin test still passes**, and the new one is shown red before green.
3. **The full gate set** against your final commit, clippy as a single invocation with the exit code
   captured explicitly. Cross-target clippy judged from your own diff.

**No CI control.** That is mine at push time, and v1 should not have asked you for it.

One commit on `main`, local, **no push, no tag**. **RFC 127 closes when this lands.**
