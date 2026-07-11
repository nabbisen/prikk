# RFC (done) - DC-25 Merge Planning Surface

**Status.** Done on main; prepared for release in 0.17.0.
**Target release.** 0.17.0.
**Tracks.** First public, non-mutating merge planning surface after DC-21 through DC-23 merge evidence.
**Touches.** CLI planning UX, store-backed plan construction, merge evidence classification mapping,
read-only mutation guarantees, output vocabulary, future TASK-08 patch-algebra/merge-reference input,
and future merge execution boundaries.
**Companion handoff.** `../handoffs/DC-25-merge-planning-surface/fdd-01-update.md`.

## Context

DC-21 created the internal merge/conflict evidence contract. DC-22 exposed that contract through the
read-only `prikk merge-evidence` command with explicit baseline and explicit left/right target
selectors. DC-23 stabilized the diagnostic text output.

That sequence answers "what evidence does Prikk see?" It does not yet answer the genuinely user-facing
"plan my merge" workflow: Prikk still cannot infer the merge base, execute a merge, publish a merge
commit, or materialize conflicts. DC-25 deliberately does not claim that value increment. It remains
an explicit-input, read-only planning boundary for future execution to consume or supersede.

DC-25 defines a plan classification layer over the existing evidence report. It separates
non-executable but potentially useful confluent-subset evidence from blocked cases, preserves the
underlying evidence outcome and reason, and states why no merge action is available yet. This is the
fourth read-only step in the merge/evidence line; the later user-value step is still merge-base
discovery plus merge execution.

The surface must remain read-only and explicit. It must not implement merge execution, automatic
merge-base discovery, merge commits, active-WAL merge drafts, or conflict resolution. The goal is to
create a reviewable plan boundary without changing the existing evidence command into a mutating
workflow.

## Design Goals

1. Add a public read-only `prikk merge-plan` boundary over explicit baseline and explicit left/right
   targets.
2. Introduce a plan-status vocabulary that maps the DC-21/DC-23 evidence outcome into non-executable
   planning classifications without hiding the underlying evidence outcome/reason.
3. Preserve the existing `prikk merge-evidence` command and its diagnostic meaning.
4. Keep command success separate from plan status: a valid plan request exits successfully even when
   the plan is blocked by conflict, unsupported algebra, deferred design, or evidence failure.
5. Keep all inputs explicit in this first planning slice; do not infer merge bases or branch merge
   intent.
6. Keep output text-only, deterministic, privacy-preserving, and not a durable machine-readable schema.
7. Prove the planning command is read-only for success and failure paths.
8. Keep repository integration and selector resolution in `prikk-store`; keep `prikk-replay`
   workspace-internal.

## Non-goals

DC-25 does not add:

- `prikk merge` as a mutating command;
- merge execution, merge commits, multi-parent Blocks, or branch publication;
- automatic merge-base discovery;
- branch merge semantics, branch copy/fork, branch switching, tags, remotes, branch deletion, or
  branch rename;
- active-WAL merge drafts or worktree conflict materialization;
- conflict markers, conflict-resolution UI, or worktree merge application;
- persisted merge-plan, merge-evidence, proof, witness, or conflict objects;
- same-node text operational transforms;
- semantic/language-aware merge;
- path-scoped merge analysis or display-path filtering;
- JSON, CSV, or stable machine-readable output;
- object schema, repository-layout, ref/WAL/trust, or publication changes;
- patch-algebra crate extraction;
- public stable Rust API for `prikk-replay` or internal merge-evidence/merge-plan types.

## Proposed Public UX

### Command Shape

The first planning command should be:

```text
prikk merge-plan \
  --baseline-block <block-id> \
  (--left-block <block-id>|--left-ref <ref>) \
  (--right-block <block-id>|--right-ref <ref>) \
  [<repository-root>]
```

Rules:

- `--baseline-block` is required.
- Each side must choose exactly one selector: `--left-block` or `--left-ref`, and `--right-block` or
  `--right-ref`.
- Ref selectors resolve only to current local branch target Blocks through existing RefState
  validation.
- The optional positional argument remains the repository root. It is not a path filter.
- The command must show submitted selectors and resolved target Block identities.
- The command must not infer merge base, branch ancestry intent, publication intent, or worktree
  target.

The command name is intentionally `merge-plan`, not `merge --plan-only`, for the first planning slice.
It keeps the public boundary separate from future mutating `prikk merge` semantics. A later DC may
decide whether `prikk merge --plan-only` aliases or replaces this command.

### Plan Status Vocabulary

DC-25 adds a plan-level status derived from the evidence report:

| Plan status | Meaning |
|---|---|
| `ConfluentSubset` | The selected candidates are proven confluent for the current supported operation subset. This is not a whole-merge or execution-readiness guarantee. |
| `BlockedConflict` | The evidence report found a concrete conflict. User resolution design is required before execution. |
| `BlockedOrderedDependency` | The evidence report found an ordered dependency. A future execution design would need an ordering/sequence policy before applying anything. |
| `BlockedUnsupported` | The request contains operation kinds or relations outside the supported algebra subset. |
| `BlockedDeferred` | The relation is in the supported domain but intentionally deferred, such as same-node text transforms. |
| `BlockedNotConfluent` | Replay or final-state comparison failed after otherwise supported analysis. |
| `BlockedEvidenceFailure` | Required sealed evidence is missing, malformed, unreadable, wrong-type, or identity-invalid. |
| `BlockedInvalidCandidate` | Candidate input is malformed or incomplete before analysis can produce a usable plan. |

Mapping from DC-21 evidence outcomes:

| Evidence outcome | Plan status |
|---|---|
| `Confluent` | `ConfluentSubset` |
| `Conflict` | `BlockedConflict` |
| `OrderedDependency` | `BlockedOrderedDependency` |
| `Unsupported` | `BlockedUnsupported` |
| `Deferred` | `BlockedDeferred` |
| `NotConfluent` | `BlockedNotConfluent` |
| `EvidenceFailure` | `BlockedEvidenceFailure` |
| `InvalidCandidate` | `BlockedInvalidCandidate` |

The plan must preserve the original evidence outcome and reason code. The plan status is an
additional user-facing planning classification, not a replacement for DC-21 evidence vocabulary.
Plan-status text is diagnostic text for this release line, not a stable machine-readable schema.

The eight statuses are intentionally not only a spelling change from evidence outcomes:

- the plan status carries the execution decision layer (`ConfluentSubset` or `Blocked*`);
- the evidence outcome/reason remains visible as the diagnostic source;
- the `action` field explains the next available user action and the still-deferred capability;
- future merge execution can consume the plan classification without changing the evidence report
  vocabulary.

DC-25 therefore keeps the one-to-one mapping but treats the plan status as a non-executable go/no-go
classification layer. It does not claim that any status is directly executable.

### Plan Summary Shape

The store-level plan shape may remain internal, but the design shape is:

```text
MergePlan {
  baseline_block_id,
  left_selector,
  left_target_block_id,
  left_operation_count,
  right_selector,
  right_target_block_id,
  right_operation_count,
  status,
  evidence_outcome,
  evidence_reason,
  action,
  items,
}
```

`action` is a short stable explanation of the next available user action:

| Plan status | Action text intent |
|---|---|
| `ConfluentSubset` | "review only; merge execution is not implemented" |
| `BlockedConflict` | "inspect evidence; conflict resolution is not implemented" |
| `BlockedOrderedDependency` | "inspect ordering evidence; execution ordering policy is not implemented" |
| `BlockedUnsupported` | "inspect unsupported operation evidence" |
| `BlockedDeferred` | "inspect deferred design evidence" |
| `BlockedNotConfluent` | "inspect replay/final-state mismatch evidence" |
| `BlockedEvidenceFailure` | "repair or verify repository evidence before planning" |
| `BlockedInvalidCandidate` | "select valid sealed candidates before planning" |

The action wording may change during implementation review, but it must not imply that DC-25 can
write a merge result.

### Text Output

Default output should be concise and text-only:

```text
merge plan
baseline block: <block-id>
left selector: ref heads/topic-a
left target block: <block-id>
left operations: 3
right selector: block <block-id>
right target block: <block-id>
right operations: 2
status: BlockedConflict
evidence outcome: Conflict
reason: pair_conflict
action: inspect evidence; conflict resolution is not implemented
items: 1 displayed of 1

cross:
  left[0] op_seq=1 ChangePerm src/lib.rs
  right[0] op_seq=1 ChangePerm src/lib.rs
  outcome: Conflict
  reason: pair_conflict
  phase: classification

note: read-only plan; no merge commit, ref update, WAL write, object write, or worktree change was performed
```

Display requirements:

- preserve submitted selector text and resolved target Block identity for both sides;
- preserve left/right operation counts;
- show plan status before detailed evidence items;
- show the evidence outcome and reason code without renaming the DC-21/DC-23 vocabulary;
- show displayed/total item counts;
- reuse the DC-23 item rendering rules where possible;
- include a short read-only note;
- avoid wording that implies `ConfluentSubset` means a merge commit was created or can be created by
  this command.

The first implementation may reuse the existing merge-evidence display item model, but the `merge-plan`
top-level output must be distinguishable from `merge-evidence` so tests and users do not confuse a
diagnostic evidence report with a planning decision.

### Exit Status

Process status follows request validity, not plan status:

| Condition | Exit |
|---|---:|
| Valid request and merge plan produced, for any plan status | 0 |
| Invalid CLI arguments, missing selectors, or ambiguous selectors | 1 |
| Selector/ancestry/object/ref failure prevents identifying the requested planning inputs | 1 |
| Unexpected internal error | 1 |

`BlockedConflict`, `BlockedUnsupported`, `BlockedDeferred`, `BlockedEvidenceFailure`, and other
blocked statuses are successful command results when a plan is produced.

The boundary is: if the baseline plus left and right target Blocks are all identifiable, produce a
plan, possibly blocked, and exit `0`; if any planning input cannot be identified, report a CLI error
and exit `1`. Evidence-level failures on identifiable inputs may become `BlockedEvidenceFailure` or
`BlockedInvalidCandidate` plans.

## Store and CLI Boundary

`prikk-store` should own:

- selector validation and resolution;
- explicit-baseline candidate sequence derivation;
- evidence report construction through the existing merge-evidence path;
- mapping evidence outcome/reason into plan status/action;
- producing a structured display model that does not expose raw operation payloads.

The CLI should own argument parsing and text rendering only. It should not inspect internal
patch-algebra types or reconstruct planning semantics from strings.

`prikk merge-evidence` remains the evidence-only diagnostic command. `prikk merge-plan` may reuse the
same underlying evidence construction, but the plan output is a separate UX boundary.

`prikk-replay` remains internally scoped. DC-25 must not move selector resolution, object reading,
lineage walking, patch algebra, lifecycle-cache persistence, or worktree behavior into `prikk-replay`.

## Security and Privacy

DC-25 inherits the DC-21 through DC-23 privacy rules:

- no raw text spans;
- no replacement text;
- no blob bytes or binary payloads;
- no absolute host paths;
- no repository-private `.prikk` paths;
- no signer secrets, seed material, trust private state, or arbitrary key material;
- no arbitrary object debug dumps;
- no panic messages or backtraces in normal error paths.

Displayed paths must be repository-relative. Errors should be precise enough to diagnose invalid
selectors or evidence failures without dumping object payloads.

## Read-only Invariant

For both successful and failing requests, `merge-plan` must not:

- write objects;
- write ref pointers or ref logs;
- write active WAL files;
- write trust policy or trust keys;
- write worktree files;
- acquire publication locks for mutation;
- perform repair, checkout, rollback, or seal operations.

Tests should snapshot relevant repository paths before and after successful and failing invocations.

## Migration and Implementation Plan

### Phase 1 - Store Plan Model

- Add a store-level merge-plan request/target surface parallel to `MergeEvidenceTarget`.
- Reuse explicit-baseline candidate derivation and evidence report construction.
- Add plan-status mapping from evidence outcome.
- Return a structured display model with selector summaries, operation counts, status, evidence
  outcome, reason, item counts, and rendered item inputs.

### Phase 2 - CLI Surface

- Add `prikk merge-plan` argument parsing with the same selector rules as `merge-evidence`.
- Add text rendering that is distinct from `merge-evidence` but reuses safe evidence item summaries.
- Add help text.
- Preserve `merge-evidence` behavior unchanged.

### Phase 3 - Tests

- Add store tests for every evidence outcome to plan-status mapping where fixtures already exist.
- Add CLI tests for `ConfluentSubset` and blocked-conflict output.
- Add argument tests for missing/ambiguous selectors.
- Add read-only success and failure tests covering objects, refs, active WAL, and worktree files.
- Add privacy tests for stdout and stderr.
- Add regression tests proving `merge-evidence` output does not change except where explicitly
  reviewed.

### Phase 4 - Documentation

- Add an mdBook command page for `merge-plan`.
- Cross-link `merge-evidence` and `merge-plan` without implying either command executes a merge.
- Include a built-book link/reachability check for the new `merge-plan` / `merge-evidence`
  cross-links; `mdbook build` alone is not enough to catch broken authority links.
- Update `CHANGELOG.md`, `ROADMAP.md`, and `rfcs/IMPLEMENTATION-STATUS.md` during release
  preparation.

## Resolved Design Review Decisions

Architect design review accepted `merge-plan` as the right first command name for this slice. It
explicitly defers any future `prikk merge --plan-only` alias or replacement to a later DC, after
mutating merge semantics are designed.

The accepted non-executable confluent status is `ConfluentSubset`, not `Clean`, to avoid implying
global merge cleanliness or executability.

DC-25 must not create a current-state FDD-01 / patch-algebra reference. That consolidation belongs to
TASK-08 after DC-26 decides the documentation home. This handoff only records merge-plan facts that
the later TASK-08 reference should include.

Selector, ancestry, object, and ref failures remain CLI errors when they prevent identifying the
baseline or target inputs. Evidence-level failures after inputs are identifiable can be represented as
blocked plans.

## Required Review Gates

Implementation review should include at least:

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
TMPDIR=<workspace-local tmp> cargo test -p prikk --test merge_evidence --quiet
TMPDIR=<workspace-local tmp> cargo test -p prikk --test merge_plan --quiet
cargo test -p prikk-store merge_plan --quiet
TMPDIR=<workspace-local tmp> cargo test --workspace --quiet
mdbook build docs
<built-book link/reachability check for merge-plan and merge-evidence docs>
git diff --check
```

Review requests should include line-count evidence for changed Rust files and confirm test modules
remain outside implementation files.

## Acceptance Criteria

DC-25 was accepted when reviewers agreed that:

- the public planning command name and command shape are appropriate;
- plan status mapping is useful without hiding the evidence report;
- non-goals prevent accidental merge execution or branch semantics;
- output is clear, deterministic, text-only, and privacy-preserving;
- `merge-evidence` remains a diagnostic evidence command;
- store/CLI/replay crate boundaries remain consistent with DC-19 through DC-23;
- implementation and release gates are concrete enough for a future implementation review.

DC-25 is done when the reviewed design is implemented, tests and docs are committed, review accepts
the implementation, and release/status files are up to date in the release commit.
