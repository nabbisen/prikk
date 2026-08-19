# RFC 115 Stage 3 — the exchange artifact and the accept path: implementation handoff

**RFC:** `rfcs/accepted/115-sync-investigation.md` (ACCEPTED 2026-08-19).
**Design:** `rfcs/handoffs/115-sync-investigation/design-v1.md` — **D1 (§2), D2 (§3), D5 (§6), §7 and
§8 all govern here. Read them in full.** This is the increment §8 was written for.
**Follows:** Stage 1 (patch-set digest, `cea29f4`) and Stage 2 (recognition claim, `106243e`), both
accepted. **This branch stacks on Stage 2.**

This is the largest of the three stages and the only one that receives data from outside the
repository. The owner's standing instruction applies with full force: **networking and data sharing
are verified especially carefully, on both function and security.** Where this handoff and speed
disagree, this handoff wins.

Every architectural decision below is **ruled, not open**. If something is genuinely undecidable
from this document, that is my defect — escalate rather than choose.

---

## 0. What is settled, and must not be re-opened

1. **The unit is the patch** (owner's ruling, RFC 115 §2.2). Blocks are not exchanged.
2. **Patch identity stays content-only; ordering travels in the artifact** (D1). `parent_patch_ids`
   stays `Vec::new()` — populating it is a change to identity semantics needing its own RFC.
3. **There is no stored "accepted but unsealed" state. It is derived** (D2). No new container.
4. **Accept and seal are different acts** (D5). Accept verifies and writes; sealing is a separate,
   explicit, local act under the receiver's own maintainer key.
5. **Trust never expands on receipt** (§8.2). No artifact can cause a maintainer key to be adopted.
6. **`preconditions` are not evaluated** (design §8's resolution). They stay inert here exactly as
   they are for locally authored patches.
7. **Transport is out of scope** (RFC 115 §3). This produces and consumes bytes. Moving them is
   somebody else's problem, deliberately.

---

## 1. Scope — and one thing this stage deliberately does not deliver

**In scope:** the artifact format, the accept path, D2's derived query, and closing the
received-namespace verification gap.

**Ruled out of Stage 3: the seal-from-accepted path.**

D5 says the receiver's own seal is "a separate, explicit, local act." What it does not say is *how* an
accepted patch reaches a block, and the answer is not currently available: `seal` builds a block from
**the active WAL** (`prikk-cli/src/seal.rs` → `persist_wal_patches`), and an accepted patch is a
written object that was never in the WAL. Closing that needs either foreign patches injected into the
local authoring WAL — whose invariants (`require_patch_record`, baselines,
`current_tip_matches_wal_patches`) assume locally authored records — or a new seal path taking
explicit patch ids. **Both are real design work that Stage 3 does not otherwise touch, so neither
belongs in it.**

**What lands, therefore, is: you can receive patches, verify them, store them, and see exactly what
you hold that is not yet sealed — and you cannot yet seal them.** That is a genuinely incomplete
feature and I am stating it plainly rather than letting it be discovered. It is also the right cut:
it puts the increment boundary exactly on the trust boundary, so the half that admits foreign bytes
and the half that confers local authority are reviewed separately. **Do not quietly extend scope to
close it.** Escalate if you believe the cut is wrong.

---

## 2. What already exists — build on it, do not rebuild it

| Surface | Where | Use it for |
|---|---|---|
| `export_bundle` / `import_bundle` | `prikk-store/src/bundle.rs` | **The model for everything structural here** — length-prefixed sections, magic, bounds-before-decode, the report types. |
| `BundleImportOptions` | `bundle.rs:87` | The bounds shape. `DEFAULT_BUNDLE_MAX_OBJECT_COUNT` = 100_000, `DEFAULT_BUNDLE_MAX_TOTAL_BYTES` = 256 MiB. |
| `record_author_key_material` / `check_author_key_conflict` | `prikk-store/src/author_key_index.rs` | Transported key material. Takes `&ActiveLock` as a compile-time precondition. |
| `verify_author_signature` | `author_key_index.rs` | DC-53's four outcomes. **Sources keys from the layout's recorded index** — see §4.2 item 7, which this constrains. |
| `compute_patch_set_digest` | `prikk-store/src/patch_set_digest.rs` | Stage 1. Refuses unsorted input. |
| `RecognitionClaimPayload`, `check_recognition_claim_consistency` | Stage 2 | Block-level recognition. Refuses malformed claims. |
| `list_received_pointers` | `prikk-store/src/received.rs:116` | **Already exists.** §6's fix needs it and nothing new. |
| `ensure_ref_target_valid` | `prikk-store/src/refs/verify/scan.rs:405-424` | Kind-aware ref-target validation. §6 reuses it as-is. |
| `persisted_object_types` + `decode_container_records` | `layout.rs`, `container.rs` | §5's derived query enumerates stored objects this way — the same walk `verify/objects.rs:163` uses. |

**Read `import_bundle` in full before designing the accept path.** It has already been through two
security defects that Stage 3 can inherit for free if you copy its current shape, and can re-earn if
you don't — see §4.3.

---

## 3. The artifact

### 3.1 Framing

- **Magic:** `PEXCH001`. New format, not a bundle variant — a bundle is block-level and
  genesis-complete; this is neither.
- **Representational, not frozen.** RFC 114 §3 lists bundles as representational, and this is the
  same kind of thing: it carries objects whose identity is already frozen, and carries no identity of
  its own. It may change in a later version with a documented read path. **Say so in the module doc**,
  so the next person does not treat it as a frozen surface — and equally, do not treat that licence
  as permission to be careless.
- **`PEXCH001` is emitted on export and accepted on import. There is no retired version yet.** When
  there is, the rule is RFC 114's: *read what the past wrote; write only the present.*

### 3.2 Sections, in order

1. **Declared patch-set digest** (32 bytes) — Stage 1's digest over the artifact's own patch ids.
2. **Ordered patch list** — Patch envelopes, in the sender's application order (D1). Order is
   artifact metadata; it is not part of any object's identity and must never be treated as such.
3. **Blobs** — every blob any carried patch references.
4. **Author key material** — `key_id → public_key`, the same shape the bundle's author-key section
   carries.
5. **Recognition claims** — Stage 2 `RecognitionClaim` envelopes with their signatures. May be empty.

**The declared digest is not redundant.** The receiver recomputes it over the sorted, deduplicated
ids of the patches it actually decoded and **refuses on mismatch**. That catches truncation,
reordering-with-substitution, and a sender whose own view of what it was sending disagrees with the
bytes — cheaply, before any signature work.

---

## 4. The accept path

### 4.1 The invariant that orders everything

Design §8.1: **a refused exchange leaves nothing behind.** Objects are content-addressed and
harmless, but **no key material and no claim may be recorded from an exchange that failed.**

Therefore: **every check that can fail runs before any write.** Not "mostly before". The write phase
must contain nothing that can fail for a reason attributable to the artifact's content.

### 4.2 Order of operations — implement in exactly this order

**Phase A — bounds, before decoding anything.**
1. Total byte length against the configured maximum.
2. Declared counts against their maxima, each checked at the earliest point the format allows. DC-86's
   standard: *a declared count over the limit must not cost more than reading one integer to reject.*

**Phase B — decode and self-check, still no writes.**
3. Decode all sections.
4. Recompute the patch-set digest over the decoded patches; refuse on mismatch with the declared one.
5. Artifact-internal author-key conflict check: two different public keys for one `key_id` **within
   the artifact** refuses the whole import. (`import_bundle` learned this the hard way; the reasoning
   is in its own comments.)
5b. Artifact-versus-repository conflict check: `check_author_key_conflict` is read-only, so run it
   here for every entry as a cheap early refusal, **before** any signature work. This does **not**
   replace Phase D's check under the lock — that one is authoritative, because check-then-act without
   the lock is a race. Two checks, deliberately: one to fail fast, one to be correct.
6. **Closure completeness (§8.4).** Every blob referenced by a carried patch must be present — in the
   artifact or already in this repository. A missing referent **refuses the whole exchange**; there is
   no partial apply. `parent_patch_ids` is always empty today, so there is nothing to walk there —
   **check it anyway and refuse if it is ever non-empty**, because the day it stops being empty this
   is the code that must not silently ignore it.

**Phase C — cryptographic verification, still no writes.**
7. Every carried patch's AUTHOR signature.

   **Read this item carefully — the obvious implementation does not work.**
   `verify_author_signature(layout, envelope)` sources its keys from
   `lookup_author_key_entries(layout, ..)`, i.e. **the repository's recorded index**. At Phase C that
   index does not yet contain the artifact's material, because recording it is Phase D — and moving
   the recording earlier would record key material from an exchange that may still fail, which is
   precisely what §4.1 forbids. The two constraints are real and they do not compose.

   **Ruled: extract the verification core, do not duplicate it.** Factor
   `verify_author_signature`'s preimage construction, the one-key_id-one-public-key invariant, and
   the `verify_ed25519` call into a shared function taking the candidate key material as a
   parameter. `verify_author_signature` keeps its current signature and passes the recorded
   entries; the accept path passes **the union of the artifact's material and this repository's
   already-recorded material**. One definition of *how* an author signature is checked, two key
   sources — the same shape `check_author_key_conflict` was extracted into, for the same reason.
   **Do not write a second, parallel verification policy.**

   **A signature that fails against material the artifact itself supplied refuses the whole
   exchange** (§8.6). A patch with *no* material available in either source reads `Unverifiable` —
   **never `Sound`** — and does not by itself refuse.
8. Every recognition claim's own signature.
9. Every recognition claim against blocks this repository already holds, via
   `check_recognition_claim_consistency`. **`Contradicted` refuses the whole exchange.**
   `BlockAbsent` is expected and fine.

   *On the apparent tension with "never gating" (§8.3):* a claim never **confers** trust and never
   gates on trust — that is what D3 rules. A claim **proven false against evidence already held** is
   not a trust opinion at all; it is a demonstrated integrity failure of the artifact. Accepting
   patches from a sender while holding proof it signed a falsehood is not caution, it is negligence.
   Refuse.

**Phase D — validate-all-then-record-any, under one lock.**
10. Write the patch, blob and claim objects.
11. Under a **single** `ActiveLock`: first `check_author_key_conflict` for **every** entry against
    this repository's material, **then** `record_author_key_material` for every entry. Never
    check-then-record one entry at a time.

    *This is not a style preference.* Doing it per-entry is the exact defect recorded in
    `multi-key-import-partial-write-v1.md`: a conflict at entry *k* left entries `1..k-1` durably
    appended to a container with no prune, no compaction and no repair. Splitting validate and record
    across the lock boundary reintroduces it as a check-then-act race. `import_bundle` already does
    this correctly — copy that structure.

### 4.3 Replay must be inert (§8.7)

Re-accepting an identical artifact writes no new object, records no new key material, records no new
claim, and changes no state. Assert it directly: run accept twice and compare object counts, the
author-key container, and the claim set. **Do not assert it via `verify_repository` returning clean**
— see §6.

---

## 5. D2's derived query

**No new container. No stored pending state.** "Accepted but unsealed" is computed:

> patch objects present in this repository, minus the patch ids reachable from any block.

Enumerate stored patches the way `verify/objects.rs:163` enumerates objects —
`persisted_object_types()` → per-type container → `decode_container_records`. Reuse Stage 1's
`patch_ids_reachable_from_block` for the subtrahend, over every ref's tip. Do not write a second
ancestry walk; Stage 1 deliberately reused `merge_evidence::ancestors_inclusive` for exactly this
reason.

Expose it as a query returning the ids, sorted. It is the only way an operator can see what an accept
actually left them holding — and, until §1's seal path exists, it is the *whole* of what accept
produces that a person can observe. Make it good.

---

## 6. The received-namespace gap — now real work

From `.git-exclude/reviewed/DC-78-bundle-tag-gap-implementation-review-v1.md` §5:

**`verify_repository` does not scan the received / `remotes/*` namespace.** `ReceivedIndex` appears
nowhere in `verify.rs` or `refs/verify/scan.rs`; the kind-aware target check runs over
`read_pointers`' replay of the *pointer index*, which the received namespace does not feed. A ref
whose target object was never shipped dangles invisibly, on both sides.

**In scope for Stage 3: give `verify` a received-namespace stage.** Iterate `list_received_pointers`,
resolve each pointer's RefState, and apply the existing `ensure_ref_target_valid` — the same
kind-aware two-hop check local refs already get. Both surfaces already exist; this is wiring, not
design. It closes the hole for `import_bundle` and for the new accept path at once, which is why it
is here rather than in either one.

**Not in scope:** adding target-presence validation inside `import_bundle` itself. Its own missing
closure check is a separate item; the verify stage makes the consequence visible everywhere, which is
the part that matters now.

**Report, do not silently handle:** a repository that already imported a tag bundle before the DC-78
fix may hold a genuinely dangling received ref, which this stage will start reporting as a failure
where it previously passed. Say in your report what such a repository now does. Do **not** soften the
check to keep it quiet.

**And the standing caution that came out of my own error:** never reason "verify would catch it" about
anything in the received namespace. Until this stage lands, it would not have. Any test proving a
receiving-side property must assert that property **directly, by id, against the receiving store**.

---

## 7. Security properties, as tests with named negative controls

Design §8's seven refusals, plus what this stage adds. Each needs a test **and** an observed-failing
control. **A refusal nobody has seen fire is not evidence.**

| # | Property | Control that must make it fail |
|---|---|---|
| 1 | A refused exchange records no key material and no claim | Force a Phase-C failure; assert the author-key container and claim set are byte-identical to before |
| 2 | Trust never expands on receipt | Assert the adopted-maintainer set is unchanged across a successful accept |
| 3 | A recognition claim is reportable, never trust-conferring | A claim naming an unadopted key still accepts; the key stays unadopted |
| 4 | Missing closure refuses the whole exchange | Drop one referenced blob → refusal, and **no patch object written** |
| 5 | Bounds are enforced before decoding | Declared count over the limit → rejected on the integer |
| 6 | A patch failing against transported material refuses; one with no material reads `Unverifiable` | Corrupt one signature → refusal. Separately: omit material → `Unverifiable`, never `Sound` |
| 7 | Replay is inert | Accept twice → second is a no-op on every surface (§4.3) |
| 8 | Digest mismatch refuses before signature work | Truncate the patch list, leave the declared digest → refusal |
| 9 | A claim contradicting a held block refuses the exchange | Ship a claim disagreeing with a locally held block → refusal, nothing written |
| 10 | A dangling received ref is now visible | Construct one; `verify` must report an item failure where it previously passed |

**On controls, from this month's reviews:** mutate **the narrowest line that should break the claim**,
not the whole function. A control that reverts two things at once reports success while leaving one
untested — that is exactly how DC-78's verify assertion passed vacuously through two rounds, and it is
the single most repeated finding in this project's reviews.

**Row 1 is the one to get right.** It is the property the two author-key defects of this month both
violated, and the only one whose failure is invisible in normal use.

---

## 8. Out of scope

- **The seal-from-accepted path** (§1). The largest exclusion; read §1 before assuming otherwise.
- **Transport of any kind** (RFC 115 §3).
- **`import_bundle`'s own closure validation** (§6).
- **Populating `parent_patch_ids`** (D1) — check-and-refuse only, per §4.2 item 6.
- **Evaluating `preconditions`** — ruled inert.
- **`verify/objects.rs:299`'s exclusion of `ObjectType::Tag`** — raised separately, still not yours.
- **A pending-acceptance container** (design §9) — D2 rules it out; §5 is the alternative.
- **The `merge_execute` fast-forward gap** — real, separable, unrelated.

---

## 9. What to report, and when

**Report before pushing.** In the report:

1. The **negative-control output for every row of §7** — actual failure text, and which single line
   each control mutated. Ten rows; do not compress them.
2. **Explicitly, for row 1:** what you compared, byte-for-byte, to prove nothing was left behind.
3. The **full gate set against the exact commit, after the last edit**: `cargo fmt --all -- --check`;
   `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
   `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`; `git diff --check`;
   `cargo audit --no-fetch`; release-policy `check` / `boundary-check` / `reference-check`. Add the
   cross-target clippy pair (`x86_64-pc-windows-gnu`, `x86_64-apple-darwin`) **only if this diff
   contains `#[cfg(target_os)]`** — check this diff, do not carry the answer forward.
4. Test counts before and after, per crate.
5. **What a repository holding a pre-existing dangling received ref now does** (§6).
6. **Whether any pre-existing `snapshot.txt` row changed.** If one did: stop and escalate.
7. Anything in this handoff that turned out to be wrong. **Say so plainly.** Stage 2's §3 rule was
   wrong in a way that would have inverted a security property into falsely accusing honest senders,
   and you found it by building against it. That was worth more than the parts I got right.

**Stop and escalate, do not guess**, if: §1's scope cut looks wrong once you are inside the code;
Phase C's refusal ordering conflicts with something real; the derived query in §5 turns out not to be
computable from the existing enumeration; or §6's verify stage fails against repositories that ought
to be healthy.
