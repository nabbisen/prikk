# RFC 105 — RFC naming gate

**Status.** Accepted by the project owner 2026-08-17.
**Tracks.** Turning RFC 100's naming rule from a convention into a control.
**Touches.** `tools/release-policy/`, `rfcs/accepted/100-rfc-naming-alignment.md` (an addendum only —
its ruling is unchanged). No product code.

**Author-review independence.** Designed and reviewed by the same agent. Recorded rather than elided —
and this RFC exists because of an error that no second party was positioned to catch, so the note is
load-bearing rather than formulaic.

## 0. Why this exists

**RFC 100 was accepted 2026-08-11 and did not bind.** Four RFCs — DC-96, DC-97, DC-98, DC-99 — were
created after it, by the architect, in the naming scheme it retired. It went unnoticed for six days and
four increments, and was caught by the project owner reading a filename.

Four causes, and only the first is inattention:

1. **The allocation procedure derived the convention from the artifacts.** "Highest existing `DC-` is 95,
   so 96." But RFC 100 *deliberately decoupled the convention from the artifacts* — *"existing RFCs keep
   their names permanently."* The population no longer encodes the norm, by design, so reading the
   population returns the superseded norm **forever**. The procedure could not have found the rule at any
   point in the future either.
2. **The anomaly was spent as a premise.** The allocating reasoning was, verbatim, *"two numbering schemes
   coexist, so DC-96 is the next DC."* Two coexisting conventions is the fingerprint of a convention
   change; it was used as a step in the argument rather than as a question.
3. **The rule was never a control.** Every other load-bearing rule here is machine-checked — the unsafe
   boundary, dependency placement, the publication command inventory, procedure grammar. RFC 100 shipped
   as prose, and its own acceptance criteria invoke `reference-check` only for broken links.
   `unsafe_boundary.rs`'s module doc states this project's standard: *"a control the controlled party can
   silently remove is a convention, not a control."* RFC 100 was never a control, and failed exactly as
   that doc predicts.
4. **No second party was positioned to catch it.** `rfcs/` is architect-only by design; the developer
   consumes handoffs and never allocates RFC numbers. **RFC naming is a category of decision with no
   reviewer.** Every other architect error this cycle was caught by a gate, a CI run, or the developer.
   This one had none of those available.

**Cause 3 is the one that is fixable by code, and this RFC fixes it.** Cause 4 is inherent to the role
split and is mitigated only by cause 3 being closed.

## 1. What is already settled

- **RFC 100's ruling stands unchanged.** New RFCs are `NNN-slug.md` numbered from 100; existing RFCs keep
  their names permanently. This RFC adds enforcement, not a new rule.
- **DC-96 through DC-99 are frozen, not renamed.** DC-96's identifier is permanently public: it appears in
  `crates/prikk-ffi/Cargo.toml`'s `description` — the crates.io package description for a released
  version — and in that crate's README and `lib.rs`. Renaming it would make the repository disagree with
  published artifacts forever, which is precisely the anti-pattern RFC 100 cites. Renaming only 97-99
  would leave a gap in the series and three schemes instead of two, the partial migration RFC 100 argues
  against directly.
- **The next RFC number is 106.**

## 2. The obstacle, stated as a problem

The rule cannot be a pattern match alone: **116 legacy files must keep passing forever.** So the gate is
necessarily allowlist-shaped, and an allowlist raises the question DC-90 already answered for
`UNSAFE_EXEMPT_CRATES` — what stops the controlled party from simply adding an entry?

A second problem, discovered while measuring: **RFC 100's own table says 86 `DC-` files for the range
09–95, and the tree now holds a different count** because four were added to it. A gate whose frozen list
is transcribed from a document inherits that document's staleness. The list must be **derived from the
tree at the moment the gate lands**, never copied from prose — the same error one layer up.

## 3. Acceptance criteria

1. **A `boundary-check` failure for any new non-conforming name** under `rfcs/accepted/`, `rfcs/done/`,
   `rfcs/archive/`, and `rfcs/handoffs/` — files and handoff directory names alike, since RFC 100 governs
   both.
2. **The legacy allowlist is exact and derived from the tree**, not transcribed from RFC 100's table or
   from this RFC. State how it was generated.
3. **Every allowlisted name must correspond to a file that exists.** This is the self-guard: an entry
   cannot be added for a name that does not exist yet, so pre-authorising a future `DC-100` is not
   available. Adding a legacy entry alongside a new legacy file remains possible and remains a
   deliberate, visible edit to a reviewed constant — the same standard `UNSAFE_EXEMPT_CRATES` sets.
4. **Demonstrated by negative control**: a non-conforming name added, `boundary-check` watched to fail,
   the name removed. Observed, not reasoned — the bar every control in DC-97, DC-98 and DC-99 met.
5. **RFC 100 gains an addendum** recording that DC-96-99 were created after its acceptance in violation of
   it, are frozen under the same rule as the rest of the legacy set, and that RFC 105 now enforces the
   rule. Without this, a reader meeting `DC-99` dated six days after RFC 100 reasonably concludes the rule
   was abandoned.
6. Green three-platform CI.

## 4. Non-goals

- **Renaming any existing RFC**, including DC-96-99. §1 settles this.
- **Changing RFC 100's rule.** Enforcement only.
- **Checking slug quality** — that a slug is lowercase and hyphenated is checkable; that it is a *good*
  description is not, and a gate that pretends otherwise is worse than none.
- **Retrofitting a reviewer for architect-only decisions.** Cause 4 is real and out of scope; this RFC
  reduces its blast radius rather than closing it.

## 5. Staging

One stage. Report before implementing: the derived legacy inventory, and how `rfcs/handoffs/consolidation`
— a handoff directory with no RFC number at all, found while measuring — should be classified. It is
either a legacy entry to freeze or a finding; do not decide it silently.
