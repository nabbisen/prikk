# DC-25 Future FDD-01 Input - Merge Planning Surface

Status: Companion for done DC-25
Related RFC: `../../done/DC-25-MERGE-PLANNING-SURFACE.md`
Target future reference: TASK-08 patch algebra / merge-evidence concepts, after DC-26 decides the
documentation home.

## Purpose

DC-25 introduces the first public merge planning boundary after the released merge-evidence command.
It records how Prikk maps existing evidence outcomes into plan statuses while keeping the surface
read-only and explicitly non-executing.

This handoff does not create a current-state FDD-01 reference. TASK-08 owns the later patch-algebra /
merge-evidence concept reference, and DC-26 decides the documentation home before that reference is
authored. DC-25 only records facts that the later reference should include.

It does not create branch merge semantics, merge commits, worktree conflict materialization, or a
stable JSON schema.

## Deferred TASK-08 Reference Inputs

The later TASK-08 patch-algebra / merge-evidence reference should include:

- `prikk merge-evidence` as the evidence-diagnostic surface;
- `prikk merge-plan` as the read-only planning surface;
- explicit baseline and explicit left/right target selectors for the first planning slice;
- the plan-status vocabulary:
  - `ConfluentSubset`;
  - `BlockedConflict`;
  - `BlockedOrderedDependency`;
  - `BlockedUnsupported`;
  - `BlockedDeferred`;
  - `BlockedNotConfluent`;
  - `BlockedEvidenceFailure`;
  - `BlockedInvalidCandidate`;
- the mapping from DC-21 evidence outcome to DC-25 plan status;
- the rule that plan status does not replace evidence outcome/reason code;
- the rule that plan-status text is diagnostic text, not a stable machine-readable schema;
- the rule that command exit status reflects request validity, not whether the plan is blocked;
- the rule that unidentifiable baseline/target inputs are CLI errors, while evidence failures after
  input identification may be blocked plans;
- the read-only invariant: no object, ref, active-WAL, trust, worktree, repair, checkout, rollback, or
  seal writes;
- the privacy invariant inherited from DC-21 through DC-23;
- deferred items: merge execution, automatic merge-base discovery, branch publication, multi-parent
  Blocks, persisted plans/evidence/proofs, conflict resolution, display-path filtering, JSON output,
  and public `prikk-replay` API stabilization.

## Required Tests

- Store-level evidence-outcome to plan-status mapping.
- CLI output for at least a `ConfluentSubset` plan and a blocked conflict plan.
- Missing and ambiguous selector failures.
- Exit `0` for produced blocked plans.
- Exit `1` for invalid arguments and unidentifiable planning inputs.
- Read-only success and failure checks.
- Privacy checks for stdout and stderr.
- Regression checks that `merge-evidence` output remains unchanged unless explicitly reviewed.

## Implementation Errata Checklist

Implementation review must verify:

- `merge-plan` does not call any mutating seal, checkout, rollback, repair, ref publication, WAL append,
  object write, trust write, or worktree materialization path;
- `merge-plan` does not infer merge base or branch merge intent;
- the optional positional argument remains repository root, not a display-path filter;
- `ConfluentSubset` output does not imply a merge commit was created or can be created by DC-25;
- `BlockedEvidenceFailure` and `BlockedInvalidCandidate` preserve the DC-17/DC-21 evidence-scope
  distinctions;
- selector display remains explicit about submitted selectors versus resolved Block identities;
- all displayed paths are repository-relative and safe;
- changed Rust files stay within file-size guidance and test modules remain outside implementation
  files.
