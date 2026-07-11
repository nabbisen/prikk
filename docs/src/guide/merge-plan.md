# Merge Plan

DC-25 (0.17.0) adds `prikk merge-plan`, a read-only planning classification over the
existing [merge evidence](merge-evidence.md) report. It answers what Prikk can say about the selected
explicit inputs today; it does not execute or prepare a merge commit.

```sh
prikk merge-plan \
  --baseline-block BLOCK \
  (--left-block BLOCK | --left-ref REF) \
  (--right-block BLOCK | --right-ref REF) \
  [path]
```

Selector rules:

- `--baseline-block` is required and names the sealed baseline block.
- Each side must choose exactly one selector: `--left-block` or `--left-ref`, and `--right-block`
  or `--right-ref`.
- Ref selectors resolve only through the current local branch target block.
- The optional positional argument is the repository root. It is not a path filter.

The command is read-only. It does not infer merge bases, execute merges, publish merge commits, write
objects, refs or WAL records, materialize worktree conflicts, or persist proof/witness/plan objects.

## Output

Output is text-only and intended for human planning diagnostics. It is not a durable
machine-readable schema.

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

Plan status maps the underlying evidence outcome to a non-executable classification:

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

`ConfluentSubset` means the selected candidates are proven confluent only for the currently supported
operation subset. It is not a whole-merge guarantee and does not mean Prikk can create a merge commit.

## Exit Status

| Condition | Exit |
|---|---:|
| Valid request and a merge plan was produced, for any plan status | 0 |
| Invalid CLI arguments, missing selectors, or ambiguous selectors | 1 |
| Selector, ancestry, object, or ref failure prevents identifying the requested inputs | 1 |
| Unexpected internal error | 1 |

Process success is independent of plan status: a produced `BlockedConflict` plan exits `0`.

## Deferred

- `prikk merge`, merge execution, and conflict resolution;
- automatic merge-base discovery;
- branch merge semantics, branch publication, merge commits, and multi-parent blocks;
- active-WAL merge drafts and worktree conflict materialization;
- display-path filtering and scoped/path-limited merge analysis;
- persisted proof/witness/merge-plan objects;
- JSON or other machine-readable output;
- public `prikk-replay` API stabilization.
