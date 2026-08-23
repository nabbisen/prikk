# Patch Algebra and Merge Evidence

This page is the authoritative current-state reference for Prikk's patch algebra and merge-evidence
concepts. It describes the current implementation and is grounded in the code, released RFCs, and
implementation status records listed in the anchor table at the foot of the page.

For command syntax and examples, see the [merge evidence](../guide/merge-evidence.md) and
[merge plan](../guide/merge-plan.md) guides.

## Core Caveats

- Prikk is early implementation software and is not a production Git replacement.
- Patch algebra and merge evidence are currently read-only analysis surfaces.
- `prikk merge-evidence` and `prikk merge-plan` require explicit baseline, left target, and right
  target inputs. They do not infer merge bases or branch merge intent.
- Current confluence results apply only to the supported operation subset and the selected explicit
  candidate sequences.
- `Confluent` and `ConfluentSubset` alone do not create a merge commit — `prikk merge` (DC-74) is the
  separate, explicit command that executes a confluent merge; see the [merge guide](../guide/merge.md).
- Active-WAL merge drafts, worktree conflict materialization, conflict-resolution UI, persisted
  proof/witness/plan objects, JSON output, same-node text operational transforms, path-scoped
  analysis, and public stable Rust APIs remain deferred.

## Patch Operations and Ordering

A Patch contains ordered operations. The evidence displays use `op_seq` to show the one-based operation
sequence recorded by a Patch operation, while bracketed indexes such as `left[0]` and `right[0]` show
the zero-based position in the derived left or right candidate sequence.

The current evidence model summarizes operation kind, optional node id, and a safe repository-relative
path when one is available. It does not expose raw operation payloads. Preconditions and evidence
facts are checked through the store-backed patch-algebra evidence boundary; malformed required sealed
evidence is an evidence failure, not ordinary unsupported algebra.

## Pair Classification

Internal pair classification currently uses four categories:

| Pair class | Meaning |
|---|---|
| `Independent` | The classifier sees no ordering or conflict relation for the pair, subject to later replay proof. |
| `OrderedDependency` | The pair has a required order, such as create-after-delete relations that can only be considered in one direction. |
| `Conflict` | The pair has a concrete conflict witness, such as same-path creation, live-state mismatch, mode/blob mismatch, or delete/mutation conflict. |
| `Unknown` | The relation cannot be safely classified, either because the operation/relation is unsupported, evidence is insufficient, or the design is intentionally deferred. |

These Rust categories are implementation details, not stable public API. Public commands surface the
separate merge-evidence outcomes described below.

Intent metadata is advisory. It does not override replay, lifecycle, preimage, evidence, or
commutation proof requirements.

## Commutation

Prikk treats a pair as commuting only when both conditions hold:

- the classifier reports `Independent`; and
- replaying the pair in both orders produces the same lifecycle state.

If the classifier reports an ordered dependency or conflict, the pair does not commute. If required
evidence is missing or malformed, the analysis fails closed as an evidence problem. If a relation is
not supported or is intentionally deferred, it remains unknown rather than being treated as safe.

## Flat Confluence

Current confluence is flat and explicit-input. The analysis receives a sealed baseline state plus two
candidate operation sequences derived from explicit left and right targets.

The current check requires:

- each candidate sequence to replay validly enough for the supported subset;
- cross-pairs between left and right to commute;
- replay of left-then-right and right-then-left to succeed; and
- final lifecycle states to be equal.

This is not automatic branch merge semantics. It does not choose a merge base, publish a result,
materialize a worktree, create a merge commit, or create multi-parent Blocks.

## Evidence Outcomes

`prikk merge-evidence` prints the public DC-21/DC-23 outcome vocabulary:

| Outcome | Meaning |
|---|---|
| `Confluent` | The selected sequences are proven confluent under the current supported analysis. This is scoped evidence, not execution readiness. |
| `Conflict` | A concrete conflict witness was found. |
| `OrderedDependency` | A relation requires ordering policy that the current public merge surface does not execute. |
| `Unsupported` | The operation kind or relation is outside the supported algebra subset. |
| `Deferred` | The relation is known but intentionally deferred, such as same-node text transforms or sequence-internal dependency handling. |
| `NotConfluent` | Replay or final-state comparison failed after otherwise supported analysis. |
| `EvidenceFailure` | Required sealed evidence is missing, malformed, unreadable, wrong-type, or identity-invalid. |
| `InvalidCandidate` | Candidate input is malformed or insufficient before analysis can produce usable evidence. |

`EvidenceFailure` is distinct from `Unsupported` or `Deferred`: required sealed evidence failures must
not be hidden as unknown algebra.

## Reason Codes and Proof Phases

Evidence output also prints `reason:` and item-level `phase:` fields. Reason codes explain why an
outcome was produced; phases say which proof stage produced the item.

Current public reason-code names include:

| Reason code | Meaning |
|---|---|
| `proven_confluent` | The selected pair or sequence passed the current confluence proof. |
| `pair_conflict` | A cross-side pair produced a conflict witness. |
| `ordered_dependency` | A cross-side pair requires a specific order. |
| `unsupported_operation` | The operation or relation is outside the current supported subset. |
| `same_node_text_transform_deferred` | Same-node text operational transforms are intentionally deferred. |
| `sequence_internal_dependency_deferred` | A sequence-internal dependency blocks flat confluence analysis. |
| `pair_replay_failed` | Replaying a pair in both orders did not prove commutation. |
| `final_state_mismatch` | Final lifecycle states differed after composed replay. |
| `missing_required_evidence` | Required sealed evidence was absent. |
| `malformed_required_evidence` | Required sealed evidence was present but malformed. |
| `wrong_type_required_evidence` | Required sealed evidence had the wrong object kind. |
| `unreadable_required_evidence` | Required sealed evidence could not be read. |
| `invalid_unsealed_candidate` | Optional unsealed candidate evidence was malformed. |
| `insufficient_unsealed_candidate_evidence` | Optional unsealed candidate evidence was insufficient for analysis. |

Current public proof phases include:

| Phase | Meaning |
|---|---|
| `classification` | Pair classification or evidence validation produced the item. |
| `replay-both-orders` | Pair replay in both operation orders produced the item. |
| `flatness` | Candidate-sequence flatness checks produced the item. |
| `final-state-comparison` | Final lifecycle-state comparison produced the item. |

`composed-replay` exists only behind test-only display code and is not a current public phase.

## Merge Plan Mapping

`prikk merge-plan` preserves the underlying evidence outcome and maps it to a non-executable planning
status:

| Evidence outcome | Plan status | Action |
|---|---|---|
| `Confluent` | `ConfluentSubset` | Review the evidence, then run `prikk merge` (DC-74) to execute. |
| `Conflict` | `BlockedConflict` | Inspect evidence; conflict resolution is not implemented. |
| `OrderedDependency` | `BlockedOrderedDependency` | Inspect ordering evidence; execution ordering policy is not implemented. |
| `Unsupported` | `BlockedUnsupported` | Inspect unsupported operation evidence. |
| `Deferred` | `BlockedDeferred` | Inspect deferred design evidence. |
| `NotConfluent` | `BlockedNotConfluent` | Inspect replay/final-state mismatch evidence. |
| `EvidenceFailure` | `BlockedEvidenceFailure` | Repair or verify repository evidence before planning. |
| `InvalidCandidate` | `BlockedInvalidCandidate` | Select valid sealed candidates before planning. |

`ConfluentSubset` is intentionally narrow. It means the selected candidates are proven confluent only
for the currently supported subset. It is not a whole-merge guarantee and does not mean Prikk can
create a merge commit.

## Privacy and Output Limits

Evidence and plan output are intended for human diagnostics, not as durable machine-readable schema.
The current display model avoids raw replacement text, raw text spans, blob bytes, absolute host
paths, `.prikk` private paths, signer secrets, key material, arbitrary object debug dumps, and raw
operation payloads. Displayed paths are repository-relative when available and safe.

## Deferred Work

**`prikk merge` (DC-74) executes confluent merges** — see the [merge guide](../guide/merge.md). Still
deferred: automatic merge-base discovery, branch merge semantics beyond a two-sided confluent merge,
conflict resolution, active-WAL merge drafts, worktree conflict materialization, conflict-resolution
UI, persisted proof/witness/merge-evidence/merge-plan objects, same-node text operational transforms,
path-scoped analysis, display-path filtering, JSON output, patch-algebra crate extraction, and public
stable Rust APIs for replay, patch algebra, merge evidence, or merge planning internals.

## Claim-to-Source Anchors

| Claim | Source anchors |
|---|---|
| Pair classification uses `Independent`, `OrderedDependency`, `Conflict`, and `Unknown`. | [`types.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/patch_algebra/types.rs), [`classify.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/patch_algebra/classify.rs), [DC-16](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-16-PATCH-ALGEBRA-FOUNDATION.md) |
| Commutation requires classifier independence plus replay-both-orders proof. | [`commutation.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/patch_algebra/commutation.rs), [DC-18](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-18-PATCH-ALGEBRA-COMMUTATION-CONFLUENCE.md) |
| Flat confluence checks individual sequence validity, cross-pair commutation, composed replay, and final lifecycle-state equality. | [`commutation.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/patch_algebra/commutation.rs), [`analysis.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/patch_algebra/report/analysis.rs), [DC-18](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-18-PATCH-ALGEBRA-COMMUTATION-CONFLUENCE.md) |
| Required sealed evidence failures are reported separately from ordinary unsupported algebra. | [`evidence.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/patch_algebra/evidence.rs), [`error.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/patch_algebra/report/error.rs), [DC-17](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-17-PATCH-ALGEBRA-EVIDENCE-CONTRACT.md) |
| Merge-evidence public outcomes are `Confluent`, `Conflict`, `OrderedDependency`, `Unsupported`, `Deferred`, `NotConfluent`, `EvidenceFailure`, and `InvalidCandidate`. | [`types.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/patch_algebra/report/types.rs), [`display.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/merge_evidence/display.rs), [DC-21](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-21-MERGE-CONFLICT-EVIDENCE-CONTRACT.md) |
| Reason-code and proof-phase strings are display vocabulary, not persisted object schema. | [`display.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/merge_evidence/display.rs), [`mapping.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/patch_algebra/report/mapping.rs), [DC-21](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-21-MERGE-CONFLICT-EVIDENCE-CONTRACT.md) |
| `merge-evidence` is read-only and requires explicit baseline plus left/right targets. | [`merge_evidence.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/merge_evidence.rs), [DC-22](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-22-PUBLIC-MERGE-EVIDENCE-UX.md), [merge evidence guide](../guide/merge-evidence.md) |
| `merge-plan` maps evidence outcomes to `ConfluentSubset` and `Blocked*` statuses without adding merge execution. | [`merge_plan.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/merge_evidence/merge_plan.rs), [DC-25](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-25-MERGE-PLANNING-SURFACE.md), [merge plan guide](../guide/merge-plan.md) |
| Evidence and plan output avoid raw text spans, replacement text, blob bytes, absolute host paths, and arbitrary object debug dumps. | [`display.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/merge_evidence/display.rs), [DC-21](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-21-MERGE-CONFLICT-EVIDENCE-CONTRACT.md), [DC-23](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-23-MERGE-EVIDENCE-UX-STABILIZATION.md) |
| Patch algebra, merge evidence, and merge plan internals are not public stable Rust APIs. | [DC-20](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-20-REPLAY-BOUNDARY-STABILIZATION.md), [DC-25](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-25-MERGE-PLANNING-SURFACE.md), [implementation status](https://github.com/nabbisen/prikk/blob/main/rfcs/IMPLEMENTATION-STATUS.md) |

## Provenance

This reference consolidates released records through DC-25 and follows the DC-26 documentation-home
model: current-state references live in the published mdBook, while RFCs retain design history and
gating material. It does not change code, schema, CLI behavior, merge semantics, or public API
stability.
