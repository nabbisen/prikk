# RFC (accepted) - DC-22 Public Merge Evidence UX Boundary

**Status.** Accepted for implementation after design review v1 clarifications.
**Target release.** v0.15.0.
**Tracks.** First public read-only merge/conflict evidence surface after DC-21.
**Touches.** CLI evidence display, store-backed evidence request construction, explicit candidate
selection, report redaction, output stability, and FDD-01 merge/conflict wording.

## Context

DC-21 released an internal, read-only merge/conflict evidence report vocabulary. It gives Prikk a
stable diagnostic taxonomy for `Confluent`, `Conflict`, `OrderedDependency`, `Unsupported`,
`Deferred`, `NotConfluent`, `EvidenceFailure`, and `InvalidCandidate`, plus deterministic report
entries and privacy rules.

That report is still not a user workflow. A developer cannot yet ask Prikk, from the CLI, "what would
Prikk know about these two candidate histories?" The next step should make the evidence visible
without crossing into merge execution, automatic merge-base discovery, branch publication, or worktree
conflict materialization.

DC-22 defines the first public evidence UX boundary. It should expose a read-only explanation surface
over sealed repository objects and keep every input explicit enough that later branch-merge semantics
are not accidentally frozen.

## Design Goals

1. Add a public read-only UX for explaining merge/conflict evidence using the DC-21 report taxonomy.
2. Require an explicit sealed baseline for the first public surface; do not infer a merge base.
3. Allow explicit left/right candidate selection from sealed object identity, with optional ref-target
   selectors only as target resolution, not merge-base semantics.
4. Preserve DC-21 diagnostic categories, reason codes, primary precedence, and privacy rules.
5. Keep output concise enough for a developer to scan, while preserving stable reason codes for tests
   and future tooling.
6. Keep the public surface non-mutating: no active WAL writes, ref updates, object writes, worktree
   writes, lock acquisition for publication, or merge commits.
7. Keep repository integration in `prikk-store`; keep patch algebra and resolver construction out of
   `prikk-replay`.

## Non-goals

DC-22 does not add:

- merge execution;
- `prikk merge` as a command that creates a merge result;
- automatic merge-base discovery;
- branch copy/fork, branch switching, branch merge, branch deletion, tags, or remotes;
- multi-parent Blocks or merge commit publication;
- active-WAL merge drafts;
- persisted conflict-witness, proof, merge-plan, or merge-evidence objects;
- worktree conflict files, conflict markers, checkout conflict materialization, or resolution UI;
- same-node text operational transforms;
- semantic/language-aware merge;
- schema, repository-layout, ref/WAL/trust, or publication changes;
- public stable Rust API for `prikk-replay`;
- patch-algebra crate extraction.

## Proposed Public UX

### Command

The first public CLI surface should be a read-only command:

```text
prikk merge-evidence \
  --baseline-block <block-id> \
  --left-block <block-id> \
  --right-block <block-id> \
  [<repo-path>]
```

The command name is intentionally not `merge`. It reports evidence only.

DC-22 accepts target-ref conveniences under narrow rules:

```text
prikk merge-evidence \
  --baseline-block <block-id> \
  --left-ref heads/topic-a \
  --right-ref heads/topic-b \
  [<repo-path>]
```

Rules:

- `--baseline-block` is required in DC-22.
- each side must choose exactly one target selector: `--left-block` or `--left-ref`, and
  `--right-block` or `--right-ref`;
- ref selectors resolve only to current target Blocks through existing ref-state validation;
- ref selectors do not imply branch merge, merge-base discovery, ancestry search between refs, or
  publication intent;
- display metadata must show both the submitted selector and the resolved target Block identity;
- if a selector is malformed, missing, ambiguous, or points to missing/corrupt evidence, the command
  exits non-zero with an evidence/selector error and does not write anything.

### Candidate Sequence Resolution

For each side, the implementation should derive a sealed candidate sequence by walking the selected
target Block's single-parent chain back to the explicit baseline Block.

The resolution is valid only when:

- the baseline Block exists, decodes as a Block, and is reachable from the target through single-parent
  Blocks;
- every Block on the side chain is readable, identity-valid, and single-parent after the baseline;
- every referenced Patch is readable, identity-valid, and decodes as a supported Patch envelope for
  the existing DC-21 evidence adapter;
- the derived patch order is the historical order from baseline-exclusive to target-inclusive:
  walk target-to-baseline by parent links, reverse the collected Blocks, then concatenate Patch
  references in each Block's canonical stored order;
- left and right target selection is deterministic and independent.

The command must fail closed when:

- the baseline is not an ancestor of a target;
- a multi-parent Block is encountered;
- a corrupt cycle or impossible ancestry condition is detected;
- an object is missing, wrong-type, malformed, or unreadable;
- a ref selector resolves to an unpublished or corrupt ref;
- candidate sequence derivation would require branch merge-base semantics.

Object/evidence failures discovered while resolving required sealed inputs should map to the DC-21
`EvidenceFailure` category when a report can still be constructed. Selector or argument errors that
prevent identifying the requested evidence may remain ordinary CLI errors.

A side may resolve to an empty baseline-to-target sequence when its target equals the baseline. Empty
sequence evidence is a valid diagnostic input, not a selector error. If both sides are empty, the
preferred outcome is deterministic identity evidence, usually `Confluent`, when supported by the
DC-21 adapter. If the current adapter cannot represent empty-sequence evidence, the command should
produce a DC-21 report outcome such as `Unsupported` or `InvalidCandidate` with exit `0`, not a
pre-report CLI failure.

### Output Shape

The default output should be human-readable text with stable labels. It should be concise by default:

```text
merge evidence
baseline block: <block-id>
left selector: ref heads/topic-a
left target block: <block-id>
right selector: block <block-id>
right target block: <block-id>
outcome: Conflict
reason: pair_conflict
items: 1

cross left[0] EditText src/lib.rs <-> right[0] EditText src/lib.rs
  outcome: Conflict
  reason: pair_conflict

note: read-only evidence; no merge commit, ref update, WAL write, or worktree change was performed
```

Display rules:

- preserve exact DC-21 outcome and reason-code names;
- show baseline, submitted selectors, resolved target identities, operation side/index, operation
  kind, repository-relative path, proof phase, evidence scope, and reason code when present;
- do not print raw text spans, replacement text, blob bytes, absolute host paths, signer secrets,
  trust private state, or arbitrary object debug dumps;
- sort entries exactly as the DC-21 report sorts them;
- print a short non-mutating note;
- avoid implying that `Confluent` means a merge commit can be safely created by this command.

The first implementation must not add JSON output. Text output is enough for v0.15.0. Stable labels
are intended for human diagnostics and regression tests, but they are not a durable external schema.
If a future JSON output is added, it should be a later DC because it freezes more structure than a
diagnostic CLI display.

### Exit Status

The command should use process status to separate command success from evidence outcome:

| Condition | Exit |
|---|---:|
| Valid request and DC-21 evidence report produced, for any DC-21 outcome | 0 |
| Invalid CLI arguments, missing required selectors, or ambiguous selectors | 1 |
| Selector/ancestry/object/ref failure prevents construction of the requested report | 1 |
| Unexpected internal error | 1 |

Evidence outcomes are not command failures. `Confluent`, `Conflict`, `OrderedDependency`,
`Unsupported`, `Deferred`, `NotConfluent`, `EvidenceFailure`, and `InvalidCandidate` all exit `0`
when the command successfully identifies the requested baseline/targets and produces a DC-21 report.

## Store API Boundary

`prikk-store` should own request validation and evidence construction. A possible shape is:

```text
MergeEvidenceRequest {
  baseline_block_id,
  left_target,
  right_target,
}

MergeEvidenceTarget =
  Block(block_id)
  Ref(ref_name)
```

The store-level helper should:

- resolve explicit selectors;
- derive sealed candidate sequences from the explicit baseline to each target in historical order;
- call the DC-21 report adapter;
- return a report plus public display metadata, including submitted selectors and resolved target
  Block IDs;
- avoid object writes, ref writes, active-WAL writes, and worktree writes.

The DC-21 internal report shape may remain internal. DC-22 may add a thin CLI-facing view model or
formatter so CLI output does not freeze every internal report field as a public API.

`prikk-replay` remains responsible only for replay/lifecycle semantic substrate. DC-22 must not move
store-backed resolver construction, object reading, patch algebra, lifecycle-cache persistence, or
worktree behavior into `prikk-replay`.

## Security and Privacy

DC-22 inherits DC-21 privacy requirements and applies them to public display:

- evidence entries must not include raw text bytes, replacement text, blob bytes, absolute host paths,
  signer secrets, private trust state, or arbitrary decoded-object debug output;
- repository-relative paths, node ids, object ids, operation kinds, sequence labels, evidence scopes,
  proof phases, outcomes, and reason codes are acceptable;
- CLI output should not expose more object payload detail than needed to identify the evidence
  relation;
- display tests must cover redaction for text spans, replacement text, blob bytes, and absolute paths;
- redaction applies to stdout, stderr, normal report output, malformed/corrupt object errors,
  debug-derived display paths, and test assertion helper text where practical.

## Migration Plan

### Phase 1 - Request and Resolution Design

- Add request/target types for explicit baseline and left/right targets.
- Add block/ref selector validation.
- Derive linear candidate sequences from explicit baseline to each target.
- Fail closed on missing ancestry, multi-parent Blocks, wrong object type, malformed evidence, and
  unpublished/corrupt refs.
- Accept empty side sequences as report-level diagnostic inputs, not selector errors.

### Phase 2 - Display Boundary

- Add a CLI-facing view or formatter over DC-21 reports.
- Preserve outcome/reason-code names.
- Keep output deterministic, concise, and privacy-preserving.
- Add the `prikk merge-evidence` command only as read-only evidence display.

### Phase 3 - Tests

- Add store tests for explicit baseline/target sequence derivation.
- Add CLI tests for successful confluence/conflict/deferred/unsupported display as feasible.
- Add tests that every produced DC-21 report outcome returns exit 0, while malformed requests return
  exit 1.
- Add privacy tests for text/blob/path redaction on stdout and stderr.
- Add before/after mutation-audit tests proving no WAL, ref pointer/log, object store, or worktree
  files are modified for both successful and failing command cases.

### Phase 4 - Documentation

- Update README, ROADMAP, implementation status, and release notes.
- Add or update FDD-01 handoff wording for public evidence UX.
- Keep release notes explicit that merge execution remains deferred.

## Release and Compatibility Rules

DC-22 must not change:

- object ids or canonical payload bytes;
- patch identity, operation order, or replay semantics;
- lifecycle semantics;
- text-span identity or inverse behavior;
- repository layout;
- ref/WAL/trust behavior;
- verification or doctor semantics;
- existing checkout, rollback, commit, seal, log, status, worktree-status, or trust behavior.

## Test and Review Requirements

Implementation review should include:

- `cargo test --workspace`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- `cargo fmt --check`;
- `git diff --check`;
- focused store tests for baseline-to-target sequence resolution;
- focused CLI tests for output labels, reason codes, and exit status;
- privacy tests proving no raw text spans, replacement text, blob bytes, absolute host paths, or
  arbitrary object debug dumps appear in merge-evidence stdout or stderr;
- mutation audit proving the command performs no object/ref/WAL/worktree writes by snapshotting
  object-store entries, ref pointer/log files, active WAL paths, and worktree files before and after
  both successful and failing command cases;
- audit that the command path does not call publication, seal, active-WAL append,
  checkout/materialization, repair, or schema migration helpers;
- line-count and test-module placement audit;
- explicit statement that no merge execution, merge commit publication, schema change, worktree
  conflict materialization, patch-algebra extraction, or `prikk-replay` public API stabilization was
  added.

## Open Questions

1. Should DC-22 accept ref selectors, or only block selectors?
   Answer: accept ref selectors as narrow current-target aliases. `--baseline-block` remains
   mandatory, each side chooses exactly one selector, and output shows both submitted selector and
   resolved Block ID.
2. Should empty left/right sequences be valid when a target equals the baseline?
   Answer: yes. Empty side sequences are valid diagnostic inputs. If unsupported by the current
   adapter, they produce a DC-21 report outcome with exit `0`, not a selector error.
3. Should the output include a machine-readable format?
   Answer: no; keep v0.15.0 text-only to avoid freezing a JSON schema.
4. Should `Confluent` exit with a different status from `Conflict`?
   Answer: no. Every successfully produced DC-21 report outcome exits `0`.
5. Should CLI display use the internal DC-21 report directly?
   Answer: no; use a thin display view/formatter so internal report evolution remains possible.

## Acceptance Criteria

DC-22 design is accepted when review agrees on:

- explicit baseline and target-selection rules;
- whether ref selectors are accepted in v0.15.0;
- baseline-to-target sequence derivation and fail-closed cases;
- public display shape, labels, reason-code stability, and privacy rules;
- exit-status semantics;
- store/CLI/replay crate boundaries;
- explicit deferral of merge execution, merge-base discovery, merge commits, schema changes,
  persisted evidence objects, and worktree conflict materialization;
- implementation test and review gates.
