# Rollback Draft

PR-028 adds an explicit mutating rollback-draft command for the supported patch-operation subset.

The command is:

```sh
prikk rollback-draft --append-inverse [path] [--ref REF] -m "rollback message"
```

The command performs the same supported inverse validation used by `inverse-plan` and
`rollback-preview`, then appends a signed inverse Patch envelope to the active WAL.

Safety rules:

- `--append-inverse` is required.
- `-m <message>` is required and must not be empty.
- the target ref must be published and must resolve to a supported single-parent block chain.
- the supported replay/inverse subset must validate successfully.
- the active WAL must be empty.
- the active WAL must not contain a trailing partial record.

What it mutates:

- appends one signed inverse Patch envelope to `.prikk/active/default/queue.wal`.

What it does not mutate:

- it does not write object files directly.
- it does not publish refs.
- it does not modify the worktree.
- it does not authorize rollback by policy.

PR-029 adds a pre-seal verification command:

```sh
prikk rollback-draft-verify [path] [--ref REF]
```

After reviewing and verifying the draft, the existing local seal scaffold can publish it:

```sh
prikk seal --allow-no-audit
```

PR-030 keeps that seal path unchanged, but `prikk log` and `prikk verify` now classify the sealed Block as a rollback Block when it contains rollback-marked Patch objects.

Supported inverse operation subset:

- `CreateFile` -> `DeleteFile`
- `DeleteFile` -> `CreateFile`
- `ReplaceBinary` -> swapped `ReplaceBinary`
- full-file `EditText` -> full-file inverse `EditText`

Deferred:

- rollback-specific ref publication policy
- authorization and audit policy for rollback
- rollback branch/reflog semantics
- worktree rollback materialization policy
- arbitrary-span text rollback
- commutation, confluence, and conflict witnesses
- plugin execution and remote sync
