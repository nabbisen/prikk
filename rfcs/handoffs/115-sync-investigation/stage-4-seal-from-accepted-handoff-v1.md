# RFC 115 Stage 4 — sealing what you accepted: implementation handoff

**RFC:** `rfcs/accepted/115-sync-investigation.md` (ACCEPTED 2026-08-19).
**Design:** `rfcs/handoffs/115-sync-investigation/design-v1.md` — **D5 (§6) and D6 (§11) govern. Read
§11.6 in full; it is the ruling this stage most depends on and the easiest to implement backwards.**
**Follows:** Stages 1-3 (`0128c91`) and the D6 order amendment (`1e72235`), all merged.
**Base:** current `main`.

**This stage closes criterion 1's first gap.** Until it lands, a receiver can accept, verify and store
patches and cannot turn them into sealed history — so "two machines can exchange sealed history" is not
yet true.

Every architectural decision below is **ruled, not open**. If something is genuinely undecidable from
this document, that is my defect — escalate rather than choose.

---

## 0. What is settled, and must not be re-opened

1. **The order comes from the recognition claim**, which since `1e72235` carries the block's
   `patch_ids` verbatim (D6). Do not re-derive it; `patch_algebra` cannot produce one (D6 §11.2).
2. **The order is a hint that must be *tried*, never a fact that is *trusted*** (D6 §11.6). See §4.
3. **Patch identity stays content-only** (D1). `parent_patch_ids` stays empty and unevaluated.
4. **Trust never expands.** The receiver seals under their **own** maintainer key;
   `verify_signer_trusted` is unchanged and still gates.
5. **No stored pending state** (D2). The unsealed set stays derived.
6. **`preconditions` are not evaluated.**

---

## 1. What Stage 4 is: one claim, one block

**Ruled: a single recognition claim produces a single block.**

A claim already describes exactly one block — its `block_id`, and since D6 its patch sequence verbatim.
So the receiver reconstructs a *local equivalent* of that block: the same patches, in the same order,
on the receiver's own parent, sealed by the receiver's own key.

**The resulting block id will not equal the claim's `block_id`, and that is correct, not a failure.**
A different parent yields a different state root yields a different id. RFC 115 §2.4-§2.7 already ruled
that blocks diverge between repositories by design and that this loses nothing, because identity lives
at the patch level. **Say so in the module doc**, because "the ids don't match" will otherwise be read
as a bug by whoever meets it first.

Surface: a function taking the target ref, the claim id, and the receiver's signer — the same three
things `seal` needs, from a different source.

---

## 2. What already exists — build on it, do not rebuild it

| Surface | Where | Use it for |
|---|---|---|
| `seal`'s block construction | `prikk-cli/src/seal.rs:152-200` | **The exact shape to mirror.** Parent from the current `RefState`, `derive_next_state_root`, `BlockPayload`, `signed_envelope`, then the `RefStatePayload` with `update_seq + 1` and `previous_ref_state_id`. |
| `verify_signer_trusted` | called at `seal.rs:151` | Unchanged. Gates before any seal. |
| `derive_next_state_root` | `block_state.rs` | Computes the new state root — **and is where divergence surfaces. See §4.** |
| `accepted_but_unsealed_patch_ids` | `patch_exchange.rs` | Stage 3's derived query. The set this stage draws from. |
| `check_recognition_claim_consistency` | `recognition_claim.rs` | Now exact sequence equality (D6). |
| `rollback_draft`'s empty-WAL guard | `rollback_draft.rs:78-81` | **The precedent §5 follows.** |
| Container walk | `verify/objects.rs:163`, as reused by `accepted_but_unsealed_patch_ids` | Enumerating stored claims. |

**The patches are already written objects.** There is no `persist_wal_patches` step; that is the whole
structural difference from `seal`.

---

## 3. Selecting the claim — refuse ambiguity, never resolve it

1. The named claim must exist and decode.
2. **Every patch it names must be present** in this repository. A missing one **refuses** — no partial
   application, the same rule Stage 3's §8.4 closure check enforces.
3. **Every patch it names must be currently unsealed** — present in `accepted_but_unsealed_patch_ids`.
   If all of them are already sealed, this is a **no-op success**, not an error: replay must be inert
   (§8.7), and re-running a completed seal is exactly that. If *some* are sealed and some are not,
   **refuse** — that is a partially-applied state this stage must not deepen.
4. **If two stored claims name overlapping patch sets and disagree on order, refuse**, naming both.
   Do not pick one, do not prefer the newer, do not merge them. Ambiguity about order is precisely
   what must not be guessed, and there is no rule that would make one choice defensible.

**A claim's signature outcome does not gate selection.** Per D6 §11.6, an `Unverifiable` claim may
supply an order. **Report the outcome alongside the result** — the operator must be able to see they
sealed on an unattributed order — but do not refuse on it. Refusing would make this stage useless on
first contact, where every claim is permanently `Unverifiable`.

---

## 4. The correctness risk that defines this stage: divergence is not corruption

**This is the first time prikk applies patches that were not authored against the state they are being
applied to.** Everything `apply_candidate_patches` currently serves is sealed-history replay, where a
patch definitionally applied cleanly when authored. Its error type reflects that assumption — and
`TextSpanResolutionFailed`'s own doc says so outright: *"This is an integrity failure (the sealed edit
applied cleanly when authored), not a user/merge conflict."*

**That comment is true of today's callers and false of this one.** An accepted patch failing to apply
to the receiver's tip is an ordinary divergence — the two histories moved differently — and reporting
it as repository corruption would be a serious diagnostic defect: it tells an operator their repository
is broken when nothing is broken.

**Ruled classification.** When applying accepted-but-unsealed patches onto the receiver's tip:

| `LifecycleReplayError` variant | Stage 4 reads it as |
|---|---|
| `InconsistentLifecycleEffect` | **Divergence** — expected, reportable |
| `TextSpanResolutionFailed` | **Divergence** — expected, reportable |
| `MissingBlockInLineage`, `UnreadableBlockInLineage`, `MergeLineageUnsupported`, `LineageCycle`, `HorizonNotInLineage` | **Integrity** — the receiver's *own* lineage is damaged |
| `MalformedPatchInLineage` | **Integrity** — accept verified these objects |
| `MissingBlobForLifecycleEffect` | **Integrity** — accept's closure check should have caught it |

The split is principled, not arbitrary: the first two describe *the patch disagreeing with the state it
met*; the rest describe *this repository being broken*.

**Implementation consequence you must handle:** `derive_next_state_root` returns
`Result<MerkleRoot, PrikkError>`, and `From<LifecycleReplayError> for PrikkError`
(`lifecycle_cache/replay.rs:137`) flattens the variant away. **You need a variant-preserving path** —
either a sibling entry point returning the typed error, or classification before conversion. Do not
match on error *strings*.

**On divergence: refuse the seal, cleanly, naming the patch and what disagreed. Write nothing.** Do not
attempt resolution, do not partially seal, do not fall back to another order. Resolving divergence is
merge's job and is out of scope (§7).

---

## 5. The active WAL must be empty

**Ruled: refuse when the active WAL is non-empty.**

Sealing accepted patches advances the branch tip. Locally queued WAL patches were composed against the
*old* tip, and since DC-66 a queue chains baselines — so advancing the tip underneath them invalidates
assumptions they were built on.

**Precedent, followed rather than invented:** `rollback_draft` already requires an empty active WAL for
exactly this class of reason — *"composing a correct inverse against a queue's chained, not-yet-sealed
baseline is unaddressed."* The same reasoning applies unchanged here.

Refuse with a message that tells the operator what to do: seal or discard local work first.

---

## 6. Security properties, as tests with named negative controls

Each needs a test **and** an observed-failing control. **A refusal nobody has seen fire is not evidence.**

| # | Property | Control |
|---|---|---|
| 1 | Sealing requires a locally trusted signer | Seal with an unadopted key → refused by `verify_signer_trusted` |
| 2 | Trust does not expand | Adopted-maintainer set byte-identical across a successful seal |
| 3 | An `Unverifiable` claim can still supply an order | Claim from an unadopted signer → seals, outcome reported as `Unverifiable` |
| 4 | Divergence refuses as divergence, **not** as integrity | Craft a patch that cannot apply to the receiver's tip → refusal classified divergence, and **nothing written** |
| 5 | Genuine integrity failures still read as integrity | Remove a block from the receiver's own lineage → integrity, not divergence |
| 6 | A missing named patch refuses | Delete one → refusal, no block written |
| 7 | Partial seal state refuses | Some named patches already sealed → refusal |
| 8 | Ambiguous claims refuse | Two claims, overlapping sets, different order → refusal naming both |
| 9 | Replay is inert | Seal the same claim twice → second is a no-op, no new block |
| 10 | Non-empty active WAL refuses | Queue a local commit → refusal |
| 11 | The sealed block carries the claimed order verbatim | Sort the order before sealing → block's `patch_ids` differs from the claim's |

**Rows 4 and 5 are the pair that matters.** Together they prove the classification is real rather than
a label — either alone can pass with the distinction unimplemented.

**On controls:** mutate **the narrowest line that should break the claim.** A control reverting two
things at once reports success while leaving one untested — the single most repeated finding in this
project's reviews.

---

## 7. Out of scope

- **Resolving divergence.** Refuse and report; merging is `merge`'s job.
- **Transport** (RFC 115 §3). Still open, still not this.
- **CLI wiring**, unless it falls out trivially — Stage 3 kept these surfaces at `prikk-store` level,
  matching `bundle.rs`'s own "writing bytes to a file is a CLI concern" note. Follow that; say in your
  report if you think the boundary should move.
- **`import_bundle`'s missing closure validation** — a separate open item.
- **The third near-identical ref-tip resolution** — recorded, its own increment.
- **`parent_patch_ids`**, **`preconditions`**, **`verify/objects.rs:299`'s Tag exclusion** — all ruled
  out or raised elsewhere.

---

## 8. What to report, and when

**Report before pushing.** In the report:

1. **Negative-control output for every row of §6** — actual failure text, and which single line each
   control mutated. Eleven rows; do not compress them.
2. **For rows 4 and 5 specifically:** how you kept the `LifecycleReplayError` variant alive to the
   classification point, and how you proved the two paths genuinely diverge.
3. **The full gate set against the exact commit, after the last edit**: `cargo fmt --all -- --check`;
   `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
   `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`; `git diff --check`;
   `cargo audit --no-fetch`; release-policy `check` / `boundary-check` / `reference-check`. Cross-target
   clippy pair only if this diff contains `#[cfg(target_os)]` — check this diff.
4. Test counts before and after, per crate.
5. **Whether any `snapshot.txt` row changed.** It must not — this stage adds no object type and no
   schema. If one did: stop and escalate.
6. Anything here that turned out to be wrong. **Say so plainly.** Three of my handoffs this month
   contained an error you found by building against them, two of them security-relevant, and each was
   worth more to me than the parts I got right.

**Stop and escalate, do not guess**, if: §4's classification does not survive contact with the real
error paths; §3's no-op-vs-refuse rule turns out to have a case it does not cover; the empty-WAL
refusal in §5 blocks something that ought to work; or sealing produces a block whose `patch_ids` cannot
match the claim's for a reason this document does not anticipate.
