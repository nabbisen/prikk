# Worktree Patch Drafts

PR-025 supports two worktree patch draft modes.

The default mode keeps the PR-019 coarse file-level behavior:

```sh
prikk commit --from-worktree -m "record changes"
```

The opt-in text mode prefers conservative full-file UTF-8 text edits for modified tracked files:

```sh
prikk commit --from-worktree --text-edits -m "record text changes"
```

Both commands compare the current worktree against the snapshot manifest referenced by `heads/main`
or by the ref selected with `--ref`.

Default operation generation:

- missing tracked file -> `DeleteFile`
- modified tracked file -> `ReplaceBinary`
- untracked regular file -> `CreateFile`

Opt-in `--text-edits` generation:

- missing tracked file -> `DeleteFile`
- untracked regular file -> `CreateFile`
- modified tracked UTF-8 file -> full-file `EditText` with `anchor_id = "full-file"`
- modified tracked binary or invalid UTF-8 file -> `ReplaceBinary`

This is intentionally not a minimized text diff. It records the whole file replacement behind the
content-anchor contract introduced in PR-023 and replayed in PR-024.

Deferred work:

- arbitrary span discovery
- minimized text diff generation
- rename detection
- inverse and commutation
- conflict witnesses and merge algebra
