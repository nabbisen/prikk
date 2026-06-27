# Content-Anchored Text Edits

PR-024 introduces conservative replay for one narrow `EditText` form.

The supported replay shape is a full-file exact-span replacement:

- `anchor_id` must be `full-file`.
- The current full file bytes must be valid UTF-8.
- `text_span_hash(current_file_bytes)` must equal the recorded `old_span_hash`.
- The whole file is replaced by the UTF-8 `replacement` string.

This deliberately avoids byte offsets and line offsets. Presentation offsets may be derived later, but they are not part of patch identity or replay preconditions.

Current validation rules:

- `anchor_id` must be non-empty ASCII without whitespace or control characters.
- `old_span_hash` is exactly 32 bytes.
- The span hash is computed by `text_span_hash(bytes)`.

Deferred work:

- anchor discovery from real files
- text diff generation
- arbitrary-span replay
- inverse generation
- commutation and conflict witnesses
