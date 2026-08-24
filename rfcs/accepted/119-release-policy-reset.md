# RFC 119 — Release policy tooling: reset

**Status.** **ACCEPTED by the project owner 2026-08-24**, who ruled that DC-45's assets and approach
*"do not match our reality, the currency and our perspective"* and that **rewriting design and scripts is
acceptable.** **§8's prerequisites precede design.**

**Independence.** Author-reviewed — the standing ceiling. **The architect proposed the reset framing and
records it; §7 states what that leaves unchecked.**

**Arises from.** An investigation into DC-45's undischarged decommissioning obligation, which found the
obligation's premise no longer describes the system.

**Supersedes.** DC-45's three outstanding obligations (§4). **Reframes** DC-93 and DC-94, which are
downstream of them.

---

## 1. The finding: correctness is defined by a retired tool

**`tools/release-policy/src/oracle/verify.rs:23`:**

```rust
const OBSERVATIONS_PATH: &str = "release/oracle/python-observations-v1.json";
```

**The standing gate — `cargo run -p prikk-release-policy --locked -- check`, "all 154 oracle cases
passed", run before every commit in this project — validates the Rust implementation against recorded
observations of the Python harness it replaced.**

**So the Rust tool's definition of *correct* is "matches what the Python did."**

That is not a migration artifact awaiting cleanup. **It is the current, load-bearing definition**, and
it has three consequences:

- **Retiring the Python does not retire its authority.** The frozen observations carry it indefinitely.
  DC-93's deletion was never blocked by the files; it is blocked by what defines correctness.
- **It is a transcription at the root** — 28KB of one tool's recorded behaviour, standing in for a
  statement of what the policy *is*. **RFC 118's principle is violated at the foundation of the very
  tool that enforces other rules.**
- **DC-94's map binds Rust check categories to *Python* check categories.** Its own prerequisite asks
  *"what is an executed check registry?"* — a question that has no good answer while the registry's
  counterpart is a dead harness.

## 2. What DC-45 actually achieved, stated fairly

**It worked.** The Rust harness exists, runs in CI, is ~2000 lines, and its checks are real —
`boundary-check`'s eleven categories, `reference-check`, `release-notes`. **Nothing here says the
previous work was wasted or careless.**

**What it did was a faithful migration**, and a faithful migration's correctness criterion is
necessarily *"the new one matches the old one."* **The defect is that the criterion was never
retired.** The scaffolding became the foundation.

## 3. Current reality the design must match

- **The Python is not in CI.** No workflow invokes it.
- **It survives as the oracle** for `differential-check`, which is itself neither in CI nor in the
  standing gate set.
- **`release/` holds 18 Python files and 37 JSON artifacts**, including a 278KB manifest.
- **The release procedure itself changed 2026-08-23**: the owner's scheduling grant superseded the
  three-authority lane transition. **DC-45's obligations were written for a release regime that no
  longer operates.**

## 4. The three obligations, and what happens to them

DC-45 required `ROADMAP.md`, `MILESTONES.md` and `rfcs/IMPLEMENTATION-STATUS.md` to *"continue to name"*
three obligations *"until each is accepted"*:

| Obligation | Disposition |
|---|---|
| **Stability rerun** | **Discharged** 2026-08-08 |
| **Five-file decommissioning** | **Superseded.** Its premise — that removal is imminent and gates a release-candidate increment — is false: the files are the differential oracle, and the RC-increment trigger belongs to a superseded release regime |
| **Eight-file evidence retention** | **Superseded by the same reasoning**; to be restated by this RFC's design or dropped |

**Two of the three named documents no longer carry status** — `rfcs/IMPLEMENTATION-STATUS.md` is retired
and `ROADMAP.md`'s status sections are removed. **The obligation's own mechanism is gone.**

**This RFC discharges the reporting obligation by superseding it, not by ignoring it.**

## 5. What the reset must produce

**A statement of what release policy *is*, independent of any implementation.** Today the answer is a
manifest of cases; **the design must say what the policy asserts about a release**, in terms a reader can
check against the product, not against a prior tool.

**Then, in RFC 118's terms:**

- **The oracle derives, or is authored as specification.** What it must not be is a transcription of a
  retired implementation's outputs.
- **Whatever remains hand-maintained is declared**, with a completeness gate over it — Gate A's shape.
- **The Python is removed** once nothing defines correctness by reference to it, which is a consequence
  of the above rather than a separate obligation.

## 6. Non-goals

- **Not a criticism of DC-45's execution** (§2).
- **Not deleting checks.** `boundary-check`'s eleven categories, `reference-check` and `release-notes`
  are real and stay. **Their oracle is what is in question, not their existence.**
- **Not a release-procedure change.** The 2026-08-23 grant governs; this RFC touches tooling only.
- **Not RFC 118's stages** — this is a sibling application of the same principle, not a dependency.

## 7. What the author-review ceiling leaves unchecked

**The architect found the `python-observations-v1.json` dependency, judged it foundational rather than
incidental, and proposed a reset on that basis. Nobody has tested that judgment.**

**The specific risk: the observations may encode genuine policy decisions that exist nowhere else.** If
so, "retire the oracle" would destroy the only record of what the policy is — and the correct answer
would be to *recover* the policy from them first. **§8.1 exists for exactly that.**

## 8. Blocking prerequisites

1. **What do the 154 cases actually assert?** Read the manifest. **Do they encode policy decisions
   recorded nowhere else?** If yes, recovering them is the first work, not the last.
2. **What is `differential-check` for, once the Python is gone?** It compares two implementations; with
   one, it has no counterpart. **Retire, or repurpose against the specification?**
3. **What survives of `release/`'s 37 JSON artifacts** — which are policy, which are fixtures, which are
   the migration's own bookkeeping?
4. **Does DC-94 still make sense?** Its map binds Rust categories to Python ones. **Reframed, or
   withdrawn?**
