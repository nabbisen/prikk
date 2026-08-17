# RFC 100 — RFC naming alignment

**Status.** Accepted (2026-08-11)
**Tracks.** RFC directory hygiene. Aligns this project's RFC filenames with
[RFC-000](../done/000-rfc-lifecycle-policy.md), which the project already adopted but has not followed
in its naming.
**Touches.** `rfcs/` filenames for **new** RFCs only, `rfcs/handoffs/` directory names for new RFCs,
and `rfcs/README.md`. No existing RFC is renamed. No product code.

## Summary

RFC-000 prescribes `NNN-slug.md` — zero-padded number, lowercase hyphenated slug. This project has
instead used `DC-N-UPPERCASE-TITLE.md` and `PR-NNN-UPPERCASE-TITLE.md`. `DC` stood for *design change*,
a phase the project has left.

**New RFCs are numbered from 100 and named `NNN-slug.md`. Existing RFCs keep their names permanently.**

## The problem, measured

| Series | Range | Padding | Count |
|---|---|---|---|
| `PR-NNN` | 001–030 | three digits | 30 |
| `DC-NN` | 09–95 | two digits | 86 |
| `NNN` (RFC-000 itself) | 000 | three digits | 1 |

Three inconsistencies: two prefixes for one lifecycle, two padding widths, and — the one that
constrains the fix — **numeric duplication**. `DC-09` through `DC-30` occupy the same numbers as
`PR-009` through `PR-030`. Under RFC-000's scheme both would render as `009-…` through `030-…`.

## Why existing RFCs are not renamed

RFC-000 is explicit twice over: *"Numbers are never reused"*, and renumbering during reorganisation is
listed as an anti-pattern because *"external references — issue trackers, commit messages, Slack
history, design-review documents — all reference RFC numbers. Renumbering breaks every one of those
references silently."*

In this project those references are dense and load-bearing: `rfcs/EXECUTION-ORDER.md`'s queue,
`FINDINGS.md`'s owning-RFC column, `MILESTONES.md`, every `rfcs/handoffs/<name>/` directory, the review
records under `.git-exclude/reviewed/`, `docs/src/` reference pages, and the commit history itself.

De-prefixing the existing files is therefore not available: 009–030 would collide, and resolving the
collision requires renumbering one of the two series — the anti-pattern.

**A partial migration is worse than either end state.** Renaming only `DC-31`…`DC-95`, where nothing
collides, would leave a directory that is half-converted: three naming schemes instead of two, and no
rule a reader can apply. Freezing the legacy names gives one rule instead.

## Why 100 and not 001

`001` is taken by `PR-001`. A third series starting there would give one number three meanings and
destroy the property the numbering exists to provide — that a number identifies exactly one document,
permanently.

`100` sits above every number in use (`DC` reaches 95, `PR` reaches 30), so it collides with nothing now
and cannot collide later. **The gap at 096–099 is deliberate and worth keeping**: a visible
discontinuity marks where the scheme changed, and a reader who notices it can find this RFC.

## The rule

1. **New RFCs are `NNN-slug.md`**, numbered sequentially from `100`, zero-padded to three digits, with a
   lowercase hyphenated slug. This file is `100-rfc-naming-alignment.md`.
2. **Numbers are assigned at file creation, never reused, never renumbered** — RFC-000 unchanged.
3. **New handoffs live at `rfcs/handoffs/NNN-slug/`**, matching the RFC's filename.
4. **Existing `DC-*` and `PR-*` RFCs and handoffs keep their names permanently.** They are not renamed,
   renumbered, or migrated.
5. **A prefix means legacy; a bare number means current.** That is the whole discriminator, and it needs
   no lookup table.
6. **Folder remains the source of truth for state**, per RFC-000. The project's 5-folder variant
   (`proposed/`, `accepted/`, `done/`, `archive/`) is unchanged and already sanctioned in
   `rfcs/README.md`.

## Acceptance criteria

1. This RFC is itself named under the rule it defines.
2. `rfcs/README.md` states the rule, the 100 boundary, and the legacy-prefix discriminator, so a new
   contributor can name a file correctly without reading this RFC.
3. **`rfcs/README.md`'s index is verified against the directory** — RFC-000 requires every RFC to be
   listed and every listed RFC to exist. That has not been checked recently and this is the natural
   moment. **Report divergence rather than silently fixing it**; a stale index is a finding about
   process, not just a broken link.
4. No existing RFC file, handoff directory, or cross-reference is renamed or rewritten.
5. Release-policy `check`, `boundary-check`, and `reference-check` pass — `reference-check` reads
   documentation references and is the gate that would catch a broken link introduced here.

## Non-goals

- **Renaming, renumbering, or migrating any existing RFC.** See above; this is the anti-pattern
  RFC-000 warns about.
- **Reconciling the `DC`/`PR` duplication.** It is frozen, documented, and harmless once the
  discriminator rule exists.
- **Changing RFC content, templates, or the Status-field convention.** RFC-000 already governs those.
- **Any product, tooling, or release-lane change.**

## Addendum (2026-08-17): the rule held; it was unenforced, and now it is enforced

This RFC shipped as prose with no machine check. Between its acceptance (2026-08-11) and 2026-08-17,
four RFCs — `DC-96`, `DC-97`, `DC-98`, `DC-99` — were created by the architect in the `DC-NN` scheme
this RFC retired. It went unnoticed for six days and four increments, and was caught by the project
owner reading a filename.

**`DC-96` through `DC-99` are frozen under the rule this RFC already states**, for the same reason as
every other legacy name: `DC-96`'s identifier is published in `crates/prikk-ffi/Cargo.toml`'s
description, its README, and its `lib.rs`, so renaming it would make the repository disagree with a
released artifact permanently — the exact anti-pattern *"Why existing RFCs are not renamed"* describes.
Renaming only the unreleased three would leave a gap in the series and three schemes instead of two, the
partial migration this RFC argues against directly. **They are not an exception to this RFC; they are
four more entries in the legacy set it already anticipated, added later than the rest.**

**The next RFC number is 107.**

[RFC 105](105-rfc-naming-gate.md) turns the rule stated here into a `boundary-check` control. So a
reader meeting `DC-99` dated six days after this RFC's acceptance has the answer here rather than
inferring the wrong one: **the rule did not lapse — it was never a control, and it is one now.**
