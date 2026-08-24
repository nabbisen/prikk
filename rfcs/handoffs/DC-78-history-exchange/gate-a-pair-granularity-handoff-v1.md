# Gate A — make the RFC 114 completeness guard pair-granular

**Base:** current `main`. **Under `003-landing-work-on-main.md`.**
**Origin:** found by the dev team while implementing `(Patch, 2)` (`8c31a78`) — **Gate A did not fail
when both my handoffs said it would.**

**This is a verification gate. Read §3 before writing the fix, and §4 before believing it works.**

---

## 1. What is wrong

`rfc114_gate_a_every_admitted_pair_is_frozen_or_declared_unwritten`
(`signature_contract_tests/vectors.rs:389`):

```rust
for &object_type in ALL_OBJECT_TYPES {
    for schema_version in 0u32..=8 {
        if validate_format2_schema(&envelope).is_ok() {
            assert!(frozen.contains(&object_type)
                 || RFC114_ADMITTED_BUT_UNWRITTEN.contains(&object_type), …)
```

**The loop iterates pairs. The assertion reads only `object_type`.** `schema_version` appears **solely
in the failure message.** So once a type has one vector, **every future schema of that type passes for
free.**

**Both lists are `ObjectType`-granular:** `frozen: [ObjectType; N]` and
`RFC114_ADMITTED_BUT_UNWRITTEN: &[ObjectType]` — the latter despite its own doc at `:338` saying
*"**Pairs** `validate_format2_schema` admits…"*. And `:340` claims the guard *"checks every admitted
pair is either frozen…"*. **The docs describe the intended design; the code implements a weaker one.**

**This is pre-existing.** `RefState` has admitted two schemas since DC-61 with one `frozen` entry. It
was never exercised until a second type gained a second schema.

## 2. What is NOT wrong — do not over-correct

**There is no live gap today.** Every admitted pair has a literal vector — `(Block,2)`, `(RefState,1)`,
`(RefState,2)`, `(Patch,1)`, `(Patch,2)`, `(RefUpdate,1)`, `(Tag,1)`, `(Blob,1)`,
`(RecognitionClaim,1)` — and `Attestation` is declared unwritten.

**So this is preventive.** **Do not go looking for a missing vector to add; there isn't one.** If you
believe you have found one, **stop and report** — that would be a much larger finding than this
increment.

## 3. The fix

**Make both lists pair-granular** — `(ObjectType, u32)` — and make the assertion check the **pair**.

**`RFC114_ADMITTED_BUT_UNWRITTEN` too.** It is the escape hatch; leaving it type-granular means a new
schema on `Attestation` would still slip through. **Its own doc already says "pairs" — make the code
match the doc, not the other way round.**

**Use `admitted_schemas` as the enumeration source** where that is natural (`8c31a78` introduced it) —
**do not hand-maintain a third list of what is admitted.**

**Correct `:338` and `:340`'s doc comments** only where they now overstate or understate what the code
does. **They are currently correct as statements of intent; after this they become correct as statements
of fact.**

## 4. The control is the whole increment

**A guard that has never been observed failing is not a guard.** This one passed while being wrong, for
the entire life of `RefState`'s second schema.

**Required: admit a pair that has no vector, observe Gate A fail, then revert.** For example, add a
throwaway schema to an already-vectored type in `admitted_schemas`, run Gate A, **quote the failure**,
and restore.

**Confirm the failure message names the schema version**, not just the type — the current message
interpolates `{schema_version}` but the predicate ignores it, and after this fix the two must agree.

**Also confirm Gate A still passes unmodified**, so the fix has not simply broken it.

**Do not commit with the throwaway pair in place.** `git status` clean before the final gate run.

## 5. Out of scope

- **Adding or changing any identity vector.** §2.
- **`validate_format2_schema`'s admitted set.** Read it; do not change it.
- **The `CURRENT_FORMAT_VERSION` migration tripwire** — a different RFC 114 mechanism, untouched here.
- **`MILESTONES.md`** — criterion 2's overstated claim is **already corrected by me**.

## 6. What to report

1. **The new pair-granular predicate**, and both list types.
2. **The control** (§4): the throwaway pair used, **the quoted failure**, and confirmation the tree was
   clean afterwards.
3. **Confirmation the failure message names the schema version.**
4. **Confirmation no vector was added or changed** (§2).
5. **Full gate set against the exact commit, after the last edit.** Test counts — **expected unchanged**.
6. Anything here that was wrong.

**Stop and escalate, do not guess**, if: making the guard pair-granular reveals an admitted pair with no
vector (§2) — **that is the finding I would most want and must not be quietly fixed by adding a
vector**; or `RFC114_ADMITTED_BUT_UNWRITTEN` turns out to need type granularity for a reason the code
does not state.
