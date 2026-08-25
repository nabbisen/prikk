# Conflict witnesses — surface them; automatic resolution is refused by design

**Base:** current `main` (`1c2a8a1`). **Under `003-landing-work-on-main.md`.**
**Closes:** the ROADMAP "Conflict arbitration" theme — **not by building an arbitrator.**

---

## 1. The ruling, derived from this project's own decisions

**Automation may not author a conflict resolution.** A resolution is a patch; a patch must be authored
and signed; so an automatic arbitrator signs content on a human's behalf.

**DC-35:** *"Automation may verify evidence but cannot occupy either accountable approval identity."*

**DC-74 already applied that at the patch layer**, and its reasoning is the point:

> In a context-dependent model (Darcs-style) merging *transforms* the incoming patch, its canonical
> bytes change, its ObjectId moves, and the original AUTHOR signature no longer covers it — so whoever
> merges must re-sign content they did not write. That is DC-35's "automation cannot occupy an
> accountable approval identity," arriving at the patch layer. **prikk's design avoids this entirely.**

**prikk's merge was built so that nobody re-signs content they did not write.** An auto-resolver would
reintroduce exactly that. **So arbitration is refused by the architecture, not merely unscoped.**

**Resolution already exists**: merge refuses, and a human reconciles by authoring patches under their
own key. That is ordinary committing and needs nothing built.

**Record this ruling** in `docs/src/reference/` beside the merge material, so the theme does not return
as an ergonomics request. **State the derivation, not the conclusion alone** — a reader must be able to
follow DC-35 → DC-74 → here.

## 2. What is actually missing

`patch_algebra` produces **twelve** `ConflictWitnessKind` variants — `SamePathCreate`, `NodeIdReuse`,
`LiveStateMismatch`, `KindMismatch`, `ModeMismatch`, `BlobMismatch`, `TextSpanOverlap`,
`TextAnchorStale`, `DeleteMutationConflict`, `UnsupportedOperation`, `MalformedOperation`,
`UnknownRelation`.

**None reaches the user.** `mod patch_algebra` is **private**; `ConflictWitness` is `pub(crate)`;
neither `merge_execute` nor `merge_evidence` consumes a witness.

**What the user gets today** (`merge_execute.rs:107`) is real but coarse:

```
merge refused: {from_ref} is not confluent with {into_ref} from baseline {baseline_block_id}
(outcome: {outcome}, reason: {reason})
```

**An outcome and an optional reason — but no per-conflict detail**: no kind, no path, no node. **I
described this as "no cause" when first reporting it; that was wrong, and the correction matters,
because the gap is narrower than it sounded.** You are adding detail to a refusal that already
explains itself at the whole-merge level.

## 3. Where it goes, and what must not become public

**Home: the existing public merge-evidence display layer.** `MergeEvidenceDisplay`,
`MergeEvidenceDisplayItem`, and `MergeEvidenceDisplayOperation` are already re-exported from
`prikk-store`, and `MergeEvidenceDisplayItem` already carries `reason_code: &'static str` — **a stable
string code is an established pattern here, not a new one.**

**`ConflictWitness`'s fields are not equally publishable:**

| Field | Disposition |
|---|---|
| `kind` | **Publish**, as a stable kebab-case label |
| `path: Option<RepoPath>` | **Publish** — the user needs it |
| `node_id: Option<NodeId>` | **Publish** if it is meaningful to a user; argue either way |
| `left_op_seq`, `right_op_seq` (`u32`) | **Do not publish raw.** Internal indices into a patch's operation list — publishing freezes an internal representation as an interface |
| `text_span: Option<[u8; 32]>` | **Do not publish raw.** Argue for a rendering, or omit |

**If you conclude an op-seq pair is genuinely necessary to make a witness actionable, say so and
propose a rendering** — do not export the raw indices because they were there.

## 4. Apply the stage 4/5 pattern

`ConflictWitnessKind` is an enum whose labels become an external interface. **That is exactly
`VerificationStage`**, and RFC 118 stages 4 and 5 settled how this project does it:

- **one token list** generating the enum, its `ALL`, and its `label()` — so a thirteenth kind cannot be
  added to one and forgotten in the others;
- **`label()` documented as an external interface**, stable kebab-case, renaming = breaking;
- **a completeness gate** proving every variant is covered.

**Reuse it. Do not invent a second shape**, and say plainly if the macro does not transfer.

## 5. Out of scope

- **Any resolver, assisted or automatic** (§1).
- **Changing conflict detection**, or what `patch_algebra` decides.
- **Making `mod patch_algebra` public.** Publish a curated view, never the module.
- **`--format json` for merge.** `verify` has one; merge is a separate decision. **Do not add it here.**
- Changing the existing refusal's outcome/reason strings.

## 6. Controls

1. **A real conflict surfaces its witnesses** — construct one of each of at least three distinct kinds
   (`SamePathCreate`, `TextSpanOverlap`, and one delete-related), and quote the user-visible output.
2. **The completeness gate fires** on a kind that exists but is unlabelled or unmapped — mutate, quote,
   revert.
3. **Nothing internal leaked** — show the public API gained no `op_seq`-shaped field and no raw
   `[u8; 32]`, and that `patch_algebra` is still private.
4. **The existing refusal is unchanged** for merges that already refused — byte-compare against
   `1c2a8a1` for a conflict case.
5. **Full suite green**, and say whether the count moved and why.

**Quote every failure.** A control that passes without your assertion firing is worse than none.

## 7. What to report

1. **Where the ruling is recorded**, and the derivation you wrote.
2. **Your field-by-field disposition** (§3), with reasoning for `node_id`, `op_seq`, and `text_span`.
3. **Whether the stage 4/5 macro transferred**, and what you did if not.
4. All five controls (§6), quoted.
5. **Full gate set against the exact commit, after the last edit**, including `mdbook build`.
6. **Every numbered requirement's disposition, including ones that went without incident.**
7. Anything here that was wrong.

**Stop and escalate, do not guess**, if: surfacing a witness requires `patch_algebra` to become public;
a witness kind cannot be rendered without exposing an internal type; or **you find a path where merge
already resolves a conflict rather than refusing** — that would contradict §1 and everything else here
should stop until I have looked at it.
