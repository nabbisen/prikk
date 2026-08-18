# RFC 105's naming gate — a legacy RFC cannot change lifecycle directory without a tool change

**Raised by the architect 2026-08-18**, on hitting it. **Small, self-contained, and blocking a
docs-only action the project owner has already authorized.**

## 1. What happened

The owner authorized moving DC-53's RFC from `rfcs/proposed/` to `rfcs/done/` after it completed. The
move is correct and mechanical. **`boundary-check` refuses it, in both directions at once:**

```
rfcs/done/DC-53-...md: does not conform and is not in the legacy allowlist
rfcs/proposed/DC-53-...md: allowlisted but does not exist
```

The allowlists are **per-directory** (`RFC_PROPOSED_LEGACY`, `RFC_ACCEPTED_LEGACY`, `RFC_DONE_LEGACY`,
…), and the self-guard requires every allowlisted name to exist **at that specific location**. So moving
a legacy `DC-`/`PR-` file between lifecycle directories fails the gate until two lines in
`tools/release-policy/src/boundary/rfc_naming.rs` change with it.

**I cannot make that change** — nothing under `tools/` is the architect's to write — and the move is
mine. So a routine lifecycle transition now needs a round trip between us.

## 2. Why this is a defect in the gate, not merely friction

**The rule the gate exists to enforce is about names.** RFC 100 froze legacy `DC-*`/`PR-*` **filenames**;
it said nothing about which lifecycle directory they may sit in. Moving a completed RFC to `done/` is
the normal, expected operation the lifecycle is built around.

**As written, the gate treats a legitimate lifecycle move as a naming violation.** That is a category
error, and it will recur for every legacy RFC that completes — there are still six under `proposed/`.

**And it lands on the wrong person.** The gate makes the architect's own documented duty — keeping the
RFC record current — require a change to a crate the architect may not touch. **Staleness in the RFC
record is exactly what this gate's sibling rules exist to prevent**, and this one now taxes fixing it.

## 3. The fix I am ruling

**A legacy name is allowlisted if it appears in any of the five lists; the self-guard requires it to
exist somewhere under `rfcs/`, not at one specific path.**

- **Keeps the inventory.** The five lists still record where each legacy record was, and adding a new
  legacy name is still a deliberate, visible edit.
- **Keeps the anti-reservation guard.** "Reserve `DC-100` before it exists" still fails, because the name
  must exist *somewhere*.
- **Keeps the file-versus-directory distinction.** A file-form entry must still be matched by a file and
  a `handoffs/` directory entry by a directory — that check is about form, not location, and must not be
  weakened.
- **Removes the category error.** A move between lifecycle directories stops being a naming failure,
  which is what RFC 100 actually says.

**Update the module doc in the same change.** Its current text states the self-guard as *"exists at that
specific location"* and explains why — that reasoning is what this ruling changes, and a doc that
survives the code it describes is the staleness pattern this project keeps finding.

## 4. Scope

- **Only** `tools/release-policy/src/boundary/rfc_naming.rs` and its tests.
- **No RFC file moves in this change** — the DC-53 move is mine and follows once the gate allows it.
- The gate must still fail on a genuinely non-conforming **new** name in any governed location. **Add a
  test proving that**, so this relaxation cannot be read as weakening the rule.

## 5. Report before implementing

Per the standing shape. **If you conclude the current per-location behaviour is deliberate and worth
keeping**, say so with the reasoning rather than implementing this — I ruled it a defect, and I would
rather hear a case against it than have it built because I asked.
