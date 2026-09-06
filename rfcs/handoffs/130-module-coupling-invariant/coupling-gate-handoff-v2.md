# RFC 130 — the module coupling invariant and its gate (v2)

**Supersedes** `coupling-gate-handoff-v1.md`, which instructed building §4.1's *absolute* acyclicity
rule and is marked do-not-work-from.
**RFC:** `rfcs/accepted/130-module-coupling-invariant.md` — **§4b is the ruling and is settled input.**
**Base:** `main` at `c5b1010`.

**The reissue was asked for and it was right to ask.** The dev team stopped rather than work from a
superseded document or write the replacement themselves. **§1 is the only section that changes; every
other instruction in v1 survives and is restated here so this file stands alone.**

---

## 1. What changed since v1 — acyclicity is an allowlist, not an absolute

**v1 said: "a second cycle fails the build, full stop."** That is withdrawn.

**§4b ruled: a new cycle fails the build until it is declared with a reason** — the same
allowlist-with-reasons idiom §4.1 already chose for hubs, and which §4 applied to hubs and not to
cycles. The correction finishes an amendment §4 left half-applied; it is not a weakening.

**Why the absolute rule could not ship:** RFC 122 — a correct fix for a High-severity defect, found
by an external audit and required by the architect — **created a cycle.** An absolute rule would have
rejected it, exactly as the bare degree bound §4 already rejected would have. §4 found the right
counter-example and applied it to one of its two rules.

**One place this is stricter than the hub treatment (§4b.3).** A hub entry says why this is
consolidation rather than sprawl. **A cycle entry must additionally state what would have to change to
remove it.** That makes the allowlist a ledger of structural debt rather than a list of permanent
excuses. **An entry that cannot say what would remove the cycle is an entry nobody understands** —
report it rather than inventing wording.

## 2. The four cycles, and why declaring three of them is real work

The crate has **four cycles merging six modules into one strongly-connected component** — `active`,
`refs`, `trust`, `worktree_patch`, `patch_replay`, `lifecycle_cache`:

| Cycle | Standing |
|---|---|
| `active ↔ refs` | the only one anyone has ever evaluated |
| `trust ↔ refs` | present at `04e9391` and **missed by two independent derivations**; never evaluated |
| `lifecycle_cache ↔ patch_replay` | created by `7a01168` (RFC 122); never evaluated |
| `active → worktree_patch → patch_replay → active` | created by `7a01168`; never evaluated |

**Per-cycle entries, not one per-SCC entry** (§4b.4). The six-module component is the symptom; the
four cycles are the causes and three are independently removable. One entry covering the cluster would
record the symptom and lose every cause.

**Writing three of these reasons is the evaluation that has never happened.** If a reason cannot
honestly be written — if `trust ↔ refs` turns out to be accidental coupling nobody intended — **that
is a stop-and-report, not an entry to invent.** It is also the gate paying for itself before it runs
once.

## 3. Everything below survives from v1, restated

**3.1 The external artifact never came, and rebuilding is better.** RFC 130 §8's offered
coupling-gate script and 68-node edge list were requested on 2026-09-01 and never arrived. Do not
wait. **Rebuilding is the better outcome**: §4.2 item 3 requires the gate's edge definition to be
stated and tested, and an allowlist derived from a *different* extractor than the gate uses drifts
from it silently. **The gate's own edge definition is authoritative and every allowlist entry derives
from it.**

**3.2 Re-measure; do not copy §2's table.** Those numbers are from `04e9391`. Since then RFC 122
moved two hubs, RFC 123 and RFC 135 added code, and four releases shipped. **Re-measure at your own
base with your own extractor and write the derivation down.** §2's own structural conclusion has
already been falsified once (§4a); if your graph disagrees with it again on a *conclusion*, stop and
report.

**3.3 What the gate enforces beyond cycles.** Middle-hubs: **declared, not bounded.** A module newly
crossing the threshold fails until someone records why it is consolidation and not sprawl.

**3.4 What must never be gated (§5).** Line count and module count — the numbers that prompted the
work and the least diagnostic of everything measured.

**3.5 §4.2's four open questions are yours**, with one now sharper:

1. **The threshold** — derived from your measured distribution, with the derivation written down.
   **§4a left a methodology difference to settle first: 61 production top-level modules by the
   re-derivation against §2's 68.** That is eight `#[cfg(test)]`-gated top-level modules, not drift.
   **Both numbers cannot seed a threshold; settle which set counts before deriving one.**
2. **Where it runs** — `boundary-check` composes seven sub-checks, each a module with its own
   `check(root, errors)`, and `boundary::tests::workspace_and_product_boundaries_hold` runs the whole
   thing against the real repository root under `cargo test`. An eighth module is the established
   shape.
3. **Edge extraction** — state the definition and test it: `use crate::<module>` forms, grouped
   imports, **re-exports** (the re-derivation showed a cycle visible *only* through crate-root
   re-exports), and fully-qualified `crate::x::y` paths in expression position.
4. **Test files excluded, and the exclusion part of the tested definition.** The re-derivation's
   `cfg` satisfiability approach is the right one and worth reusing: **a `#[cfg(...)]` marks a module
   test-only only if its formula cannot be satisfied with `test` false.** `fsutil/anchored.rs`'s
   `none` module contains the word `test` in its gate and is genuine production code on some
   platforms; a substring check gets it wrong.

## 4. Controls

1. **The RFC's own counter-example.** Run the finished gate against `7a01168` and its parent and show
   it **accepts both** — with the cycle declared, as §4b requires. If it rejects either, the design
   has been implemented as an absolute rule despite §4b, and that is a stop-and-report.
2. **The current graph passes**, and **show the declared set your extractor produced** — not merely
   that the gate is green.
3. **A fifth, undeclared cycle fails.** Introduce one in a scratch worktree, confirm the failure names
   it, remove it. A cycle gate nobody has seen fire is not evidence.
4. **Declaring it clears the failure**, and **an entry with an empty or placeholder reason — or with
   no statement of what would remove the cycle — is refused.** That is the property that makes the
   idiom work; `DECLARED_UNDOCUMENTED`'s own tests are the model.
5. **A new hub fails until declared**, same shape.

## 5. Gates

The full set from `rfcs/EXECUTION-ORDER.md` §6 rule 9, run as the last action against the exact final
commit: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features --locked
-- -D warnings`; `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`;
`cargo +1.85.0 check --workspace --all-targets --locked`; `git diff --check`; `cargo audit
--no-fetch`; `RUSTDOCFLAGS="-D rustdoc::private_intra_doc_links" cargo doc --workspace --no-deps`;
release-policy `check`, `boundary-check`, `reference-check`.

**Quote every command in the list.** `boundary-check` is the gate you are changing — report it
separately, and report the `prikk-release-policy` test count before and after.

## 6. Out of scope

**RFC 131** (grouping and `pub(in ...)` scoping) is sequenced after this. **No module moves, no
renames, no visibility changes**: the gate records today's structure, it does not repair it. **No crate
split** (§6, ruled out on evidence). **Do not act on any of the four cycles** — declaring one is not
licence to remove it, and removing one is a separate design question nobody has opened.

**No `CHANGELOG.md` entry**: this adds no user-facing surface. Say so in the report rather than
leaving it unmentioned.

## 7. Reporting

Per `.git-exclude/tasks/dev-team/003-landing-work-on-main.md`: commit locally on `main`, do not push,
report to `.git-exclude/review-request/`. Include §3.2's re-derivation and how it compares to §2's
table, the threshold derivation and which module set it used, §4's five control results, **the four
declared cycle entries with their reasons and their what-would-remove-it statements**, and every
departure.
