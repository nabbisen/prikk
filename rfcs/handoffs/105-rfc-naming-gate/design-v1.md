# RFC 105 — RFC naming gate — design v1

**RFC:** `rfcs/accepted/105-rfc-naming-gate.md`. Read §0 first; the causes are the design constraints.

**Note the directory this file is in.** `rfcs/handoffs/105-rfc-naming-gate/` — `NNN-slug`, per RFC 100,
which governs handoff directory names as well as RFC filenames. The four increments this RFC exists to
prevent got that wrong too.

## 1. Where it goes

`tools/release-policy/src/boundary/`, alongside `unsafe_boundary.rs` and `placement.rs`, wired into
`boundary::check` the same way. **`unsafe_boundary.rs` is the model to read first** — not for its subject
but for its shape: a named frozen constant, a self-guard that makes the constant hard to widen quietly,
and a module doc that states what the gate *cannot* see.

Not `reference-check`: that reads documentation references for link integrity and this is a structural
rule about filenames. Keeping them separate keeps each one's failure message meaningful.

## 2. The rule

For every entry directly under `rfcs/accepted/`, `rfcs/done/`, `rfcs/archive/` (files) and
`rfcs/handoffs/` (directories):

- **Conforming**: matches `^[0-9]{3}-[a-z0-9]+(-[a-z0-9]+)*` — three-digit zero-padded number, lowercase
  hyphenated slug. Files additionally end `.md`.
- **Otherwise**: must appear in the frozen legacy allowlist.
- **Anything else fails.**

## 3. The allowlist — derive it, do not transcribe it

**RFC §3.2 is the point here.** RFC 100's own table states 86 `DC-` files for range 09–95; the tree no
longer matches, because four were added to it. **A list copied from prose inherits the prose's staleness
— the same error one layer up from the one this gate exists to prevent.**

Generate it from the tree at the commit this lands, record the command you used in the module doc, and
state the counts you got. My own measurement, for cross-checking rather than for copying: **86 `DC-`, 30
`PR-`, 6 numeric, and one unnumbered handoff directory** (§5).

## 4. The self-guard

RFC criterion 3: **every allowlisted name must correspond to an entry that exists.** A name in the list
with no file fails the check.

That closes the cheap bypass — pre-authorising `DC-100` before creating it. It does not close a
deliberate two-line edit adding both file and entry, and **it should not pretend to**: that is exactly
the standard `UNSAFE_EXEMPT_CRATES` sets, where the exemption is *"named explicitly, never inferred."* A
visible edit to a reviewed constant is the control; invisibility was the problem.

**Say this in the module doc**, in the "what this gate cannot see" section `unsafe_boundary.rs` already
models. Also say: it cannot tell whether a slug describes its RFC, and it cannot catch a *wrong number*
that is correctly formatted — 106 used twice would pass.

## 5. Report before implementing

1. **The derived inventory** and the command that produced it.
2. **`rfcs/handoffs/consolidation`** — a handoff directory with no RFC number, found while measuring. It is
   either a legacy entry to freeze or a finding worth raising. **Do not decide it silently**; report what
   it is and what it belongs to, and I will rule.
3. **Whether any other entry fails the rule and is not obviously legacy.** The measurement bucketed by
   prefix; something could be `DC-`-shaped and still malformed. Bucketing is not validating.

## 6. The negative control

Criterion 4, and it is not optional: **add a non-conforming name, watch `boundary-check` fail, remove
it.** Observed on a real run.

This gate is a control whose entire purpose is to fail when something is wrong. A control that has never
been seen to fail is the thing this project has now found twice — DC-97's G1 and DC-99's identity
comparison — and it would be a poor joke to add a rule-enforcement gate without proving it enforces.

`boundary-check` runs natively, so unlike the Windows work this costs no CI cycle: demonstrate it locally
and report the failure output verbatim.

## 7. RFC 100's addendum

Criterion 5. Append to `rfcs/accepted/100-rfc-naming-alignment.md` — do not edit its existing text, which
is a ruling and stands:

- DC-96 through DC-99 were created after its acceptance, in violation of it, by the architect.
- They are frozen under the same rule as the rest of the legacy set, for the reasons in RFC 105 §1 —
  DC-96's identifier is in a published crates.io package description, and a partial rename would leave
  three schemes.
- RFC 105 now enforces the rule that these four broke.

**Write it so a reader who meets `DC-99` dated after RFC 100 gets the answer there**, rather than
concluding the rule lapsed.

## 8. Gates

The standing set per `EXECUTION-ORDER.md` §6 rule 9, both cross-target clippy runs, and green
three-platform CI. `tools/release-policy` has its own test suite — a unit test for the new check belongs
with it, in the shape `unsafe_boundary/tests.rs` already uses.
