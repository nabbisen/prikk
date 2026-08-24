# Trust gate — caller-level negative coverage for the five untested surfaces

**Base:** **do not start this until `verify-local-tag-publication-trust-handoff-v1.md` has been
implemented, reviewed and pushed** — that is the *increment*, not the commit that issued its handoff.

**Correction, 2026-08-24.** This line previously read *"after ... (`a2d9a3f`) lands"*. **`a2d9a3f` is
the commit that issued that handoff — doc-only, no source touched.** Citing it as the precondition
conflated "the handoff exists" with "the work is done", and the dev team caught it by checking whether
the named base had actually landed before starting. **The precondition is an implemented increment with
a review in `.git-exclude/reviewed/`, not a SHA in `rfcs/handoffs/`.**

**Under `003-landing-work-on-main.md`.**
**Origin:** architect sweep, 2026-08-24, answering *"is there any fix required on implementation or
tests around it?"* — **implementation: no. Tests: this.**

**Pure test coverage. No behaviour change, no source edit outside test files.**

---

## 1. Why this exists — the argument, not the checklist

`verify_signer_trusted` **is** unit-tested for both outcomes (`trust/tests.rs:240,424`).

**Those unit tests passed the entire time `prikk tag create` was ungated.** They prove the *function*
works. They prove nothing about whether any *caller* calls it — which was precisely the defect
`053e442` fixed.

**Today, caller-level proof exists for exactly three of eight gated surfaces**, all three added by
`053e442`. **If `seal`'s gate were deleted tomorrow, the whole suite would still pass.** `seal` is the
model DC-63 told every other surface to follow, and it is the least-proven of them.

**This increment converts "we believe these gate" into "these are proven to gate."**

## 2. The five surfaces, and where each gate lives

| Surface | Gate | Suggested host |
|---|---|---|
| `prikk seal` | `seal.rs:129,151` | your choice — **`seal` is the priority of the five** |
| `prikk merge` | `merge_execute.rs:118` | `dc74_merge_execution.rs` |
| `prikk sync build` | `sender.rs:201` | `rfc116_sync_cli.rs` |
| `prikk sync seal` | `seal_from_accepted.rs:160` | `rfc116_sync_cli.rs` |
| `prikk sync adopt-tag` | `tag_travel.rs:421` | `rfc117_stage3_tag_travel_cli.rs` |

**Verify each line number** — they are from my sweep, and my line references have been wrong before.

## 3. Method: reuse the scenario, swap the signer at the last step

The three tests from `053e442` are the template (e.g. `dc63_tag_surface.rs:445-463`). The shape:

1. Build the scenario with the **existing** helpers — `seeded_repo`, `support::trust_maintainer`.
2. At the **final** command only, override `PRIKK_MAINTAINER_KEY_ID`/`PRIKK_MAINTAINER_SEED` with an
   untrusted-but-**well-formed** key.
3. Assert the command fails **and** that stderr contains `"not trusted by policy"`.

**The well-formed part is load-bearing.** `053e442`'s tests use a valid 64-hex seed so the refusal is
for being *untrusted*, not *malformed*. **A short or invalid seed makes the test pass for the wrong
reason** — the exact failure this project keeps finding. **Reuse a valid seed.**

**`sync build`, `sync seal` and `adopt-tag` need real prior state** — a have-list, an accepted claim, a
received tag. **Reuse the existing scenario setup in those files rather than building new fixtures**;
these three are the reason this is not a fifteen-minute increment.

## 4. Negative controls — the point of the increment

**Each new test must be observed failing with its gate removed.** Not "the suite passes" — **the
specific test, with the specific gate neutralised, failing.**

**Use a mutation that keeps the call**, e.g. `let _ = verify_signer_trusted(...)` rather than deleting
the line. **I used exactly that when reviewing `053e442`** and it is strictly stronger: a mutation that
deletes the call would also be caught by anyone grepping for the gate, while a swallowed refusal would
not. **Report the observed failure output for each of the five.**

**Restore the source and confirm `git status` is clean afterwards** before running the final gates.

## 5. Out of scope

- **Any source change outside `crates/prikk-cli/tests/`** (and store-level test modules if a surface
  cannot be driven through the CLI — **say which and why** if so).
- **The gates themselves.** If one turns out not to fire, **stop and report** — that is a finding, not
  something to fix here.
- **`verify`'s Tag handling** — the increment before this one.
- **`MILESTONES.md`, `ROADMAP.md`, the badge.**

## 6. What to report

1. **Five tests, one per surface**, with the host file for each.
2. **Five negative controls, each with observed failure output** (§4).
3. **Confirmation each test's untrusted key is well-formed** (§3) — quote the seed length.
4. **Any surface that could not be driven through the CLI**, and what you did instead.
5. **Anything that did not fire** — §5's stop-and-report.
6. **Full gate set against the exact commit, after the last edit.** **Test counts rise by five**; say so.
7. Anything here that was wrong, **including my five line numbers**.

**Stop and escalate, do not guess**, if: a gate does not fire under its negative control; a surface needs
so much scaffolding that the test would duplicate an existing scenario wholesale (**say so — reusing an
existing test's setup by extracting a helper is fine, copying it is not**); or you find a **sixth**
maintainer-signing surface my §2 sweep missed — **that would mean the implementation is not clean after
all, and it is the finding I would most want to hear.**
