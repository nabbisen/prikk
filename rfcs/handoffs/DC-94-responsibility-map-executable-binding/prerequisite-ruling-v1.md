# DC-94 — Prerequisite Investigation Ruling v1

**Reviewing:** `.git-exclude/review-request/prikk-dc-94-prerequisite-questions-v1.md`.

**Accepted. Cleared to design.** Not "already done" — §5's easy outcome does not apply, and they said so
rather than reaching for it.

**They corrected what the map actually is (§2), and that correction has a consequence for DC-93 that
neither report connects (§3).**

## 1. Verified

- **`responsibility.rs` validates map shape only** — schema version, exactly 50 entries, non-empty
  strings, pairwise uniqueness in both columns. Nothing outside the file is read. Confirmed.
- **No test exercises `responsibility::verify` at all.** `oracle/self_test/tests.rs` has zero references
  to it. Confirmed — the only thing exercising it is the real map passing its own checks, which proves
  the map is well-formed today, not that malformation would be caught.
- **`defaults:` appears exactly once in the governed corpus** — `.github/workflows/docs.yml:26`, using
  only `working-directory`. Confirmed. My first grep used too rigid a pattern and found nothing; I
  re-ran it looser rather than contradicting them on a grep artifact, having made precisely that mistake
  in DC-93 an hour ago.

## 2. Correction: the map is not what my RFC said it was

DC-94 §1 describes a "50-entry map relating release-policy **responsibilities** to the **checks that
discharge them**" — which reads as a governance mapping, an NFR to its enforcing check.

**It is not.** The entries are `{python_check, rust_check}` pairs:

```json
{"python_check": "pack-malformed",     "rust_check": "profile:malformed"}
{"python_check": "pack-duplicate-name","rust_check": "profile:duplicate-name"}
```

**It is self-test mutation-category correspondence between the retired Python harness and the Rust
one** — narrower and more specific than my framing, and they cross-checked it against the literal string
arguments at the Rust self-test call sites rather than inferring from names.

**Confirmed: their reading is the correct one, and it is the intended scope.** DC-45 retained the Python
as a correctness baseline; this map records which Python self-test category each Rust one replaced.
Binding it means proving **the Rust harness still runs a case for every category the Python harness
covered** — a migration-completeness guard, not a policy-to-check map.

They asked for confirmation because it changes what "binding" concretely means. It does, and asking was
right.

## 3. The consequence neither report draws: DC-93 changes what this map means

**DC-93 retires the Python.** After it lands, `python_check` names categories in a harness that no longer
exists in the tree.

That does **not** make the map worthless — the opposite. It becomes the only record of what the retired
implementation covered, and the binding becomes the guard that retiring it lost nothing. **That is a
better argument for DC-94 than the one in its own RFC**, and it makes the two increments complementary
rather than merely independent.

**But it has an ordering implication I am ruling on now:** the map's `python_check` column must be
understood as **historical by design** once DC-93 lands, and the binding must be built against
`rust_check` only. A future reader finding a column naming files that no longer exist will otherwise
"clean it up." **Say so in the map's own schema documentation when the binding is built** — that column
is evidence, not a live reference.

Neither increment's scope changes. DC-93 does not wait for DC-94, and DC-94 does not wait for DC-93.

## 4. §3.2's self-registration answer

I asked what keeps a new registry from drifting, saying I would rather hear the objection from them than
find it at review. **They answered it rather than deferring it**, and the answer is right: a separately
maintained list of expected names is the original problem restated one layer up. Threading a shared
collection through the existing call sites — each already receives its `name: &str` — makes the registry
**a record of what this run executed rather than a claim about what should exist**. It cannot drift from
reality because reality populates it.

**Endorsed as the shape to design toward**, with their own caveat intact: it is an observation about a
shape, and the threading is implementation work whose cost is not yet established. If it does not survive
contact with the three call-site patterns, report that.

## 5. §3.3 and §3.4

**Bidirectional failure: accepted**, and on better grounds than my RFC gave. They did not defer to my
proposition; they showed both directions are independently plausible *because* case names are typed at
scattered call sites rather than selected from a checked list. The failure surface justifies the rule.

**`defaults.run`: cleared.** One real occurrence, using one of the two allowed keys, with room to spare.
And their point that the validator changes what is tolerated *inside* the skipped block rather than
whether the skip fires is the distinction that makes this additive and safe.

## 6. A finding they surfaced beyond the ask

**The pre-existing shape checks have no negative control either.** Acceptance criterion 2 therefore
covers more than I wrote it to cover: it must prove the *existing* shape checks fire (wrong count,
duplicate `rust_check`, empty string) as well as the new binding. They identified this as the same shape
as DC-95's Finding A in a different corner of the tool, which is exactly the right connection.

**Criterion 2 is amended accordingly** — negative controls for the pre-existing checks and the new
binding, not the new binding alone.

## 7. Standing

- **DC-94: cleared to design**, under §4's shape and §6's widened criterion 2.
- Touches no product code; an ordinary CI run suffices.
- DC-93 and DC-95 are ruled separately; all three remain independent.
