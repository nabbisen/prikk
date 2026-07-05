# DC-22 FDD-01 Update - Public Merge Evidence UX Boundary

Status: Companion for accepted DC-22; design review v1 clarifications folded in
Related RFC: `../../accepted/DC-22-PUBLIC-MERGE-EVIDENCE-UX.md`
Target FDD: FDD-01 Patch Algebra

## Purpose

DC-22 defines the first public, read-only UX for DC-21 merge/conflict evidence. It should let a user
ask for evidence about two explicit sealed candidate histories from an explicit sealed baseline without
creating a merge, inferring a merge base, publishing a Block, writing refs/WAL, or materializing
conflicts in the worktree.

## Required FDD-01 Body Updates

FDD-01 should add:

- a public evidence command boundary, provisionally `prikk merge-evidence`;
- mandatory explicit baseline Block identity for the first public merge-evidence surface;
- explicit left/right target selectors, with block selectors required and ref selectors allowed only
  as narrow current-target resolution;
- a rule that ref selectors do not imply merge-base discovery, branch merge, or publication intent;
- output metadata for both submitted selectors and resolved target Block identities;
- sealed candidate sequence derivation from baseline-exclusive to target-inclusive over single-parent
  Block chains by walking target-to-baseline, reversing the collected Blocks, and concatenating Patch
  references in each Block's canonical stored order;
- fail-closed handling for missing ancestry, multi-parent Blocks, missing/wrong-type/malformed objects,
  corrupt cycles, impossible ancestry, unpublished or corrupt refs, and candidate derivation that
  would need merge-base semantics;
- empty side sequences as report-level diagnostic inputs, not selector errors;
- text output labels for baseline, left target, right target, outcome, reason code, deterministic
  evidence entries, and a read-only note;
- text-only output for v0.15.0, with no JSON or durable machine-readable schema;
- exit-status semantics where every successfully produced DC-21 report outcome is command success;
- privacy rules that public output must not include raw text spans, replacement text, blob bytes,
  absolute host paths, signer secrets, trust private state, or arbitrary object debug dumps on stdout
  or stderr;
- statement that DC-22 does not add merge execution, merge commits, persisted proof/witness objects,
  JSON schema, worktree conflict materialization, or public `prikk-replay` API stabilization.

## Required Tests

- explicit baseline and block-target selector validation;
- ref-target selector validation as narrow current-target aliases;
- baseline-to-target sequence derivation in historical order;
- empty side sequence handling;
- fail-closed missing ancestry;
- fail-closed multi-parent Block if such evidence is constructible in tests;
- wrong-type, malformed, missing, or unreadable sealed evidence maps to a stable error/report path;
- every produced DC-21 report outcome preserves DC-21 reason codes where feasible;
- every produced DC-21 report outcome returns command success;
- malformed arguments and unresolvable selectors return command failure;
- output ordering is deterministic;
- public output redacts raw text spans, replacement text, blob bytes, absolute host paths, and
  arbitrary object debug dumps on stdout and stderr;
- command does not modify object store, refs, active WAL, or worktree files in both successful and
  failing command cases.

## Implementation Errata Checklist

Implementation review must verify:

- `merge-evidence` is read-only and does not share code paths that can publish refs, append active WAL
  records, write objects, or materialize worktree files.
- Candidate derivation uses an explicit baseline and does not infer merge bases.
- Ref selectors are target selectors only and display both selector text and resolved Block ID.
- The CLI display is a view over DC-21 evidence, not a new taxonomy.
- The display view does not freeze internal report fields as durable object schema or a JSON API.
- Read-only proof uses concrete before/after snapshots of object-store entries, ref pointer/log files,
  active WAL paths, and worktree files.
- The command path does not call publication, seal, active-WAL append, checkout/materialization,
  repair, or schema migration helpers.
- `prikk-replay` remains limited to replay/lifecycle semantic substrate.
- Release notes repeat the non-goals: no merge execution, no merge commits, no persisted evidence
  objects, no schema changes, no conflict materialization, and no public `prikk-replay` API stability.
