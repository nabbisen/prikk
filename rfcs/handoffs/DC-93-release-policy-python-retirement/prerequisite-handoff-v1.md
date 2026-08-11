# DC-93 Release Policy Python Retirement — Prerequisite Handoff v1

**Cleared to answer §3's four questions only.** Accepted 2026-08-11,
`rfcs/accepted/DC-93-RELEASE-POLICY-PYTHON-RETIREMENT.md`. **Delete nothing yet.**

## 1. Why this exists, and why it is narrower than DC-52 was

DC-45 made the Rust command authoritative on 2026-07-21 and retained the Python as a rollback path.
Three weeks on, **nothing invokes it** and the tree still carries 18 files / 2,895 lines.

DC-52 covered this, was handed off in advance on 2026-07-28, and then never got a row in
`EXECUTION-ORDER.md` — so it fell out of the only view that answers "what next." Its analysis was sound
and is archived intact (`rfcs/archive/`, plus `handoffs/DC-52-python-oracle-decommissioning/`); **read
it, it is still the best account of the obligations.** What changed is the shape: DC-52 made two *added*
checks preconditions for the *removal*, so an increment meant to reduce complexity read as one that
increased it. Those checks are now DC-94, and they gate nothing.

**The cost worth removing is not only the unused Python.** The authoritative Rust tool carries a
Python-recognition path of its own — `command_scan.rs:124-149` (`is_python`, `python_policy`,
`has_python_policy_target`, `has_python_interpreter`), matching `release/check-policy.py` by exact path,
plus four accepted invocation spellings in `release-policy-command-inventory-v1.json`. A
security-sensitive command scanner is Python-aware because the Python exists. That goes with it.

## 2. Where to start, and the trap in it

**§3.1 first, and take it seriously — this is not a pure deletion and I said so in the RFC.**

Five of the 18 files live under `release/oracle/` and are **oracle tooling**, not the policy engine:
`generate-manifest.py`, `verify-manifest.py`, `manifest_verify.py`, `manifest_self_test.py`,
`coverage_contract.py`. The Rust tool **reads** `release/oracle/oracle-manifest-v1.json` directly
(`differential.rs:58`).

So the question that decides this increment's scope: **is that manifest frozen, or regenerable?** If any
of those five is still the only way to produce or verify material the Rust tool consumes, **it is not in
scope and the increment shrinks around it.** Say so plainly — a smaller correct retirement beats a
larger one that removes a generator nobody can replace.

**Deleting a generator is not deleting what it generated.** The contract data stays regardless; that is
what makes the equivalence argument easy (§4.3 — same gates, same data, before and after).

## 3. The rest

**§3.2 — the `command_scan.rs` Python path.** It has tests and an inventory contract behind it. Report
what changes in `release-policy-command-inventory-v1.json`, what oracle cases move, and whether any
case exists *only* to exercise the Python-invocation branch. A case that becomes vacuous should be
removed, not left passing trivially.

**§3.3 — the parked release lane.** Confirm nothing there depends on the Python. Confirm, do not
assume; the lane being parked is exactly why nobody would notice.

**§3.4 — documentation.** `release/README.md` documents `python3 -B release/observe-policy.py`;
`tools/release-policy/README.md` also references it. Enumerate **every** user-facing statement that
becomes false — and derive the set yourself rather than from my two examples. That instruction is
DC-89's, and it earned its place there.

## 4. Limits

- **No deletion in this pass.** Answers first, then a removal plan, then removal.
- **Exhaustive disposition** when it comes: all 18 files individually removed or retained with a stated
  reason. None silently kept. That standard is DC-52's and it was right.
- **No change to the authoritative command, the public policy schema, the oracle's semantics, or the
  frozen contract data.**
- **No release-lane, signer, tag, or publication action.**
- **Nothing from DC-94.** It is independent and does not gate this.

## 5. Reporting

`.git-exclude/review-request/`, plain `.md`. Answer §3 in order. Findings outside scope go in the
report; I register them in `FINDINGS.md`.

## 6. Sequencing

- **DC-94 is accepted and independent** — either order, no collision expected, but they touch
  `tools/release-policy` from different directions so do not run them in one branch.
- Touches no product code. The three-platform rule does not bind it; an ordinary CI run is enough.
