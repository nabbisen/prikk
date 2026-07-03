# DC-12 FDD-03 Update - EditText Record Semantics

Status: Accepted; implemented for v0.5.0 candidate
Related RFC: `../../proposed/DC-12-ARBITRARY-SPAN-TEXT-EDITS.md`
Target FDD: FDD-03 Object Schema and Canonical Identity

## Purpose

DC-12 uses the existing FDD-03 `EditText` record for arbitrary spans. No wire fields, object types, tag
numbers, or canonical encodings are added.

## Required FDD-03 Body Updates

### Existing Record Shape

`EditText` remains the node-addressed, span-anchored operation record:

- `node_id`;
- `span_id`;
- `old_span_hash`;
- `left_anchor_hash`;
- `right_anchor_hash`;
- `replacement_text`;
- optional `presentation_hint_line`;
- optional `presentation_hint_column`;
- `old_span_text`.

The optional presentation hints are not preconditions and are not authoritative for replay. DC-12
authoring leaves them absent (`None`) until a UI consumer exists.

### Validation Clarifications

The object-level validator must continue to reject:

- all-zero `node_id`;
- `old_span_hash != text_span_hash(old_span_text)`;
- non-UTF-8 `old_span_text`;
- non-UTF-8 `replacement_text`;
- malformed canonical field layout.

Runtime application adds the node-lifecycle and localization checks owned by FDD-01. The FDD-03 record
does not store byte offsets as authority.

### Compatibility

Full-file text edits are not a distinct wire shape. They are arbitrary-span edits where the selected
old span is the entire old text. Existing full-file fixtures remain valid as long as they satisfy the
same record validators.

## Required Tests

- existing `EditText` byte-layout vectors remain unchanged;
- full-file `EditText` fixtures decode as ordinary arbitrary-span records;
- empty `old_span_text` and empty `replacement_text` are valid when their hashes and UTF-8 constraints
  are satisfied;
- presentation hints do not affect object identity or replay authority.
