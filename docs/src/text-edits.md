# Content-Anchored Text Edits

PR-024 introduced conservative replay for one narrow `EditText` form. PR-025 adds opt-in worktree
generation for the same shape.

The supported replay/generation shape is a full-file exact-span replacement:

- `anchor_id` must be `full-file`.
- The current full file bytes must be valid UTF-8 during replay.
- `text_span_hash(current_file_bytes)` must equal the recorded `old_span_hash` during replay.
- The whole file is replaced by the UTF-8 `replacement` string.

Generate this form from worktree changes with:

```sh
prikk commit --from-worktree --text-edits -m "record text changes"
```

Generation remains conservative:

- Only modified tracked files are candidates.
- Both baseline and current bytes must be valid UTF-8.
- Binary or invalid UTF-8 modifications fall back to `ReplaceBinary`.
- The default `commit --from-worktree` mode still emits `ReplaceBinary` for modified tracked files.

This deliberately avoids byte offsets and line offsets. Presentation offsets may be derived later,
but they are not part of patch identity or replay preconditions.

Current validation rules:

- `anchor_id` must be non-empty ASCII without whitespace or control characters.
- `old_span_hash` is exactly 32 bytes.
- The span hash is computed by `text_span_hash(bytes)`.

Deferred work:

- arbitrary anchor discovery from real files
- minimized text diff generation
- arbitrary-span replay
- inverse generation
- commutation and conflict witnesses
