# RFC 118 — Derive, never transcribe: binding prikk's claims about itself

**Status.** **ACCEPTED by the project owner 2026-08-24**, on the design recorded at
`.git-exclude/reviewed/claim-binding-design-v1.md`, after the owner rejected theme-by-theme work as
haphazard and asked what the core issue is. **§10's prerequisites precede design of any stage.**

**Independence.** Author-reviewed — the standing ceiling. The architect diagnosed the issue, proposed the
design, and records it; **§9 lists what that ceiling leaves unchecked.**

**Arises from.** `DC-94:20`'s own naming of the pattern, and roughly fifteen increments across the DC-78
arc that were, in substance, manual re-derivations of "does this statement match the code?" — **every one
of which found a real defect.**

**Tracks.** A structural change to how prikk states facts about itself. **No behaviour change.**

---

## 1. The issue, in the project's own words

`DC-94:20`:

> *"Today divergence between the map and what actually executes is caught by **a human reading both**.
> That is the same shape as DC-51's dependency placement and DC-90's unsafe boundary: **a rule that a
> document asserts and nothing enforces.**"*

## 2. The obvious diagnosis is wrong, and the correction matters

**prikk binds claims extensively.** `boundary-check` carries **eleven** categories over eight products —
`workspace-members`, `default-members`, `tool-metadata`, `lockfile-boundary`, `dependency-boundary`,
`dependency-placement`, `unsafe-boundary`, `rfc-naming`, `publication-allowlist`, `package-contents`,
`source-archive-contents`. **Two of DC-94's three named instances are already enforced there.** Add
`check`'s 154 oracle cases, `reference-check`, and RFC 114's byte machinery.

**This RFC is not a remedy for weak engineering.** It generalizes something this project already does
well, into the one area where it does not do it at all.

## 3. The line: what is bound, and what is not

| Claim kind | Bound |
|---|---|
| Manifest and structural | **comprehensively** — `boundary-check` |
| Byte and identity | **rigorously** — RFC 114, Gate A, literal vectors |
| Prose about `release-policy`'s own invocation | **narrowly** — `reference-check`, three required paths |
| **Prose about prikk itself** | **not at all** |

**Every documentation defect found in the DC-78 arc lived in the last row.**

## 4. The principle is already this project's, from RFC 105

`boundary/rfc_naming.rs:27` records how its own exemption lists were built:

> **"RFC 105 design-v1.md §3: derive, never transcribe."**

**RFC 118 is that principle applied to prikk's claims about itself.**

## 5. The proven pattern, at miniature scale

`admitted_schemas` + Gate A, completed at `f1528b8`:

1. **One declaration of truth in code** — `admitted_schemas(object_type)`.
2. **Consumers call it**, never restate it. The schema-1 hardcodes in `wal.rs`, `store_resolvers.rs` and
   `replay.rs` were **defects precisely because they were transcriptions**, and were replaced by calls.
3. **A completeness gate over the declaration**, at the granularity the contract is defined at —
   observed failing in both directions.

**Every stage below is this shape.**

## 6. Why checking is not the design

**Checking that copies agree preserves the copies. Every surviving copy is a future divergence.**

A checker comparing four restatements of the command surface makes drift *loud*; it does not make it
*impossible*, and it commits the project to maintaining four copies permanently, now gated.

**Where a fact can be derived, it must be derived. Checking is for the join with authored prose (§8),
not for facts.**

## 7. The worst instance, and the first stage

**One fact, four statements, zero derivations:** 24 dispatch arms in `main.rs` (the *de facto*
authority), 48 lines in `help.rs`, 43 in `README.md`'s Useful Commands, plus guide pages.

**This single duplication produced four DC-78 increments** — the staleness-by-omission sweep, `help.rs`'s
missing `sync tags`/`adopt-tag`, `main.rs`'s stale module inventory, and `README.md`'s stale command
list. **None of them fixed the cause.**

**Design:** a command registry as data; **dispatch derives from it**; `--help` becomes a renderer;
enumerating documentation is generated. **`help.rs` then cannot go stale, because it holds nothing of
its own.**

## 8. Facts derive, judgment is authored, the join is gated

A registry can state that `sync adopt-tag` exists and takes one positional argument. **It cannot state
why adoption is a separate act, or that it never gates on the sender's signature** — that is authored,
and it is the most valuable content in the tree.

**The gate is bidirectional over the join**, the same rule `DC-94:51` asks for its own map:

- **Every registry entry is explained somewhere** — an unexplained command becomes *countable*, not
  discovered.
- **Every explanation names a real entry** — prose naming a command that does not exist fails.

**This closes the meta-gap for one domain**: today *nothing enumerates which claims are unbound*, which
is why instances are found by a human reading both. **A registry enumerates what must be bound.**

## 9. Where this stops, deliberately

**Judgment is not gatable, and pretending otherwise would repeat an overstatement this project has
already had to withdraw** (`MILESTONES` criterion 2, corrected 2026-08-24).

The three false roadmap themes failed **not because no gate read them**, but because nobody re-read them
against a changed world — no gate could determine that DC-75 refuted DC-74's premise. **Judgment stays
authored, dated, and reviewable.**

**The author-review ceiling applies to this RFC itself:** the architect diagnosed the issue, proposed the
design, and accepted its framing. **§10's prerequisites exist partly to have someone else test it.**

## 10. Blocking prerequisites, before any stage is designed

1. **Is dispatch genuinely derivable?** `main.rs` dispatches by `match` on `&str`. **Confirm a registry
   can drive it without losing per-command argument shapes**, and say what it costs if not.
2. **What is the authority for "explained somewhere" (§8)?** `reference-check`'s inventory-plus-scanner
   is the precedent; **confirm it generalizes, or name what replaces it.**
3. **Which enumerations are in scope beyond commands?** Candidates, ranked by demonstrated harm:
   **trust-gated operations** (hand-derived twice this session, wrong once) and **`verify`'s stage
   inventory** (which makes a machine-readable `verify` result a *derived view*, dissolving the
   structured-output theme rather than adding to it).
4. **Does generation conflict with the zero-dependency CLI?** `prikk-cli` has **no third-party
   dependencies**. Generation must not import one, or must be build-time.

## 11. Non-goals

- **No behaviour change.** Every command keeps working identically; **the existing 1317 tests are the
  control.**
- **Not a documentation rewrite.** Authored prose stays authored.
- **Not `boundary-check`'s replacement.** The eleven categories stand.
- **Not RFC 108, peer trust, quarantine, or repository identity** — those wait on their own rulings, and
  this RFC does not resolve them.
