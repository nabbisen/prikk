# `tag create` / `branch create` — apply the maintainer trust gate: implementation handoff

**Base:** current `main` (`930ade1`, CI + Docs green). **Under `003-landing-work-on-main.md`.**
**Origin:** `.git-exclude/reviewed/tag-create-trust-gate-investigation-v1.md`, Part 1.
**Owner ruled 2026-08-24:** investigate first, then fix. **The investigation found a defect against the
project's own stated design.**

**This is the first code-behaviour increment in this arc.** Everything before it was documentation. **A
refusal that did not exist before will now exist** — read §3 before touching anything.

---

## 1. What is wrong

**Two surfaces publish maintainer-signed objects without checking the signer against the local trust
policy:**

| Surface | `maintainer_signer_from_env` | `verify_signer_trusted` |
|---|---|---|
| `crates/prikk-cli/src/tag.rs` | 1 | **0** |
| `crates/prikk-cli/src/branch.rs` | 2 | **0** |
| `crates/prikk-cli/src/seal.rs` (the model) | — | **3** |

`seal`, `merge`, `sync seal`, `sync build` and `sync adopt-tag` all gate. **These two do not.**

## 2. Why this is a defect and not a design choice — the evidence, which you should re-derive

- **DC-11 deferred tag signing with an explicit forward requirement** (`DC-11-MAINTAINER-TRUST-STORE.md:54-55`):
  *"tag publication/signing is outside DC-11. **The same real-key rule must be applied when tag signing
  is designed.**"*
- **DC-63 designed it** (§4, "Signing"): *"Tag objects and tag ref states are both maintainer-signed,
  **on the same terms as `seal`**... Reuse `maintainer_signer_from_env`; add no signing path."*
- **`seal`'s terms include `verify_signer_trusted`.** The implementation took the signer **source** and
  not the **gate**.
- **No RFC excludes it.** DC-63's out-of-scope list is the clock-authority question and tag *moving*.

**Re-derive all four.** My handoffs have been wrong on scope four times in this arc; this one changes
behaviour, so the cost of my being wrong is higher than usual. **If DC-63 or DC-11 does not say what I
claim, stop and report — do not implement.**

## 3. The change, and the one judgment inside it

**Add the same check `seal` performs**, at the same point in the flow — **before any object or ref
write**, so a refusal leaves nothing behind. Match `seal.rs`'s call shape exactly rather than inventing
one.

**The judgment: `branch close` and any other ref-publishing path in `branch.rs`.** `branch.rs` obtains a
signer **twice**. **Adjudicate each**: does it publish a maintainer-signed object? If yes, it needs the
gate on the same reasoning. **Report each site and your verdict**, including any you decide against.

**Do not touch `adopt_tag`** — it already gates correctly (`tag_travel.rs:421`).

## 4. Impact — state it, do not assume it

**Expected to be near-zero**, and I want that verified rather than asserted: the operator's own
maintainer key must now be in their own trust policy before creating a tag or branch. **Anyone who has
ever run `seal` already satisfies this**, since `seal` has always required it. The remedy is
`prikk trust maintainer add`.

**Confirm the refusal message is actionable** — a user who hits this must be able to tell what to do.
Reuse `seal`'s message if it already reads correctly in this context; **say which you did.**

## 5. Tests — this is where the increment earns its keep

**A behaviour change needs a test that fails without it.**

1. **A test that creates a tag with an untrusted signer and asserts refusal**, plus the same for
   `branch create`. **Assert on the specific refusal**, not merely `is_err()`.
2. **Run each new test against the pre-change code and confirm it fails** — the negative control this
   project asks for on every increment. **Report the observed failure**, not just that you ran it.
3. **Check the existing suite for tests that create tags or branches with an untrusted key and currently
   pass.** Any such test encodes the old behaviour and will break. **Do not weaken one to make it pass**
   — report it, since a test that had to change tells you the behaviour really did change.

## 6. Out of scope

- **`verify`'s exclusion of `Tag` from publication-trust checking** — Part 2 of the investigation, a
  **deliberate follow-up** so its effect can be seen separately. **Do not add `Tag` to that match here.**
- **`tag_travel.rs:361`'s false doc comment** (*"the same as `prikk tag create`"*). **After this
  increment it becomes true.** Fix it in the same commit if it does — **and say so**; if it does not,
  report why.
- **Documentation.** `trust-threat-model.md` may need updating once this lands; **report, do not edit.**
- **`MILESTONES.md`, `ROADMAP.md`, the badge.**

## 7. What to report

1. **Your re-derivation of §2's four claims** — each confirmed or corrected.
2. **Every site changed**, and every `branch.rs` site adjudicated (§3), including those you left alone.
3. **The negative control, with its observed failure output** (§5.2).
4. **Any existing test that had to change**, and why that is a behaviour change rather than a weakening.
5. **The refusal message**, and whether you reused `seal`'s.
6. **Whether `tag_travel.rs:361` became true** (§6).
7. **Full gate set against the exact commit, after the last edit.** **Test counts will change** — say by
   how much and which tests are new.
8. Anything here that was wrong.

**Stop and escalate, do not guess**, if: §2's RFC evidence does not hold; a `branch.rs` site is genuinely
ambiguous; an existing test's failure suggests the old behaviour was relied on deliberately; or the gate
turns out to break a legitimate workflow that has no `trust maintainer add` remedy — **that last one
would mean the design ruling itself needs revisiting, and it is the finding I would most want to hear.**
