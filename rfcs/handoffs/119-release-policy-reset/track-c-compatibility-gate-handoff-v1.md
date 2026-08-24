# RFC 119 track C — G1, the compatibility gate

**Base:** current `main`. **Under `003-landing-work-on-main.md`.**
**RFC:** `rfcs/accepted/119-release-policy-reset.md` §10 track C — **owner-ordered first, 2026-08-25.**

**The largest gap the reset found: nothing detects a release breaking what earlier releases wrote.**
`0.23.0` shipped exactly that, and only the *declaration* half held — by hand, in the notes.

---

## 1. What must be detected

**G1, pre-1.0 form: a release does not *silently* break what earlier releases wrote.**

**Breaking is permitted.** `0.23.0`'s `TagPayload` amendment was authorized deliberately, on the stated
basis that no production history exists. **The guarantee is against silence, not against breaking** — so
the outcome is ternary, and only the last fails:

- **compatible** — a repository written by the last release still opens and verifies;
- **breaking, declared** — it does not, and the break is declared with what it affects and the remedy;
- **breaking, undeclared** — **the only failure.**

**A gate that simply forbade breaks would have blocked a change the owner authorized. Do not build
that.**

## 2. Where it lives, and the sibling it extends

**`prikk-store`, beside `format_stability_gate.rs`** — not in `release-policy`.

That file guards **format version bumps** and states its own principle: *"the watcher does not live
inside the thing it watches."* **`0.23.0` went through the hole next to it**: an in-place payload
amendment at the same schema version, for which no format bump occurs, so Gate B never fires.

**This is that file's missing sibling. Read it first** — its three-layer structure, and its rule that
*"a gate is only trusted once it has been observed failing."*

## 3. The mechanism: a frozen repository fixture from the last release

**The precedent is `crates/prikk-cli/tests/fixtures/dc55_pre_swap_repo`** — a real repository whose
bytes were written by an older version, committed, and carrying its own instruction: **"Do not
regenerate the fixture. It is frozen."**

**Build the same thing for the last released version**: a repository created by `0.23.0`, committed as
bytes, opened and verified by current code.

**Do not generate it with current code.** A fixture produced by the code under test proves nothing —
the same trap as a control that never reaches its target, which this project has hit twice on Gate A
alone.

## 4. Fixture coverage decides what the gate can catch — the load-bearing design decision

**A fixture that omits an object type cannot detect a break in that type.** `0.23.0` broke `Tag`.
**A fixture without a tag would have caught nothing.**

**Seven persisted object types**: `Block`, `Patch`, `Blob`, `RefState`, `Tag`, `RecognitionClaim`,
`Attestation`.

**The fixture must exercise every type that release could produce.** For each type it cannot contain,
**say why in the report** — `Attestation` is never constructed in production, so its absence is a fact
about the product, not a gap in the fixture. **`RecognitionClaim` requires a sync; decide whether the
fixture covers it or whether that is a stated limitation.**

**Report coverage as a table.** It is the gate's real specification.

## 5. Declared breaks

**A committed list, with a reason and a remedy per entry** — the shape of Gate A's `frozen` /
`RFC114_ADMITTED_BUT_UNWRITTEN` pair, and of `FORMATS_WITH_MIGRATION_COVERAGE`.

**Seed it with `0.23.0`'s Tag break**, since the current release already carries one: a `0.22.1`
repository holding a tag cannot be repaired under `0.23.0`, and the remedy is to keep using `0.22.1` for
it or start fresh. **That wording already exists in `CHANGELOG.md` — cite it, do not re-derive it.**

**An entry must state a remedy or say plainly that none exists.** A break with an empty remedy is a
documentation defect, not a passing gate.

## 6. Out of scope

- **The reverse direction** — an *old* binary reading *new* data. Real (`0.23.0` broke both ways) but it
  needs the old binary, not just its output. **Report whether you see a way; do not build it here.**
- **Post-1.0 G1** — prevention rather than declaration. Not now.
- **`release-policy`, `boundary-check`, the oracle.** Tracks A and B; untouched.
- **Changing any product behaviour.**

## 7. Controls

1. **The gate fires on an undeclared break**: temporarily remove `0.23.0`'s entry from the declared list;
   observe the failure name the object type and the fixture; restore.
2. **The gate passes with the break declared** — the seeded entry.
3. **The fixture is genuinely old**: confirm it was not produced by current code, and say how.
4. **Coverage is load-bearing**: remove one object type from the fixture (or from whatever the gate
   walks) and confirm the coverage assertion notices. **If it does not, the coverage table is
   decoration** — report that rather than working around it.

**Quote every failure.**

## 8. What to report

1. **The fixture**, how it was produced, and the **coverage table** (§4) with a reason per absent type.
2. **The declared-breaks list**, and `0.23.0`'s entry.
3. **All four controls, quoted** (§7).
4. **Whether the reverse direction looks feasible** (§6) — report only.
5. **Full gate set against the exact commit, after the last edit.** Test counts rise.
6. Anything here that was wrong, **including my seven-type list**.

**Stop and escalate, do not guess**, if: a `0.23.0` fixture cannot be produced without current code
(§3); the ternary outcome cannot be expressed because the declaration is not machine-readable — **say so
rather than inventing a format**; or you find the current release breaks something **not** declared in
`CHANGELOG.md` — **that is a live defect and stops this increment.**
