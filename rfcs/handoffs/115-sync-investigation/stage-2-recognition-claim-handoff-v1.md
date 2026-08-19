# RFC 115 Stage 2 — the recognition claim object: implementation handoff

**RFC:** `rfcs/accepted/115-sync-investigation.md` (ACCEPTED 2026-08-19).
**Design:** `rfcs/handoffs/115-sync-investigation/design-v1.md` — **D3 (§4) and §8 are the
governing sections. Read both in full before starting.** This handoff does not restate them; it
resolves every decision they left open and adds the ones found since.
**Follows:** Stage 1 (`321774d`-adjacent branch `rfc-115-stage-1-patch-set-digest`, accepted).
**Precedes:** Stage 3 (the exchange artifact and accept path). **Nothing here puts bytes on a wire.**

Every architectural decision below is **ruled, not open**. If something is genuinely undecidable
from this document, that is my defect — escalate it rather than choose, per the standing rule that
a handoff must not require inventing architecture.

---

## 0. What is settled, and must not be re-opened

From the design, restated only as boundaries:

1. **Recognition is its own object type**, not `Attestation` and not an extension of it (D3). The
   reason is concrete and worth carrying: shipping Blocks as evidence fails because
   `validate_v2_lineage` errors `"format-2 parent Block {id} is missing"`
   (`block_state.rs:446-447`), so block-as-evidence either drags the whole lineage — block-level
   exchange again — or breaks the receiver's `verify`.
2. **A recognition claim is never trust-conferring.** It names a `key_id` the receiver may not have
   adopted. It is reportable, never gating (§8.3).
3. **Selective omission is unsolvable and is documented, not defended against.** A sender may
   truthfully sign claims about some patches and stay silent about others. Every byte verifies and
   the receiver is still not told everything. Say so in the module doc; do not build a countermeasure.
4. **This is a new frozen `(object_type, schema_version)` pair the moment it is first written**, so
   its Gate A vector lands **in this same increment** (RFC 114 §4). Non-negotiable.

---

## 1. What already exists — build on it, do not rebuild it

Read these before writing anything. Every one of them is the thing you should be extending.

| Surface | Where | Why it matters here |
|---|---|---|
| `ObjectType` enum | `prikk-object/src/id.rs:15-39` | Codes are `u16` and go **into the object-id preimage**. Frozen on first write. |
| `Signature::signed_bytes` | `prikk-object/src/signature.rs:120-144` | `SIGNATURE_DOMAIN ‖ algorithm ‖ object_type ‖ object_id ‖ signer_role ‖ key_id_len ‖ key_id`. |
| `maintainer_signature` | `prikk-store/src/maintainer_signing.rs:21-44` | The production signing path. Use it. Not `test_support`'s dummy. |
| `TagPayload` | `prikk-object/src/payload/tag.rs` | The shape model for a small canonical payload: `encode_canonical` + a field-tag cursor decoder that **rejects unknown tags**. |
| Identity snapshot | `prikk-object/src/vectors/snapshot.txt` + `snapshot.rs` | **This is Gate A.** Columns: `name\|type_code\|schema_version\|payload_hex\|object_id_hex`. |
| Format admission | `prikk-store/src/format.rs:22-45` | Decides which `(type, schema)` pairs may exist as stored objects at all. |
| Object verification | `prikk-store/src/verify/objects.rs:273-315` | Where a stored object's signatures get classified, and where the `Block \| RefState` trust gate lives. |

**A new variant is compiler-forced at these sites** — treat the list as your checklist, not a
surprise: `prikk-object/src/id.rs` (`from_code`, `name`), `prikk-object/src/vectors.rs`,
`prikk-object/src/vectors/hard.rs`, `prikk-store/src/format.rs`, `prikk-store/src/layout.rs:855-870`,
`prikk-store/src/file_codec/tests.rs`, `prikk-store/src/format/tests.rs`,
`prikk-store/src/signature_contract_tests/vectors.rs`.

---

## 2. The object — every field ruled

### 2.1 Identity

- **Variant:** `ObjectType::RecognitionClaim = 0x0B`. Next free code; `0x0A` is `ProjectGenesis`.
- **`name()`:** `"recognition-claim"`.
- **`schema_version`:** `1`.

### 2.2 Payload

```
RecognitionClaimPayload {
    block_id:  ObjectId,       // field tag 1 — the block the claim is about
    patch_ids: Vec<ObjectId>,  // field tag 2, repeated — sorted, deduplicated, non-empty
}
```

That is the whole payload. Three deliberate omissions, each with its reason, because each will
otherwise be "helpfully" added back:

- **No claimer `key_id` field.** The signature preimage already binds `key_id`
  (`signature.rs:141-143`), and duplicating it into the payload creates a second source of truth and
  a mandatory cross-check whose failure mode nobody has thought through. Keeping it out has a
  further benefit: two senders making the *same* claim produce the *same* object id, so the claim
  is one object carrying two signatures rather than two near-identical objects. `ObjectEnvelope`
  already supports multiple signatures.
  *(`TagPayload` does carry `author_key_id`; that is precedent I am deliberately not following, and
  this paragraph is the reason. Do not "fix" the inconsistency.)*
- **No timestamp.** This project has no trusted clock. `TagPayload.created_at` and
  `RefUpdatePayload.created_at` carry a zero sentinel only because they predate that ruling; a new
  type should not inherit a legacy placeholder. Omit the field entirely.
- **No `project_id` / genesis binding.** It would look like useful cross-project replay protection
  and is not: block and patch ids are content-addressed and globally unique, so a claim is
  meaningless where its ids do not exist, and §8.7's replay-inertness covers the rest. It would also
  bind this type to `ObjectType::ProjectGenesis`, which — **verified while writing this handoff** —
  is never constructed anywhere, has no payload type at all, and is refused in a format-2 identity
  position (`format.rs:31`). **That is the fourth declared-but-unevaluated surface found on this
  project**, after `parent_patch_ids`, `Attestation`, and `preconditions`. Do not build on it.

### 2.3 Encoding rules

- `patch_ids` is **sorted and deduplicated at construction**, and the decoder **refuses** input that
  is unsorted or contains duplicates — it does not silently normalize. Same discipline as Stage 1's
  `compute_patch_set_digest`, and the same reason: silent normalization turns a caller's bug into a
  well-formed object that is wrong.
- `patch_ids` **must be non-empty**. A claim about no patches asserts nothing and is a decode error.
- The decoder **rejects unknown field tags**, exactly as `TagPayload::decode_canonical` does.
- Bound `patch_ids`' declared count **before allocating**, to DC-86's standard: *a declared count
  over the limit must not cost more than reading one integer to reject.* Pick the limit to match
  `DEFAULT_BUNDLE_MAX_OBJECT_COUNT` (`bundle.rs`, 100_000) and name the constant.

---

## 3. What a receiver may check — and what it must **not**

This section is the one most likely to be got wrong in a well-meaning direction.

**Must not:** the claim's `block_id` and `patch_ids` are **not existence-checked**. A claim is
verifiable with none of the referenced objects present — that is the entire reason it is a claim
object and not a Block (D3). Any code that requires the block or the patches to exist defeats the
design. Do not add such a check to `verify`, to decoding, or to acceptance.

**Must:** if the receiver **does** hold the referenced block, the claim's `patch_ids` must equal
that block's own `patch_ids`. A claim contradicting a block you already hold is a **detected lie**
and must be refused, loudly, naming both sets.

*This is a refinement beyond D3 as written, and I am adding it deliberately.* D3 established that a
claim needs no lineage; it did not say what happens when the lineage is there anyway. Leaving that
silent would mean a sender could assert a false composition and have it stored unchallenged next to
the evidence refuting it. **Verify what you can; require nothing you cannot.**

Expose this as a function taking the claim and an `impl ObjectReader`, returning a three-state
outcome — `Consistent` / `BlockAbsent` / `Contradicted { .. }` — not a `bool` and not a `Result<()>`
that flattens "absent" into "fine". `BlockAbsent` is the expected case in real exchange and must not
read as a degraded one.

---

## 4. Storage position and format admission

**Ruled: a recognition claim is a stored object, on a successful exchange only.**

The design's §8.1 already assumes this — *"no key material, and **no claim**, may be recorded from
an exchange that failed"* presupposes that claims are recorded when it succeeds. It is also what
the owner's §7 condition needs: if the claim is evaluated and dropped, no third party can later
check what a sender actually asserted.

Therefore:

- `format.rs`'s `validate_format2_schema` accepts `ObjectType::RecognitionClaim => &[1]`. It goes in
  the accepting arm, **not** the `BlockSummaryCache | RecoveryNote | ProjectGenesis` refusal arm.
- `layout.rs`'s type→directory mapper gains `"recognition-claim"`.
- `verify/objects.rs`: **no change to the trust gate.** The `matches!(object_type, ObjectType::Block
  | ObjectType::RefState)` condition at line 299 stays exactly as it is. A recognition claim's
  signatures are classified by the existing `classify_signature_envelope` path and reported; they
  never confer trust. If you find yourself editing line 299, stop — you are implementing the
  opposite of D3.

**Container placement, directory naming, and index membership are representational**, not frozen
(RFC 114 §3). Follow whatever the existing container/index machinery does for a stored type and do
not design anything new there.

---

## 5. Gate A — and a real RFC 114 shortfall this stage must close

### 5.1 The identity vectors (required)

Add **two** rows to `prikk-object/src/vectors/snapshot.txt`:

- `empty_recognition_claim|11|1||<id>` — empty payload, matching every other type's empty row.
- `recognition_claim_populated|11|1|<payload_hex>|<id>` — one block id, at least two patch ids.

**Read `snapshot.rs`'s own header before touching that file.** Adding a new type is a legitimate
reason for the snapshot to grow; it is **never** a reason for an existing row to change. If any
pre-existing row moves, that is a stop-work finding — escalate with the differing rows, do not
regenerate. `PRIKK_REGEN=1` is not the tool for this increment.

### 5.2 The signature-preimage vectors (also required — and this is the shortfall)

RFC 114 §4 gate 1 specifies frozen vectors of *"committed bytes plus their expected object id **and
signature preimage**."* **The signature-preimage half was never built.** I checked: no
`signature_preimage` vector exists anywhere in `prikk-object`, and `snapshot.txt` freezes object ids
only. Gate A is currently half-delivered.

Stage 2 is where that bites, because this is the first *signed* object type added since RFC 114 was
accepted, and a claim whose only value is its signature is exactly the thing an unfrozen preimage
would silently break.

**Ruled: freeze the signature preimage for every current `(ObjectType, SignerRole)` combination, not
only for `RecognitionClaim`.** A vector covering only the newest type would not catch a change to
`SIGNATURE_DOMAIN` or to `signed_bytes`' field order — the surfaces actually at risk — and those
affect all ten types equally. The preimage is a pure function of
`(algorithm, object_type, object_id, signer_role, key_id)`, so this is a generated snapshot in the
same shape as the existing one, over a fixed synthetic object id and a fixed `key_id`. It is cheap.

**I am aware this is wider than "Stage 2".** It is delivering an obligation RFC 114 already accepted
rather than new scope, and the standing principle is that correctness beats initial effort. If the
owner would rather split it into its own increment, that is their call to make — **say so in your
report and implement Stage 2 without it**; do not decide unilaterally in either direction.

---

## 6. The received-namespace finding, folded in

From `.git-exclude/reviewed/DC-78-bundle-tag-gap-implementation-review-v1.md` §5, and carried here
because RFC 115 D2/D5 restructure exactly this namespace:

**`verify_repository` does not scan the received / `remotes/*` namespace**, and **`import_bundle`
does not validate that the ref it lands actually resolves.** `ReceivedIndex` appears nowhere in
`verify.rs` or `refs/verify/scan.rs`; the kind-aware target check at `refs/verify/scan.rs:405-424`
runs over `read_pointers`' replay of the *pointer index*, which the received namespace does not
feed. A bundle carrying a ref whose target object it never shipped is accepted, and the resulting
dangling pointer is invisible on both sides.

**For Stage 2 this is context, not work.** Do not fix it here. What it changes is one thing: **do not
reason "verify would catch it" about anything in the received namespace.** I made exactly that
mistake in the DC-78 tag-gap ruling and the correction is recorded. If a Stage 2 test wants to prove
a receiving-side property, it must assert that property **directly**, by id, against the receiving
store — the way DC-78's condition ended up doing — never via `verify_repository` returning clean.

It becomes real work in Stage 3, where the accept path lands objects.

---

## 7. Security properties, as tests with named negative controls

Design §8 lists seven refusals. These are the ones Stage 2 can actually exercise; each needs a test
**and** an observed-failing control. **A refusal nobody has seen fire is not evidence.**

| # | Property | Control that must make it fail |
|---|---|---|
| 1 | A claim signed by key K verifies only against K's material | Verify against a different key's material → must fail, never `Sound` |
| 2 | A signature over a `RecognitionClaim` cannot be presented as a signature over any other type | Rebuild the preimage with `ObjectType::Block`, same id/role/key → verification must fail |
| 3 | A claim contradicting a held block is refused (§3) | Mutate one patch id in the claim while the block is present → `Contradicted` |
| 4 | A claim about an absent block is *accepted*, not refused (§3) | Delete the block → `BlockAbsent`, not an error |
| 5 | Unsorted / duplicated `patch_ids` are refused, not normalized | Hand the decoder unsorted bytes → decode error, not a silent sort |
| 6 | An over-limit declared count is rejected without allocating | Craft a header declaring `u64::MAX` entries → rejected on the integer, not after a large read |
| 7 | Trust never expands (§8.2) | Assert no adopted-key set changes across claim verification |

Property 2 is the domain-separation one and is the reason `object_type` is in the preimage at all.
It is currently untested for **any** type — §5.2's vectors and this test are the same argument
arriving from two directions.

**On negative controls, from this week's DC-78 review:** mutate **the narrowest line that should
break the claim**, not the whole function. A control that reverts two things at once will report
success while leaving one of them untested — that is precisely how DC-78's verify assertion passed
vacuously through two rounds.

---

## 8. Out of scope

- **Any wire format, artifact container, or transport.** Stage 3.
- **The accept path** — writing exchanged patches, recording transported key material. Stage 3.
- **Fixing the received-namespace gap** (§6). Stage 3 at the earliest, and possibly its own increment.
- **`verify/objects.rs:299`'s exclusion of `ObjectType::Tag`** from the trust gate. Real, raised
  separately, **not yours and not this increment's**. Do not touch line 299 for any reason (§4).
- **Populating `parent_patch_ids`** — a change to identity semantics needing its own RFC (D1).
- **Evaluating `preconditions`** — ruled inert (design §8's resolution).

---

## 9. What to report, and when

**Report before pushing.** In the report:

1. The **negative-control output** for each row of §7's table — the actual failure text, not a claim
   that it failed. State which single line each control mutated.
2. The **full gate set run against the exact commit, after the last edit**: `cargo fmt --all --
   check`; `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
   `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`; `git diff --check`;
   `cargo audit --no-fetch`; release-policy `check` / `boundary-check` / `reference-check`. Add the
   cross-target clippy pair (`x86_64-pc-windows-gnu`, `x86_64-apple-darwin`) **only if this diff
   contains `#[cfg(target_os)]`** — check this diff, do not carry the answer forward.
3. Test counts before and after, per crate.
4. **Whether any pre-existing `snapshot.txt` row changed.** If one did: stop and escalate, per §5.1.
5. **Your decision on §5.2's scope**, and whether you implemented it or deferred it for the owner.
6. Anything in this handoff that turned out to be wrong. **Say so plainly** — three of my documents
   this month contained an error the dev team found by building against it, and each was worth more
   to me than the parts that were right.

**Stop and escalate, do not guess**, if: a decision in §2–§5 turns out to be unimplementable as
stated; a compiler-forced site in §1 wants a semantic choice this handoff does not make; or adding
the variant changes any existing object id.
