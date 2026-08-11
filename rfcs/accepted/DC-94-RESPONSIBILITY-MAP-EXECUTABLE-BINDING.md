# RFC (proposed) - DC-94 Responsibility Map Executable Binding

**Status.** **ACCEPTED by the project owner 2026-08-11.** **Independent of DC-93** — it gates nothing
and nothing gates it. §3's four prerequisites precede design.
**Independence.** Author-reviewed — the standing ceiling.
**Arises from.** DC-52's obligations 1 and 2, decoupled from its retirement obligations on 2026-08-11
so that removing complexity is not held hostage to adding a check.
**Target milestone.** M2.

## 1. The two obligations, and why they stand on their own

DC-45 left two assurance obligations recorded only as prose. They were written as preconditions for
retiring the Python; **that coupling is withdrawn** (DC-93 §2). What remains is whether each is worth
doing on its own merits. The architect's view is that the first clearly is and the second probably is.

**1 — The responsibility map is not mechanically bound.** A 50-entry map relates release-policy
responsibilities to the checks that discharge them. Today divergence between the map and what actually
executes is caught by a human reading both. That is the same shape as DC-51's dependency placement and
DC-90's unsafe boundary: **a rule that a document asserts and nothing enforces.** Binding it so that a
map entry with no executed check — or an executed check with no map entry — **fails closed** converts
prose into a checked property.

**2 — `defaults.run` rests on an assumed invariant.** The governed-procedure YAML extractor skips an
empty `run` value whose parent key is `defaults`, relying on the GitHub Actions schema forbidding an
executable scalar there. The assumption is correct and **unenforced**; architect review v11 of DC-45
required it be made explicit. Validating that the block under `defaults.run` contains only known
configuration keys (`shell`, `working-directory`) turns a schema assumption into an enforced rule and
protects the exception against future extractor changes.

## 2. What already exists, and why that matters before anything is built

`tools/release-policy/src/oracle/self_test/responsibility.rs` (55 lines) already loads
`tools/release-policy/self-test-responsibility-map-v1.json` with `deny_unknown_fields`. **Some binding
machinery is therefore already present**, and how much is the first thing to establish — this increment
may be substantially smaller than DC-52's framing implied, or already partly discharged.

## 3. Blocking prerequisites

1. **What does `responsibility.rs` already enforce?** Read it and its self-test. Does it validate map
   *shape* only, or does it already relate entries to executed checks? State precisely what is
   currently checked and what is not — the gap is the increment.
2. **What is "an executed check registry"?** There must be something enumerable to bind against.
   Report whether one exists, can be derived from the existing check dispatch, or would have to be
   introduced — and if introduced, what keeps *it* from drifting, since a registry nothing verifies is
   the problem restated one level up.
3. **Is bidirectional failure the right rule?** Map-entry-without-check and check-without-map-entry may
   not be equally wrong. Report whether both directions should fail closed, or only one, with the
   reason.
4. **Cost of the `defaults.run` validator**, and whether tightening it can reject any procedure the
   project legitimately uses today. A validator that fails closed on a valid workflow is worse than the
   assumption it replaces.

## 4. Acceptance criteria

1. §3 answered and reported before design.
2. **Negative controls per rule** — introduce a map entry with no executed check, and an executed check
   with no map entry, and show the specific check firing for each. Per DC-86's and DC-90's precedent.
3. **The `defaults.run` validator accepts every procedure currently in the tree** and rejects an
   unknown key, demonstrated both ways.
4. **What the binding cannot see is stated plainly**, in the tool's own documentation. A map entry can
   correspond to a check that exists and does the wrong thing; this binds existence, not correctness,
   and a passing check must not be read as more than that. DC-90's criterion 5 standard.
5. Gate set per `EXECUTION-ORDER.md` §6 rule 9.

## 5. If the answer is "already done"

§3.1 may find the binding largely exists. **That is a complete and useful outcome** — record what is
enforced, close the DC-45 obligation explicitly rather than leaving it as prose, and end the increment.
Discovering that an obligation was already discharged is worth the investigation on its own, and is
strictly better than building a second mechanism beside a working one.

## 6. Non-goals

- **Anything in DC-93.** No Python is retired here, and this does not gate that.
- **Any change to what the checks themselves verify** — this binds the map to them, it does not alter
  them.
- **Any new dependency.** The tool parses TOML and JSON today; this needs no more.
- **Any release-lane, signer, or publication action.**
