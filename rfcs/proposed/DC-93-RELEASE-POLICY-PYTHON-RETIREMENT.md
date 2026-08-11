# RFC (proposed) - DC-93 Release Policy Python Retirement

**Status.** **PROPOSED** — needs the project owner's acceptance.
**Independence.** Author-reviewed — the standing ceiling.
**Arises from.** The owner's question of 2026-08-11 — *"I care about increased complexity of the project
which can be possibly avoided"* — and the audit it prompted, which found DC-52 drafted, handed off in
advance on 2026-07-28, and then absent from `EXECUTION-ORDER.md` entirely.
**Supersedes** DC-52's obligations 3 and 4. **Does not** supersede its obligations 1 and 2, which move
to **DC-94** and stop gating this increment. See §2.
**Target milestone.** M2. No release-lane action; the lane stays parked.

## 1. What this removes, and why it is complexity worth removing

DC-45 made the Rust `prikk-release-policy` command authoritative on 2026-07-21 and deliberately retained
the Python implementation and frozen oracle as a rollback path. Three weeks later the retained material
is still present and **nothing invokes it** — grep-confirmed across `.github/workflows/` and `scripts/`.

Measured, not estimated:

- **18 Python files, 2,895 lines** under `release/` — `check-policy.py`, `observe-policy.py`, the
  nine-module `policy_check/` package, and five `oracle/` manifest tools.
- **The authoritative Rust tool carries a Python-recognition path of its own.**
  `command_scan.rs:124-149` implements `is_python`, `python_policy`, `has_python_policy_target` and
  `has_python_interpreter`, matching `release/check-policy.py` by exact path, and
  `release-policy-command-inventory-v1.json` enumerates four accepted spellings of the Python
  invocation.

**That second item is the argument.** The cost is not only unused Python; it is that a
**security-sensitive command scanner** carries dedicated logic to recognise Python interpreter
invocations as a governed procedure, solely because the Python exists. Retiring the implementation
retires that recognition path with it.

**The rollback objection dissolves on inspection.** The Python is not executed, so it catches nothing;
it is available to fall back to, and reverting governance to an unmaintained second implementation
weeks after cutover is not a realistic operation. More directly: **git is the rollback path.** Removing
these files removes them from the working tree, not from history. What retention actually buys is
ongoing maintenance and review surface.

## 2. Why this is not DC-52

DC-52 bundled *subtraction* (retire the Python, rule on the frozen files) with *addition* (bind the
responsibility map, add a `defaults.run` validator) and made the additions **preconditions** for the
subtraction. An increment whose purpose is reducing complexity therefore read as one that increases it.

That coupling was DC-45's policy — *do not drop the cross-check until the remaining implementation is
self-checking* — and it was sound at cutover. It is also exactly the bundling this project has rejected
five times since (DC-82 out of DC-81, DC-86 out of DC-78, §3.6 and DC-89 and DC-91 out of DC-87):
different work needs different proofs, and bundled, a reviewer cannot tell which half a failure came
from.

**DC-93 is the subtraction. DC-94 is the addition, on its own merits, gating nothing.**

## 3. Blocking prerequisites

**This is not as pure a deletion as §1 makes it sound, and §3.1 is why.**

1. **Classify all 18 files, individually.** `release/oracle/`'s five — `generate-manifest.py`,
   `verify-manifest.py`, `manifest_verify.py`, `manifest_self_test.py`, `coverage_contract.py` — are
   *oracle tooling*, not the policy engine. The Rust tool **reads** `release/oracle/oracle-manifest-v1.json`
   (`differential.rs:58`). **Establish whether that manifest is frozen or regenerable**, and therefore
   whether deleting its generator loses the ability to reproduce it. If any file is still the only way
   to produce or verify material the Rust tool consumes, **say so** — that file is not in scope and the
   increment shrinks around it.
2. **What does removing the `command_scan.rs` Python path cost?** It has tests and an inventory
   contract behind it. Report what changes in `release-policy-command-inventory-v1.json`, what oracle
   cases move, and whether any case exists *only* to exercise the Python-invocation branch.
3. **Does anything in the parked release lane depend on it?** The lane is parked and this increment must
   not disturb it. Confirm rather than assume.
4. **Documentation.** `release/README.md` documents `python3 -B release/observe-policy.py`;
   `tools/release-policy/README.md` also references the Python. Enumerate every user-facing statement
   that becomes false.

## 4. Acceptance criteria

1. §3 answered and reported before removal.
2. **Every one of the 18 files is individually dispositioned** — removed, or retained with a stated
   reason. Exhaustive; none silently kept. This is DC-52's obligation 3 standard and it was right.
3. **All three release-policy gates and the full oracle case set pass at every step** —
   `check`, `boundary-check`, `reference-check`. The contract does not move, so equivalence is not
   argued, it is demonstrated by the same gates on the same data.
4. **The Python-recognition path is gone from `command_scan.rs`**, or a reported reason it must stay.
5. **No user-facing statement is left false** — §3.4's enumeration corrected in the same increment,
   per the standard DC-89 established.
6. **The rollback consequence is stated explicitly** in the commit and in
   `docs/src/reference/release-compatibility.md` if that page's claims change: what is no longer
   available in the working tree, and that history retains it.
7. Gate set per `EXECUTION-ORDER.md` §6 rule 9.

## 5. Non-goals

- **The responsibility-map binding and the `defaults.run` validator.** DC-94. Neither gates this.
- **Any change to the authoritative command** — Rust remains authoritative; this removes what it
  replaced.
- **Any change to the public policy schema, the oracle's semantics, or the frozen contract data.**
  Deleting a generator is not deleting what it generated.
- **Any signer, release-lane, tag, or publication action.**
- **Rewriting DC-52.** It is superseded in part and its handoff (`rfcs/handoffs/DC-52-python-oracle-decommissioning/`)
  is retained as the record of what was prepared.
