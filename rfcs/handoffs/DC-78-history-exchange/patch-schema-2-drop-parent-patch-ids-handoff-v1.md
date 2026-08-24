# Delete `parent_patch_ids` at a new `Patch` schema version

**Base:** current `main` (`dc9a2ba`). **Under `003-landing-work-on-main.md`.**
**Owner-authorized 2026-08-24.** **Origin:**
`.git-exclude/reviewed/cluster-a-dag-or-chain-investigation-v1.md`.

**A behaviour change to a frozen surface — RFC 114 governs it, and it is done the way RFC 114 says.**
Read §2 before touching anything.

---

## 1. The decision, and why deleting loses nothing

`PatchPayload.parent_patch_ids` (`payload/patch.rs:57`) is **inert**: `Vec::new()` at every production
construction site, and `patch_exchange/accept.rs:184-187` **refuses** a non-empty value. DC-74:78 gave
it a purpose — *"a patch binds to `parent_patch_ids`... its context is a dependency set, not a
snapshot"* — **which was never implemented**, and DC-75:26 established there is no patch DAG.

**The ruling: keeping the field is not keeping the capability.** A real patch DAG needs a schema version
regardless, to define validation, ordering, and what a receiver checks on arrival — none of which the
field carries. **Deleting it does not delete the option.**

## 2. RFC 114 is the governing contract, and this is the shape it prescribes

**Schema 1's canonical encoding is frozen forever.** Every patch already written stays schema 1 and
**must keep decoding and verifying unchanged.** This increment **adds** a pair; it moves nothing.

**The precedent is `RefState`, and it is exact.** `payload/refs.rs:39` declares
`REF_STATE_CLOSED_SCHEMA: u32 = 2`; `decode_canonical(bytes, schema_version)` takes the version as a
**parameter** and at `:87-90` refuses the `closed` field when `schema_version < REF_STATE_CLOSED_SCHEMA`.
`validate_format2_schema` admits `RefState => &[1, REF_STATE_CLOSED_SCHEMA]`.

**Mirror it, inverted:** a named constant for the new Patch schema, and a refusal of **field 2** at that
schema **and above** — the opposite direction from `RefState`, which refuses a field *below* its
threshold. **Say in your report that you checked the direction.**

**`PatchPayload::decode_canonical` currently takes no `schema_version`.** Giving it one is a signature
change with callers. **Enumerate them and report the list.**

## 3. Gate A will fail, and that is the tripwire working

`rfc114_gate_a_every_admitted_pair_is_frozen_or_declared_unwritten`
(`signature_contract_tests/vectors.rs:389`) is **self-enforcing**: it fails the moment
`validate_format2_schema` admits a pair with no literal identity vector.

**So admitting `(Patch, 2)` will fail it until you add a vector.** **That is correct behaviour, not an
obstacle** — RFC 114 built it for exactly this. **Add the literal `(Patch, 2)` vector**, following the
existing vectors' shape.

**Do not weaken, skip, or special-case that test.** If it seems to demand something impossible, **stop
and report** — it is the mechanism that makes the frozen surface real.

## 4. The work

1. **A named schema constant**, following `REF_STATE_CLOSED_SCHEMA`'s form.
2. **`validate_format2_schema`**: `Patch => &[1, <new>]`.
3. **Encode**: schema 2 omits field 2. **Decode**: schema 1 accepts it (and must — old patches exist);
   schema 2 **refuses** it.
4. **A literal identity vector for `(Patch, 2)`** (§3).
5. **Every production construction site writes schema 2.**
6. **`accept.rs:184-187`'s refusal** — keep it for schema 1; unreachable for schema 2. **Adjudicate and
   say which**; do not delete a refusal because one schema makes it moot.
7. **Field number 2 is retired, not reused.** Say so in the payload's own doc so nothing claims it later.

## 5. Tests — the load-bearing ones are about schema 1, not schema 2

**The risk here is not that new patches break. It is that old ones do.**

1. **A schema-1 patch with field 2 present still decodes and verifies.** If no such fixture exists,
   **build one** — this is the property RFC 114 exists to protect.
2. **A schema-2 patch carrying field 2 is refused.**
3. **A repository holding both schema-1 and schema-2 patches verifies.** `RefState` already proves the
   pattern works; prove it for `Patch`.
4. **Negative controls** on (2) and (3), keeping the call and neutralising its effect. **Report observed
   output.**

## 6. Out of scope

- **Patch aggregation** — a separate owner ruling.
- **Designing a patch DAG.** This increment *removes* an unpopulated field. **If you find a reason a DAG
  needs the field kept, stop and report** rather than halting the deletion on your own reading.
- **`CURRENT_FORMAT_VERSION`.** This is an object schema version, not a repository format version.
  **If you find yourself needing to bump the format, stop** — that is a different contract with its own
  migration tripwire.
- **`MILESTONES.md`, `ROADMAP.md`, the badge.**

## 7. What to report

1. **The constant, and the refusal direction you checked** (§2).
2. **Every caller of `PatchPayload::decode_canonical`**, and what you did to each.
3. **The `(Patch, 2)` vector**, and confirmation Gate A failed before you added it — **quote the
   failure.** A Gate A that never failed means the pair was not really admitted.
4. **§4 item 6's adjudication.**
5. **All four §5 tests**, with negative-control output, and **whether a schema-1-with-field-2 fixture
   already existed or you built one.**
6. **Full gate set against the exact commit, after the last edit.** **Test counts will change.**
7. Anything here was wrong, **including my line numbers and the `RefState` precedent reading.**

**Stop and escalate, do not guess**, if: Gate A demands something the frozen-vector shape cannot express;
a schema-1 patch cannot be made to decode unchanged; the format version appears to need bumping (§6); or
**old patches turn out not to be distinguishable by schema at the point decode happens** — that last one
would mean the dual-schema approach does not work here, and it is the finding I would most want.
