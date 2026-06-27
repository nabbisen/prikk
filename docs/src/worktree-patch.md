# Worktree Patch Drafts

PR-019 introduces a minimal patch-draft command:

```sh
prikk commit --from-worktree -m "record changes"
```

The command compares the current worktree against the snapshot manifest referenced by `heads/main`
(or another ref in future extensions). It creates a signed Patch envelope and appends it to the
active WAL.

This is intentionally file-level only:

- missing tracked file -> `DeleteFile`
- modified tracked file -> `ReplaceBinary`
- untracked regular file -> `CreateFile`

Rename detection, content-anchored text-span editing, patch replay, and merge algebra are later
increments.
