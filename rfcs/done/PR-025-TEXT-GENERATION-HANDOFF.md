# PR-025 Implementation Handoff — Opt-In Full-File Text Edit Generation

## Summary

PR-025 connects the PR-023/PR-024 content-anchored text-edit scaffold to worktree patch draft
generation. The existing default `prikk commit --from-worktree -m <message>` behavior remains
compatible and continues to emit coarse file-level operations. A new opt-in mode:

```sh
prikk commit --from-worktree --text-edits -m <message>
```

emits full-file `EditText` operations for modified tracked files only when both baseline and
current worktree bytes are valid UTF-8.

## Added API

- `WorktreePatchCommitOptions`
- `WorktreePatchCommitOptions::file_level()`
- `WorktreePatchCommitOptions::prefer_text_edits()`
- `commit_worktree_changes_with_options()`
- `WorktreePatchCommitReport::text_edit_count`
- `WorktreePatchOperationKind::EditText`

The existing `commit_worktree_changes()` remains available and uses `file_level()` behavior.

## CLI Behavior

Default mode:

```sh
prikk commit --from-worktree -m "record changes"
```

- `Missing` -> `DeleteFile`
- `Modified` -> `ReplaceBinary`
- `Untracked` -> `CreateFile`

Opt-in text mode:

```sh
prikk commit --from-worktree --text-edits -m "record text changes"
```

- `Missing` -> `DeleteFile`
- `Untracked` -> `CreateFile`
- `Modified` UTF-8 tracked file -> full-file `EditText`
- `Modified` binary or invalid UTF-8 tracked file -> `ReplaceBinary`

`--text-edits` is rejected unless `--from-worktree` is also selected.

## Safety Boundary

PR-025 does not implement arbitrary-span text diffs. It records a whole-file replacement behind the
existing `anchor_id = "full-file"` replay contract:

- `old_span_hash = text_span_hash(old_full_file_bytes)`
- `replacement = new_full_file_text`

This keeps generation and replay aligned while avoiding offset-based patch identity.

## Tests Added

- text mode emits `EditText` for UTF-8 modified tracked files;
- text mode falls back to `ReplaceBinary` for invalid UTF-8/binary modified tracked files;
- existing default worktree patch tests remain unchanged.

## Deferred Work

- arbitrary span discovery;
- minimized text diff generation;
- inverse generation;
- commutation and conflict witnesses;
- merge state;
- audit plugins and sync.
