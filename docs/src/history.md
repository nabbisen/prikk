# History Inspection

PR-014 added a small read-only history view for early sealed repositories. PR-030 extends that view with rollback block classification.

```sh
prikk log [path] [--limit N] [--ref REF]
```

The command follows the current `RefState` chain from newest to oldest and validates that each entry targets a persisted Block object.

For each entry, the CLI reports:

- the target Block ID;
- RefState ID and update sequence;
- Block kind;
- parent and Patch counts;
- rollback block classification;
- rollback-marked Patch count;
- required attestation count;
- previous RefState ID.

A Block is classified as a rollback Block when it references at least one Patch envelope carrying the current development rollback marker and that Patch payload decodes under the supported replay subset.

History inspection does not yet perform full block-DAG traversal, path-aware history queries, rollback authorization checks, or patch algebra.
