# RFC (done) - DC-23 Public Merge Evidence UX Stabilization

**Status.** Released in 0.16.0.
**Target release.** 0.16.0.
**Tracks.** Stabilization of the released `prikk merge-evidence` public display boundary after DC-22.
**Touches.** Text display labels, selector summaries, cross-side item rendering, displayed/total item
counts, output determinism, read-only evidence tests, and FDD-01 merge/conflict wording.
**Companion handoff.** `../handoffs/DC-23-merge-evidence-ux-stabilization/fdd-01-update.md`.

## Context

DC-21 released the internal merge/conflict evidence report vocabulary. DC-22 then released the first
public read-only UX over that vocabulary: `prikk merge-evidence` with an explicit baseline and explicit
left/right target selectors.

That first public surface deliberately stayed small. It exposed the report, proved the command is
read-only, kept process success separate from evidence outcomes, and avoided JSON or durable evidence
objects. The next useful step is not merge execution. It is to make the public evidence display easier
to read and safer to build upon before any future branch-merge, merge-plan, or conflict-resolution
surface appears.

DC-23 stabilizes the text UX at the diagnostic boundary. It should make selector identity, operation
pairing, item counts, and report-level output explicit without changing patch algebra semantics or
creating a machine-readable schema.

## Design Goals

1. Improve the human-readable `prikk merge-evidence` output so users can see the baseline, submitted
   selectors, resolved targets, operation counts, top-level outcome, and item counts without parsing
   ambiguous item lines.
2. Make cross-side evidence entries display both left and right operation summaries when both are
   known.
3. Keep all output deterministic, privacy-preserving, and testable.
4. Preserve the DC-21 outcome and reason-code vocabulary.
5. Preserve the DC-22 command boundary: read-only evidence only, no merge execution, no merge-base
   inference, no branch publication, and no persisted proof/witness objects.
6. Avoid freezing a JSON or external Rust API contract.

## Non-goals

DC-23 does not add:

- `prikk merge`;
- merge execution;
- automatic merge-base discovery;
- branch merge semantics, branch publication, merge commits, or multi-parent Blocks;
- branch switching, branch copy/fork, tags, remotes, branch deletion, or branch rename;
- active-WAL merge drafts;
- persisted conflict-witness, proof, merge-plan, or merge-evidence objects;
- worktree conflict files, conflict markers, checkout conflict materialization, or conflict
  resolution UI;
- path display filtering, scoped evidence filtering, or path-focused report display;
- scoped/path-limited merge analysis;
- same-node text operational transforms;
- semantic/language-aware merge;
- JSON, CSV, or stable machine-readable output;
- object schema, repository-layout, ref/WAL/trust, or publication changes;
- patch-algebra crate extraction;
- public stable Rust API for `prikk-replay` or the internal merge-evidence report types.

## Proposed Public UX

### Command Shape

The existing DC-22 command remains valid:

```text
prikk merge-evidence \
  --baseline-block <block-id> \
  (--left-block <block-id>|--left-ref <ref>) \
  (--right-block <block-id>|--right-ref <ref>) \
  [<repository-root>]
```

Rules:

- `--baseline-block` remains required.
- Each side must still choose exactly one selector: `--left-block` or `--left-ref`, and
  `--right-block` or `--right-ref`.
- The optional positional argument remains the repository root, as in DC-22.
- DC-23 intentionally does not add a path filter or display filter. The current CLI convention already
  uses an optional positional path as the repository root, and a path-focused display option can be
  mistaken for scoped merge/confluence analysis. Any future display-path option requires a later DC.

### Stabilized Text Output

The default output remains text-only. DC-23 stabilizes the shape enough for human diagnostics and
regression tests, without making it a durable external schema.

Preferred shape:

```text
merge evidence
baseline block: <block-id>
left selector: ref heads/topic-a
left target block: <block-id>
left operations: 3
right selector: block <block-id>
right target block: <block-id>
right operations: 2
outcome: Conflict
reason: pair_conflict
items: 1 displayed of 1

cross:
  left[0] op_seq=1 ChangePerm src/lib.rs
  right[0] op_seq=1 ChangePerm src/lib.rs
  outcome: Conflict
  reason: pair_conflict
  phase: classification

note: read-only evidence; no merge commit, ref update, WAL write, or worktree change was performed
```

Display requirements:

- show submitted selector text and resolved target Block identity for both sides;
- show left and right operation counts separately;
- show the full-report outcome and reason before item details;
- show displayed item count and total report item count, even though DC-23 has no display filter. In
  DC-23 these counts should be equal for normal report output, but the two-count form keeps the text
  ready for future display narrowing without redefining the field;
- for cross-side items, show left and right operation summaries on separate indented lines when both
  operation summaries are known;
- for single-side items, show the side and operation summary on one indented line;
- for report-level items, show `report` without printing a fake operation label;
- preserve DC-21 outcome names and reason-code names exactly;
- preserve deterministic item ordering from the DC-21 report;
- avoid wording that implies `Confluent` means this command can create a merge commit.

The implementation may keep compatibility with existing DC-22 labels where they remain clear, but
DC-23 should remove ambiguous cross-item lines that make it unclear which side an operation belongs to.

### Privacy and Redaction

DC-23 inherits the DC-21/DC-22 privacy rules. Public output must not include:

- raw text spans;
- replacement text;
- blob bytes or binary payloads;
- absolute host paths;
- repository-private `.prikk` paths;
- signer secrets, seed material, public-key trust internals, or arbitrary key material;
- arbitrary object debug dumps;
- panic messages or backtraces in normal error paths.

### Exit Status

DC-23 preserves DC-22 exit semantics:

| Condition | Exit |
|---|---:|
| Valid request and DC-21 evidence report produced, for any DC-21 outcome | 0 |
| Invalid CLI arguments, missing selectors, or ambiguous selectors | 1 |
| Selector/ancestry/object/ref failure prevents construction of the requested report | 1 |
| Unexpected internal error | 1 |

### Store and CLI Boundary

`prikk-store` continues to own request validation, selector resolution, sealed candidate sequence
derivation, and report construction.

The CLI may own the final text rendering, but the display view supplied by `prikk-store` should expose
enough structured data to render unambiguous item lines without inspecting internal patch-algebra
types.

The display view should make the full report outcome and reason available, distinguish displayed item
count from total item count, and keep every rendered path as a safe repository-relative value. In
DC-23, because no display filter exists, displayed and total item counts are expected to be equal for
normal report output. Keeping both counts in the view and renderer is still useful because it makes
the text explicit that the count is a display count, not a hidden merge operation count.

No object, ref, WAL, or worktree writes are allowed.

`prikk-replay` remains responsible only for replay/lifecycle semantic substrate. DC-23 must not move
store-backed resolver construction, object reading, lineage walking, patch algebra, lifecycle-cache
persistence, or worktree behavior into `prikk-replay`.

## Required Tests

Implementation review must require focused tests for:

- existing DC-22 command forms still parse and run;
- cross-side conflict items display both left and right operation summaries with side labels;
- report-level items do not print a fake operation label;
- selector summaries include both submitted selector text and resolved target Block identity;
- displayed item count and total item count are present and equal for unfiltered DC-23 output;
- output ordering is deterministic;
- stdout and stderr do not leak raw content, absolute host paths, `.prikk` paths, or debug dumps;
- successful and failing invocations remain read-only for objects, refs, active WAL, and worktree
  files;
- no standalone integration-test helper target is accidentally introduced under `tests/`.

## Review Gates

Implementation review should include at least:

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
TMPDIR=<workspace-local tmp> cargo test -p prikk --test merge_evidence --quiet
cargo test -p prikk-store merge_evidence --quiet
TMPDIR=<workspace-local tmp> cargo test --workspace --quiet
git diff --check
```

The review request should include line-count evidence for changed Rust files, especially integration
tests and display/view-model modules.

## Resolved Design Review Decisions

The first architect design review accepted the display-stabilization slice and recommended deferring
the proposed path-focused display filter. DC-23 records that decision: no `--focus-path`,
`--display-path`, `--show-path`, or equivalent filter is part of 0.16.0.

If a later DC adds display-path filtering, it must define at least:

- an option name that clearly communicates display-only behavior;
- whether one or many paths are accepted;
- segment-boundary matching so `src/lib.rs` does not match `src/lib.rs.bak` and `src` does not match
  `src2`;
- cross-item matching when left and right operation paths differ;
- explicit output wording that the full-report outcome is computed over the full candidate sequences,
  not the filtered view.

For DC-23, implementation should stabilize the semantic fields, their presence, and their ordering,
but should not claim byte-for-byte text labels are a durable machine-readable schema.
