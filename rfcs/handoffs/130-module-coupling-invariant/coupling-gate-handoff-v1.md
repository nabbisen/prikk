# RFC 130 — the module coupling invariant, and the gate that holds it

**RFC:** `rfcs/done/130-module-coupling-invariant.md` — accepted by the project owner
2026-09-01, including §4's amendment (allowlist-with-reasons, **not** a bare degree bound), §5, §6
and §7's ordering.
**Base:** `main` at `9184bf2`.

> **SUPERSEDED 2026-09-06 — DO NOT WORK FROM THIS.** It instructs building §4.1's *absolute* acyclicity rule. RFC 130 §4a found four cycles rather than one, and that the absolute rule would have rejected RFC 122 — the same commit §4 uses to prove the degree bound wrong. **§4b ruled 2026-09-06: acyclicity becomes an allowlist-with-reasons.** A replacement handoff will be issued; §1's no-external-artifact reasoning and §2's re-measure instruction survive unchanged.

**Read §1 before planning: the RFC's §8 waits on an external artifact that has not arrived, and the
work does not need it. §2 is why re-deriving is better than inheriting.**

---

## 1. The external artifact never came, and that changes the approach for the better

RFC 130 §8 records that the external architect offered *"the coupling-gate script and the full
68-node edge list"*, and that taking the offer is cheaper than rebuilding. **It was requested on
2026-09-01** (`.git-exclude/tasks/architect/020-20260901-02-coupling-gate-artifact-request.md`) **and
nothing has arrived in the four days since.** Do not wait for it.

**Rebuilding is not merely the fallback — it is the better outcome**, and §4.2 item 3 already says
why: two independent extractions differed ~2% on import-form handling (`use crate::{a, b}` grouping,
re-exports, `crate::x::y` paths). **If the allowlist is derived from a *different* extractor than the
gate uses, the two drift apart silently and the gate begins enforcing a set nobody can reproduce.**

**So: the gate's own edge definition is authoritative, and every number in the allowlist is derived
from it.** State the definition, test it, and generate the declared set with it.

## 2. Do not copy §2's table — re-measure

§2's numbers are from **`04e9391`, 2026-09-01**. Since then RFC 122 moved two hubs (`patch_replay`
went 12/6 → 13/8 by the RFC's own §4 diff), RFC 123 added a decode function, and 0.32.0 shipped.
**The four hubs §2 names may not be today's four**, and the threshold §4.2 asks you to derive depends
on today's distribution.

Re-measure at your own base commit, with your own extractor, and **write the derivation down**. If
your measured graph disagrees with §2 on any conclusion — not just on a count — **stop and report**,
because §2 is the evidence the whole RFC rests on.

## 3. What the gate enforces

**3.1 Acyclicity, absolutely, after one grandfathered entry.** `active ↔ refs` is declared with its
reason. **A second cycle fails the build, full stop** — no threshold, no allowance. §4.1 is explicit
that the external review's version is right here and needs no amendment.

**3.2 Middle-hubs: declared, not bounded.** A module with both high fan-in and high fan-out. Today's
set is declared with reasons; a module newly crossing the threshold **fails until someone adds an
entry saying why it is consolidation and not sprawl.** The gate forces a recorded decision; it does
not adjudicate one.

**The idiom already exists here twice** — `DECLARED_UNDOCUMENTED`
(`crates/prikk-cli/src/commands/tests.rs`) and `RFC114_ADMITTED_BUT_UNWRITTEN`. Both work because
every entry must state a real reason rather than a placeholder. Match that.

**3.3 What must never be gated (§5): line count and module count.** They are the numbers that
prompted the work and the least diagnostic of everything measured. Gating them would institutionalise
watching the wrong thing, and would fire on healthy feature growth while a second cycle formed
unwatched.

## 4. The four design questions §4.2 leaves to you

1. **The threshold.** K = 8 is a proposal, not a ruling. Choose it so today's hubs are the declared
   set and the next accretion trips it — **derived from your measured distribution, with the
   derivation written down**, not asserted.
2. **Where it runs.** `boundary-check` is the natural home. Confirmed: `boundary.rs` already composes
   seven sub-checks (`changelog_history`, `open_work_index`, `package`, `placement`, `publication`,
   `rfc_naming`, `unsafe_boundary`), each a module with its own `check(root, errors)`, and
   `boundary::tests::workspace_and_product_boundaries_hold` runs the whole thing **against the real
   repository root under `cargo test`**. An eighth module is the established shape.
3. **Edge extraction is the load-bearing detail.** The definition must be stated and tested, or §1's
   drift happens inside our own repository. Name what counts: `use crate::<module>` forms, grouped
   imports, re-exports, fully-qualified `crate::x::y` paths in expression position.
4. **Test files must be excluded, and the exclusion must be part of the tested definition.**
   `fsutil`'s only outward edges are test-only; a gate counting them would report the cleanest module
   in the crate as coupled.

## 5. The control that matters most — the RFC's own counter-example

**§4 establishes that a bare K = 8 bound would have rejected RFC 122**, a correct fix for a
High-severity defect. That is why the design is an allowlist rather than a bound, and it is a
checkable claim, not a rhetorical one.

**Run your finished gate against `7a01168` (RFC 122's commit) and its parent, and show that it accepts
both.** If it rejects either, the design has been implemented as a bound somewhere despite §4.1, and
that is a stop-and-report.

Also required:

- **The current graph passes**, obviously — but show the declared set your extractor produced, not
  just that the gate is green.
- **A second cycle fails.** Introduce one in a scratch worktree, confirm the build fails, remove it.
  A cycle gate nobody has seen fire is not evidence.
- **A new hub fails until declared.** Same shape: push a module across your threshold, confirm the
  failure names it, then confirm adding an allowlist entry with a reason clears it.
- **An allowlist entry with an empty or placeholder reason is refused.** That is the property that
  makes the idiom work; `DECLARED_UNDOCUMENTED`'s own tests are the model.

## 6. Gates

The full set from `rfcs/EXECUTION-ORDER.md` §6 rule 9, run as the last action against the exact final
commit: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features --locked
-- -D warnings`; `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`;
`cargo +1.85.0 check --workspace --all-targets --locked`; `git diff --check`; `cargo audit
--no-fetch`; `RUSTDOCFLAGS="-D rustdoc::private_intra_doc_links" cargo doc --workspace --no-deps`;
release-policy `check`, `boundary-check`, `reference-check`.

**`boundary-check` is the gate you are changing** — report its result separately, and report the
`prikk-release-policy` test count before and after.

Cross-target clippy only if your own diff introduces `#[cfg(target_os)]`; check the diff.

## 7. Out of scope

**RFC 131** (grouping and `pub(in ...)` scoping) is sequenced after this and is not part of it — §7 of
the RFC says so. **No module moves, no renames, no visibility changes** in this increment: the gate
records today's structure, it does not repair it. **No crate split** (§6, ruled out on evidence
including `fsutil`).

**Do not act on the `active ↔ refs` cycle.** It is grandfathered with a reason; removing it is a
separate design question nobody has opened.

## 8. Reporting

Per `.git-exclude/tasks/dev-team/003-landing-work-on-main.md`: commit locally on `main`, do not push,
report to `.git-exclude/review-request/`. Include §2's re-derivation and how it compares to §2's own
table, §4.1's threshold derivation, §5's four control results, and every departure.

**If your measured graph contradicts §2 on a conclusion, that is the report** — do not implement
around it.
