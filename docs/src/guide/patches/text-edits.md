# Content-Anchored Text Edits

DC-12 supports deterministic arbitrary-span `EditText` generation and replay for existing text-file
nodes. A modified text file is represented as one enclosing span selected by byte LCP/LCS and widened
to UTF-8 character boundaries. The record remains the FDD-03 node-addressed, span-anchored `EditText`
shape; no offsets or new identity fields are added.

Author text edits from worktree changes with:

```sh
prikk commit --from-worktree --text-edits -m "record text changes"
```

`--text-edits` is retained for compatibility. Existing-node kind is authoritative, so text-file
modifications author `EditText` and binary-file modifications author `ReplaceBinary`.

Generation and replay remain conservative:

- Only modified tracked files are candidates.
- Both baseline and current bytes must be valid UTF-8.
- Text edits use a single deterministic enclosing span; multi-operation diff minimization is deferred.
- Byte-level differences that split a multibyte character are widened to the enclosing UTF-8 character.
- Binary or invalid UTF-8 modifications fail closed for text nodes; they do not become `ReplaceBinary`.
- Replay localizes by `old_span_text`, left/right anchor hashes, and `span_id`, then splices exactly.

This deliberately avoids byte offsets and line offsets. Presentation offsets may be derived later,
but they are not part of patch identity or replay preconditions.

Current validation rules:

- `old_span_hash` is exactly 32 bytes.
- `old_span_hash` must equal `text_span_hash(old_span_text)`.
- `old_span_text` and `replacement_text` must be well-formed UTF-8.
- The target `node_id` must name a live `TextFile` during replay.

Deferred work:

- multi-operation text diff minimization
- direct inverse and rollback extension for arbitrary spans
- commutation and conflict witnesses
