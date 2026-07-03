# DC-13 FDD-03 Update - Branch Ref Identity Clarification

Status: Accepted; implemented for v0.6.0 candidate
Related RFC: `../../proposed/DC-13-NONDEFAULT-REF-GENESIS.md`
Target FDD: FDD-03 Object Schema and Canonical Identity

## Purpose

DC-13 creates non-default branch refs by publishing ordinary Root blocks and ordinary signed
`RefState` / `RefUpdate` payloads. No object type, canonical tag, operation record, or block kind is
added.

## Required FDD-03 Body Updates

### Existing Record Shapes

Unborn branch publication uses the existing payload fields:

- `Block.kind = Root`;
- `Block.parent_block_ids = []`;
- `RefState.ref_name = "heads/<branch>"`;
- `RefState.kind = Branch`;
- `RefState.previous_ref_state_id = None`;
- `RefState.update_seq = 1`;
- `RefUpdate.ref_name = "heads/<branch>"`;
- `RefUpdate.old_ref_state_id = None`;
- `RefUpdate.update_seq = 1`.

The selected ref name remains identity-bearing for `RefState` and `RefUpdate`. The same first commit
published to `heads/main` and `heads/topic` therefore produces different ref-state and ref-update
object identities, while Patch and Block identity follow their existing payload rules.

### Ref-Name Validation

DC-13 requires a command-level local branch policy before authoring or publication. This validator
returns the canonical ref identity string used by command logic, active-WAL metadata, `RefState`, and
`RefUpdate`:

- ref names must be UTF-8 strings;
- DC-13 genesis accepts only `heads/<name>`;
- empty names, empty branch components, traversal segments, duplicate separators, leading/trailing
  separators after `heads/`, NUL, and control characters fail closed;
- `tags/`, `remotes/`, and `rollback/` namespaces are reserved for later designs.

No lossy conversion or Unicode normalization is performed. Ref names remain exact byte-preserving UTF-8
strings after validation. This validation is command/policy validation around existing identity fields.
It does not change canonical encoding, and object decoding of historical `RefState` / `RefUpdate`
payloads remains compatibility-aware unless a later schema design declares invalid historical ref names
unreadable.

The same validator must validate active-WAL ref metadata before comparison with the requested ref.
Comparison is byte-exact between canonical validated ref strings. Invalid metadata with a non-empty WAL
is an active-session integrity failure; invalid metadata with an empty WAL is local session debris and
is cleaned under the active lock.

## Required Tests

- byte-level existing RefState and RefUpdate vectors remain unchanged;
- non-default branch genesis encodes as ordinary Branch RefState and RefUpdate payloads;
- invalid branch ref names fail before object creation;
- Patch operation records are unchanged by the selected ref name.
