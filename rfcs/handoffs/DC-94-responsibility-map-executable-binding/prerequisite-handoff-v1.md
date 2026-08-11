# DC-94 Responsibility Map Executable Binding — Prerequisite Handoff v1

**Cleared to answer §3's four questions only.** Accepted 2026-08-11,
`rfcs/accepted/DC-94-RESPONSIBILITY-MAP-EXECUTABLE-BINDING.md`. **No design in this pass.**

## 1. What this is

DC-52's obligations 1 and 2, decoupled from its retirement obligations. **This gates nothing and nothing
gates it** — the coupling that made these preconditions for retiring the Python is withdrawn, so each
now stands or falls on whether it is worth doing.

- **Bind the 50-entry responsibility map to an executed check registry**, so a map entry with no
  executed check — or an executed check with no map entry — fails closed. Same shape as DC-51's
  dependency placement and DC-90's unsafe boundary: **a rule a document asserts and nothing enforces.**
- **Make the `defaults.run` invariant explicit.** The governed-procedure YAML extractor skips an empty
  `run` whose parent key is `defaults`, relying on the GitHub Actions schema forbidding an executable
  scalar there. Correct, and unenforced. DC-45's architect review v11 required it be made explicit.

## 2. Start with §3.1, because this may already be done

`tools/release-policy/src/oracle/self_test/responsibility.rs` exists — 55 lines, loading
`tools/release-policy/self-test-responsibility-map-v1.json` with `deny_unknown_fields`. **Some binding
machinery is already there.** How much is the first thing to establish.

Read it and its self-test, and state precisely what is enforced today: map *shape* only, or does it
already relate entries to executed checks? **The gap is the increment.** If the binding turns out
largely present, **that is a complete and useful outcome** — record what is enforced, close DC-45's
obligation explicitly instead of leaving it as prose, and end there. §5 of the RFC says this and means
it: discovering an obligation was already discharged beats building a second mechanism beside a working
one.

## 3. The rest

**§3.2 — what is "an executed check registry"?** There has to be something enumerable to bind against.
Report whether one exists, can be derived from the existing check dispatch, or would have to be
introduced. **And if introduced, say what keeps *it* from drifting** — a registry nothing verifies is
the original problem restated one level up, and I would rather hear that objection from you than
discover it at review.

**§3.3 — is bidirectional failure right?** Map-entry-without-check and check-without-map-entry may not
be equally wrong. Report which directions should fail closed, with the reason, rather than assuming
symmetry.

**§3.4 — the `defaults.run` validator's blast radius.** Confirm a tightened rule accepts every procedure
in the tree today. **A validator that fails closed on a valid workflow is worse than the assumption it
replaces** — that is the whole reason this is a prerequisite and not a one-line change.

## 4. Limits

- **No design in this pass.** Answers first.
- **No change to what the checks themselves verify** — this binds the map to them; it does not alter
  them.
- **No new dependency.** The tool parses TOML and JSON already; this needs no more.
- **Nothing from DC-93.** No Python is retired here.
- **No release-lane, signer, or publication action.**

## 5. Reporting

`.git-exclude/review-request/`, plain `.md`. Answer §3 in order. Findings outside scope go in the
report; I register them.

## 6. Sequencing

- **DC-93 is accepted and independent.** Either order — but both touch `tools/release-policy` from
  different directions, so **do not run them in one branch.**
- Touches no product code; an ordinary CI run suffices, not the three-platform rule.
