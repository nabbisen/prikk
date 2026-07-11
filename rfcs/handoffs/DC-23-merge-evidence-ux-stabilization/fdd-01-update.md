# DC-23 FDD-01 Update - Public Merge Evidence UX Stabilization

Status: Companion for accepted DC-23
Related RFC: `../../accepted/DC-23-MERGE-EVIDENCE-UX-STABILIZATION.md`
Target FDD: FDD-01 Patch Algebra

## Purpose

DC-23 stabilizes the human-readable `prikk merge-evidence` display after the first public DC-22
release. It clarifies selector summaries, operation pairing, and item counts while keeping the command
read-only and evidence-only.

This is not a merge-execution, display-filtering, or scoped-analysis phase.

## Required FDD-01 Body Updates

FDD-01 should add:

- a post-DC-22 public display stabilization phase for `prikk merge-evidence`;
- text-only output that remains diagnostic and testable, but not a durable external schema;
- explicit display metadata for:
  - baseline Block identity;
  - submitted left/right selectors;
  - resolved left/right target Block identities;
  - left/right operation counts;
  - full-report outcome and reason code;
  - displayed item count and total item count;
- cross-side evidence item display that labels both left and right operation summaries when both are
  known;
- report-level item display that does not invent a fake operation label;
- inherited privacy rules: no raw text spans, replacement text, blob bytes, absolute host paths,
  `.prikk` private paths, signer secrets, key material, or arbitrary object debug dumps;
- statement that DC-23 does not add merge execution, merge commits, merge-base discovery, display-path
  filtering, scoped confluence/conflict analysis, persisted evidence objects, JSON schema, worktree
  conflict materialization, or public `prikk-replay` API stabilization.

## Required Tests

- Existing DC-22 `merge-evidence` selector forms remain valid.
- Invalid or ambiguous selectors still fail before report display.
- Cross-side conflict output labels both sides and both operation summaries.
- Report-level output does not emit fake operation text.
- Selector summaries include submitted selector text and resolved target Block IDs.
- Displayed item count and total item count are present and equal for DC-23 output.
- Output item ordering is deterministic.
- Output redaction covers stdout and stderr.
- Read-only behavior is proven for success and failure cases.

## Implementation Errata Checklist

Implementation review must verify:

- no production merge, branch publication, active WAL append, object write, ref write, worktree write,
  repair, checkout materialization, or schema migration code path is reachable from `merge-evidence`;
- no display-path filter is added in DC-23;
- the optional positional path remains repository root and is not overloaded as a display path;
- selector display remains explicit about submitted selector versus resolved Block identity;
- all displayed paths are repository-relative and safe;
- the display view does not expose raw operation payloads or internal object debug strings;
- `prikk-store` remains the repository integration owner for selector resolution and evidence
  construction;
- `prikk-replay` remains limited to replay/lifecycle semantic substrate;
- changed Rust files stay within file-size guidance and test modules remain outside implementation
  files where required.
