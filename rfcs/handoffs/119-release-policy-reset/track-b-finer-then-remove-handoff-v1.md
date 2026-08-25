# RFC 119 track B — resolve FINER, then remove NEVER

**Base:** current `main` (`0316262`, CI + Docs green). **Under `003-landing-work-on-main.md`.**
**RFC:** `rfcs/accepted/119-release-policy-reset.md` §10 track B — **the last track, and the only one
where things are deleted.**

**Read §1 before anything. My verdicts have been wrong twice, both in the same direction, and I have
just corrected two more while writing this.**

---

## 1. Every verdict in this handoff is suspect, and here is the evidence

**The reconciliation verdicted `publication-allowlist` as post-1.0 "who may publish."** It actually
validates **the eight packages in topological publish order** and **sixteen `cargo package`/`cargo
publish` procedures** — the sequence executed by hand to publish `0.23.0`, where a wrong order leaves a
half-published release. **Corrected to NOW.** I had read the category name.

**Writing this handoff, two more corrected the same way:**

- **`dependency-placement`** — I had it FINER. It enforces that a member crate may depend **only on
  `prikk-*` or an explicit allowlist, with no renamed entries**. **That is the rule that keeps
  `prikk-cli` at zero third-party dependencies.** **NOW, not FINER.**
- **`tool-metadata`** — I had it FINER as "the tool's own manifest." It is that, and reading it
  surfaced a **gap**: it asserts MSRV `1.85.0`, edition 2024 and workspace inheritance **for the tool
  crate only. Nothing asserts them for the eight product crates**, which do inherit today but are
  ungated. **See §4.**

**Three corrections, all from reading the implementation instead of the label. Assume the rest are
wrong too, and read each one.**

## 2. Method

**For every item below: read the implementation, then answer *"what does this prevent, for this project,
today?"*** Verdict **NOW** (keep unchanged), **LATER** (park, per track A's mechanism and its
requirement that it must not run), or **NEVER** (remove).

**Report a verdict for every item, including the ones you keep.** A KEEP with its reason is as much of
the deliverable as a removal.

**Removal order matters — see §5.**

## 3. `release-evidence`'s 73 cases — case-level, not suite-level

**The single largest item, and a suite verdict would be wrong.** Its `primary_reason` distribution shows
at least three subjects inside one suite:

| Reason | Cases | Looks like |
|---|---|---|
| `evidence-tag-or-artifact` | 10 | **G4 identity — needed now** |
| `evidence-byte-identity-or-link` | 6 | **G4 identity — needed now** |
| `evidence-transition-or-attempt-prefix` | 11 | lane transitions — superseded |
| `governance-transition-or-proof` | 5 | post-1.0 |
| `schema-instance` | 18 | validation of the evidence format itself |
| `none` | 20 | passing cases across the above |

**A suite-level verdict would have discarded ~16 cases serving a guarantee prikk needs today.**

**Adjudicate case by case.** The reason codes are a starting index, **not** the answer — **I derived that
table from identifiers and reason strings, not from reading the cases.**

## 4. The `tool-metadata` gap — report, do not fix

**Nothing asserts MSRV, edition, or workspace inheritance for the eight product crates.** They inherit
today; nothing gates that they continue to.

**This is a derived-need-with-no-check** — the same shape as G1 — and it belongs to G3.

**Report it. Do not build it in this track**, which is about removal. **If you think it belongs here,
say so rather than adding it.**

## 5. Removal order — `differential-check` and the Python are entangled

**`differential-check` is NEVER** (migration scaffolding; already non-functional since track A parked
the signer cases). **The Python is its only consumer.**

**But the Python is named elsewhere**: `command_scan.rs` carries `python3 release/check-policy.py` as an
invocation marker, and `reference-check`'s inventory names it as a documented procedure. **Removing the
files without those references is a broken build; removing the references without the files is a false
claim that they exist.**

**Sequence it, and report the order you used.** **If removing one forces a change you did not expect,
that is a finding** — track A's own orphaned-pack ripple is the precedent.

## 6. The remaining NEVER items

- **`release-state`'s 23 cases** — the release-lane state machine, superseded 2026-08-24.
- **`json-parser`/`schema-evaluator`'s 15 cases** — the tool's own strictness layer over `serde_json`, on
  files this project authors. **Removing the cases is not the same as removing `json.rs`/`schema.rs`** —
  adjudicate those separately, and **do not delete code because its oracle cases went.**
- **`rfc-naming`** — verdicted NEVER **as release policy**. **That is a placement judgment, not a
  deletion proposal.** It gates RFC filenames and does so correctly. **Do not remove it.** If it should
  move out of `boundary-check`, say so; **moving is out of scope here.**

## 7. Out of scope

- **Building anything** (§4).
- **`release-signers.toml`**, product behaviour, the release procedure.
- **Track A's parked material.** It is parked, not pending removal.
- **`differential-check`'s revival** — it is being removed, not fixed.

## 8. Controls

1. **After each removal, the full gate set passes** — and **`check`'s case count changes**; report each
   step's count.
2. **Nothing silently keeps running**: confirm no removed case is referenced by any surviving file
   (track A's closure check is the precedent for how this surfaces).
3. **Removals are observable**: for at least one removed check, confirm the property it asserted is
   genuinely unasserted afterwards — **and say whether that matters.**

## 9. What to report

1. **A verdict for every item**, KEEPs included, with what each prevents (§2).
2. **`release-evidence` case by case** (§3) — the largest deliverable.
3. **The `tool-metadata` product gap** (§4), report only.
4. **The removal order used, and any ripple** (§5).
5. **All three controls** (§8), with counts at each step.
6. **Full gate set against the exact commit, after the last edit.**
7. **Which of my verdicts were wrong** — §1 says expect several.

**Stop and escalate, do not guess**, if: a NEVER item turns out to assert something prikk needs today —
**that has happened three times and is the most likely failure**; removing something makes an unrelated
check fail in a way that is not a simple reference update; or **the case-level work on
`release-evidence` reveals the reason codes do not partition the suite the way §3 assumes.**
