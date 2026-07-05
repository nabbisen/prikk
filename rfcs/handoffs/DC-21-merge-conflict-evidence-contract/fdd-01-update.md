# DC-21 FDD-01 Update - Merge Conflict Evidence Contract

Status: Companion for released DC-21; v1 re-review implementation errata folded in
Related RFC: `../../done/DC-21-MERGE-CONFLICT-EVIDENCE-CONTRACT.md`
Target FDD: FDD-01 Patch Algebra

## Purpose

DC-21 defines the first reviewable/user-facing evidence vocabulary for future merge/conflict surfaces.
It does not execute merges, publish merge commits, persist proof objects, materialize conflicts, or add
a CLI merge command.

The FDD-01 update should make future merge designs depend on evidence categories and stable reason
codes rather than raw internal patch-algebra debug output.

## Required FDD-01 Body Updates

FDD-01 should add:

- evidence categories and primary outcomes: `Confluent`, `Conflict`, `OrderedDependency`,
  `Unsupported`, `Deferred`, `NotConfluent`, `EvidenceFailure`, and `InvalidCandidate`;
- primary outcome precedence:
  1. required sealed evidence failure,
  2. invalid unsealed candidate,
  3. unsupported operation,
  4. deferred relation,
  5. concrete conflict,
  6. ordered dependency,
  7. replay/final-state mismatch,
  8. proven confluence;
- reason-code taxonomy for conflict, ordered dependency, unsupported/deferred algebra, replay proof
  failure, evidence failure, and invalid candidate input;
- evidence-scope mapping:
  - sealed-baseline required evidence failure -> `EvidenceFailure`,
  - sealed-candidate required evidence failure -> `EvidenceFailure`,
  - malformed unsealed candidate input -> `InvalidCandidate`,
  - insufficient optional unsealed candidate evidence -> `InvalidCandidate`, unless the reason is
    genuinely algebraic and maps to `Unsupported` or `Deferred`;
- allowed evidence facts: baseline block id, optional replay horizon, sequence side, operation index,
  `op_seq`, operation kind, node id, repository-relative path, relation category, evidence scope, proof
  phase, and reason code;
- disallowed evidence facts: raw blob bytes, raw text spans, absolute host paths, signer secrets,
  arbitrary object debug dumps, and persisted canonical proof bytes;
- statement that evidence reports are not durable object schema;
- statement that reason codes are release-stable diagnostic vocabulary after v0.14.0 unless superseded
  by a later DC, but not persisted object schema;
- statement that merge execution and conflict materialization remain later designs.

## Required Tests

- every reviewable/user-facing category is exercised;
- required sealed evidence failures remain `EvidenceFailure`, not `Unsupported` or `Deferred`;
- malformed unsealed candidate input is `InvalidCandidate`;
- same-node text relation remains `Deferred`;
- ordered dependencies are not reported as conflicts or confluence evidence;
- baseline block id is present in every report;
- report ordering and primary outcome are deterministic;
- reports do not include raw text/blob bytes or absolute host paths;
- if report formatting helpers are added, formatted output contains no raw text spans, raw blob bytes,
  absolute paths, or arbitrary object debug dumps.

## Implementation Errata Checklist

Implementation review must verify:

- `left_sequence` and `right_sequence` are summaries or labels, not full operation-payload containers.
  Allowed sequence facts are side/label, operation count, operation index, `op_seq`, operation kind,
  node id, repository-relative path, reason code, and proof phase.
- Report types must not store raw operation payload dumps, raw text spans, replacement text, blob
  bytes, or arbitrary decoded-object debug output.
- Reason-code docs and release notes say reason codes are release-stable diagnostic vocabulary for
  tests and future display, not persisted object schema or long-term protocol guarantees.
- Tests pin both primary outcome precedence and deterministic secondary-entry ordering.
- `NotConfluent` stays narrow: replay or final-state mismatch after otherwise supported analysis only.
  Unsupported operations, deferred same-node text, ordered dependencies, concrete conflicts, sealed
  evidence failures, and invalid unsealed candidates must not map to `NotConfluent`.
- Code organization remains internal/read-only and does not add merge CLI, merge execution, branch
  publication, multi-parent Blocks, worktree conflict materialization, persisted proof/witness objects,
  patch-algebra crate extraction, or `prikk-replay` public API stabilization.
- Release notes and implementation status repeat the DC-21 non-goals.
