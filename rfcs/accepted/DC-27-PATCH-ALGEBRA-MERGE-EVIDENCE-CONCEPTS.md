# RFC (proposed) - DC-27 Patch Algebra and Merge-Evidence Concepts Reference

**Status.** Proposed for architect review.
**Target release.** 0.17.1 candidate unless bundled with a feature release.
**Tracks.** TASK-08 patch algebra and merge-evidence concepts reference.
**Touches.** mdBook reference documentation, merge-evidence / merge-plan concept vocabulary,
claim-to-source anchors, roadmap/status docs.
**Companion handoff.** None. This is a current-state documentation reference and does not create a
gating FDD.

## Context

DC-16 through DC-18 added the internal patch-algebra foundation: pair classification, scoped evidence,
commutation proof requirements, and flat two-sequence confluence checks. DC-21 created the internal
merge/conflict evidence report vocabulary. DC-22 exposed that evidence through `prikk merge-evidence`;
DC-23 stabilized its text UX; DC-25 added `prikk merge-plan` as a read-only planning classification
over the same explicit-input evidence path.

The public documentation now has command pages for `merge-evidence` and `merge-plan`, but it lacks the
concept page that explains what those commands are reporting. Readers can see words such as
`Confluent`, `Conflict`, `OrderedDependency`, `EvidenceFailure`, and `ConfluentSubset` before they have
a reviewed current-state explanation of operation order, `op_seq`, preconditions, commutation, proof
scope, and why successful output still does not mean Prikk can execute a merge.

TASK-08 exists to close that gap. It is a documentation-reference increment only. The authoritative
home follows DC-26: `docs/src/reference/patch-algebra.md`, not `rfcs/fdds/`.

## Problem

1. **Command output is under-explained.** `merge-evidence` and `merge-plan` are intentionally honest
   but dense. Without a concept reference, users must infer meanings from historical DCs and code.
2. **The core project premise is not reader-facing enough.** Prikk presents itself as a
   block-oriented patch-theory VCS, but the published book does not yet explain the current patch
   algebra subset in one place.
3. **Over-trust risk is high.** Terms like `Confluent` and `ConfluentSubset` can sound stronger than
   the implementation permits. The docs must make clear that current evidence is read-only, scoped,
   and non-executing.
4. **The source trail is fragmented.** Relevant truth is split across DC-16, DC-17, DC-18, DC-21,
   DC-22, DC-23, DC-25, `crates/prikk-store/src/patch_algebra/`, and
   `crates/prikk-store/src/merge_evidence/`.

## Design Goals

1. Add a self-contained current-state reference page at `docs/src/reference/patch-algebra.md`.
2. Explain operations, operation order, `op_seq`, preconditions, node/path/blob evidence, and scoped
   evidence at the level needed to understand current command output.
3. Explain the current pair-classification vocabulary: `Independent`, `OrderedDependency`,
   `Conflict`, and `Unknown`, without presenting internals as stable public API.
4. Explain current commutation and confluence proof requirements: classifier independence,
   replay-both-orders proof, individual replay validity, composed replay, and final lifecycle-state
   equality.
5. Explain the DC-21/DC-23 evidence outcomes surfaced by `merge-evidence`: `Confluent`, `Conflict`,
   `OrderedDependency`, `Unsupported`, `Deferred`, `NotConfluent`, `EvidenceFailure`, and
   `InvalidCandidate`.
6. Explain the reason-code and proof-phase vocabulary printed beside evidence outcomes, including
   load-bearing distinctions such as `pair_conflict`, `missing_required_evidence`, and
   `classification`.
7. Explain the DC-25 plan mapping, especially `Confluent` to `ConfluentSubset`, without implying merge
   execution or whole-merge readiness.
8. Keep all honesty caveats visible: read-only evidence, explicit baseline inputs, no merge-base
   discovery, no merge execution, no branch publication, no persisted proof/witness/plan objects, no
   same-node text operational transforms, and no scoped/path-limited analysis.
9. Include visible claim-to-source anchors linking each major claim to code paths or released DCs.

## Non-goals

DC-27 does not add:

- code, schema, or CLI behavior;
- `prikk merge` or merge execution;
- automatic merge-base discovery;
- branch merge semantics, branch publication, merge commits, or multi-parent Blocks;
- worktree conflict materialization or conflict-resolution UI;
- persisted proof, witness, merge-evidence, merge-plan, or conflict objects;
- JSON or stable machine-readable merge-evidence / merge-plan output;
- scoped/path-limited merge analysis or display-path filtering;
- same-node text operational transforms;
- patch-algebra crate extraction;
- public stable Rust API for `prikk-replay`, `prikk-store`, patch algebra, merge evidence, or merge
  plan internals;
- new current-state FDDs under `rfcs/fdds/`.

## Proposed Documentation Shape

Create:

```text
docs/src/reference/patch-algebra.md
```

Add it under the mdBook `# Reference` section after the data model and trust/threat pages:

```md
- [Patch Algebra and Merge Evidence](reference/patch-algebra.md)
```

The page should be written as a current-state reference, not a tutorial and not a future design. It
should link to the existing command guides rather than duplicating command syntax:

- `docs/src/guide/merge-evidence.md`;
- `docs/src/guide/merge-plan.md`.

### Required Sections

The implemented page should contain at least:

1. **Core Caveats.** Early implementation status, read-only evidence, explicit baseline inputs,
   supported-subset scope, and deferred execution/merge-base/worktree/materialization behavior.
2. **Patch Operations and Ordering.** Patch operations are ordered; `op_seq` in evidence output
   identifies the operation sequence position shown to users; preconditions and node/path/blob facts
   constrain replay and classification.
3. **Pair Classification.** Current internal classification categories and their public meaning.
4. **Commutation.** Current commutation requires proof, not intent metadata or name similarity.
5. **Flat Confluence.** Current confluence is for two explicit candidate sequences over an explicit
   baseline; it is not automatic branch merge semantics.
6. **Evidence Outcomes.** Public outcome vocabulary and when each outcome should be used.
7. **Reason Codes and Proof Phases.** The diagnostic `reason:` and `phase:` fields printed by the
   public commands, including reason-code examples such as `proven_confluent`, `pair_conflict`,
   `ordered_dependency`, `missing_required_evidence`, `malformed_required_evidence`,
   `wrong_type_required_evidence`, and proof phases such as `classification`,
   `replay-both-orders`, `flatness`, and `final-state-comparison`.
8. **Merge Plan Mapping.** How `merge-plan` maps outcomes into `ConfluentSubset` and `Blocked*`
   statuses.
9. **Privacy and Output Limits.** Evidence output must avoid raw replacement text, raw spans, blob
   bytes, absolute host paths, `.prikk` internals, signer material, and arbitrary object dumps.
10. **Deferred Work.** Same-node text transforms, merge execution, merge-base discovery, persisted
   proofs/witnesses, path filters, JSON output, extraction, and stable public APIs.
11. **Claim-to-Source Anchors.** A visible table tying claims to released DCs and code paths.
12. **Provenance.** State that the page consolidates released records through DC-25 and follows the
   DC-26 documentation-home model.

## Required Claim Boundaries

The implementation must say, in public docs:

- `merge-evidence` and `merge-plan` are read-only.
- Current analysis requires explicit baseline, left target, and right target inputs.
- `Confluent` means the current supported analysis proved the selected sequences confluent under its
  scoped conditions; it is not a whole-merge or execution-readiness claim.
- `ConfluentSubset` is the planning status for that scoped result and still says merge execution is
  not implemented.
- `EvidenceFailure` is distinct from ordinary unsupported algebra; required sealed evidence failures
  must not be hidden as `Unknown`.
- Intent metadata is advisory and cannot override replay, lifecycle, or proof requirements.
- Same-node text transforms and path-scoped analysis remain deferred.
- Internal Rust types are not stable public APIs.

The implementation must not say or imply:

- that Prikk can execute a merge;
- that Prikk can infer merge bases;
- that `Confluent` or `ConfluentSubset` means a merge commit can be created;
- that evidence output is a stable machine-readable schema;
- that patch algebra is complete for every operation kind or conflict relation;
- that `prikk-replay` or patch-algebra internals are public stable APIs.

## Source Audit Requirements

Implementation must audit at least:

- `rfcs/done/DC-16-PATCH-ALGEBRA-FOUNDATION.md`;
- `rfcs/done/DC-17-PATCH-ALGEBRA-EVIDENCE-CONTRACT.md`;
- `rfcs/done/DC-18-PATCH-ALGEBRA-COMMUTATION-CONFLUENCE.md`;
- `rfcs/done/DC-21-MERGE-CONFLICT-EVIDENCE-CONTRACT.md`;
- `rfcs/done/DC-22-PUBLIC-MERGE-EVIDENCE-UX.md`;
- `rfcs/done/DC-23-MERGE-EVIDENCE-UX-STABILIZATION.md`;
- `rfcs/done/DC-25-MERGE-PLANNING-SURFACE.md`;
- `rfcs/IMPLEMENTATION-STATUS.md`;
- `docs/src/guide/merge-evidence.md`;
- `docs/src/guide/merge-plan.md`;
- `crates/prikk-store/src/patch_algebra/`;
- `crates/prikk-store/src/merge_evidence/`.

The writer may use the scratch TASK-08 file as scheduling context, but claims must be grounded in
tracked code or released RFCs. Local `.git-exclude/specs/` files are not reviewer-facing authority
unless recapped into tracked material.

## Implementation Plan

1. Create `docs/src/reference/patch-algebra.md`.
2. Add it to `docs/src/SUMMARY.md` under `# Reference`.
3. Cross-link from `docs/src/guide/merge-evidence.md` and `docs/src/guide/merge-plan.md` to the
   concept reference where useful.
4. Update `README.md`, `ROADMAP.md`, and `rfcs/IMPLEMENTATION-STATUS.md` only enough to reflect the
   active documentation increment and the new reference after implementation.
5. Do not change Rust code, command output, object schema, or release version during implementation.
6. Prepare an implementation review package after the page is drafted.

## Review Gates

Design review should verify:

- the page scope is current-state reference documentation, not a new feature design;
- the required caveats are sufficient to prevent overclaiming merge readiness;
- the source audit list is sufficient;
- TASK-08 is correctly implemented through the DC-26 documentation-home model;
- no current-state FDD under `rfcs/fdds/` is introduced.

Implementation review should verify:

```text
mdbook build docs
git diff --check
```

and should additionally include:

- proof that `docs/src/reference/patch-algebra.md` is reachable from `docs/src/SUMMARY.md`;
- proof that the `merge-evidence` and `merge-plan` guide pages link to the concept page if cross-links
  are added;
- built-book link/reachability checks for the generated patch-algebra, merge-evidence, and merge-plan
  pages, including checks that no dangling relative links escape `docs/src/` and that required
  absolute repository anchor URLs are present;
- a source-audit checklist showing which released DCs and code paths were checked;
- claim-to-source anchor table review;
- line-count evidence for new/changed docs.

## Acceptance Criteria

DC-27 is complete when:

- `docs/src/reference/patch-algebra.md` exists and is reachable from the mdBook summary;
- the page explains current operation ordering, commutation, confluence, evidence outcomes, and
  reason-code / proof-phase vocabulary, plus merge-plan mapping with visible caveats;
- the page has visible claim-to-source anchors;
- command guide cross-links are updated where useful;
- ROADMAP/status docs track the documentation increment honestly;
- implementation review accepts the documentation; and
- the completed release records DC-27 as documentation-only with no code, schema, or CLI behavior
  change.
