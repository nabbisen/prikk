# Worktree Patch Authoring

`prikk commit` authors the current worktree into a node-addressed patch and appends it to the active
WAL. The patch carries a real role-bound Ed25519 `AUTHOR` signature.

```sh
# Key material is supplied via the environment (a minimal key-input mechanism, not a trust store):
export PRIKK_AUTHOR_KEY_ID="dev-author"
export PRIKK_AUTHOR_SEED="<64 hex chars>"

prikk commit -m "record changes"
# --text-edits prefers full-file text edits for modified UTF-8 tracked files:
prikk commit --text-edits -m "record text changes"
```

`--from-worktree` is still accepted for backward compatibility but is now the only behavior, so it can
be omitted.

## Baseline

Authoring compares the worktree against a baseline node lifecycle state:

- **Published ref:** the baseline is reconstructed from authoritative node-addressed replay of the
  `heads/main` (or `--ref`) lineage — never from a snapshot manifest.
- **Genesis (fresh repository):** when the target ref has never been published, the first commit
  authors against an empty baseline, so every worktree file becomes a `CreateFile`. The following
  `seal` publishes the first block as a Root block. Genesis is scoped to the default `heads/main`.

## Operation mapping

Existing-node kind is authoritative:

- new file → `CreateFile` (fresh CSPRNG-minted `node_id`, normalized mode)
- removed tracked file → `DeleteNode`
- modified text file → `EditText` (full-file span; requires `--text-edits`) or a file-level replacement by default
- modified binary file → `ReplaceBinary`
- permission-only change → `ChangePerm`

Path handling is strict: non-UTF-8 worktree paths fail closed, and traversal/reserved-name/collision
rules apply as elsewhere.

## Out of scope (this stage)

- symlink authoring (fails closed on all symlinks)
- text↔binary kind transitions (fail closed)
- rename detection (a move is a `DeleteNode` + `CreateFile`)
- arbitrary-span text diffs, commutation, conflict witnesses

Signature scope: worktree commits are role-bound Ed25519 `AUTHOR`-signed. This does not imply
trust-store enforcement, key management, `MAINTAINER`/publication signing, or publication-grade
signing for the internal `rollback-draft` scaffold.
