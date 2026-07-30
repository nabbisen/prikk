# RFC (proposed) - DC-63 Tag Surface

**Status.** **HELD 2026-07-30 — do not implement. Handoff withdrawn.** Accepted 2026-07-30, then held the
same day when implementation found two structural blockers in `prikk-store::refs`, both confirmed
(`.git-exclude/reviewed/prikk-dc63-tag-blockers-ruling-v1.md`):

1. **`publish` rejects every `tags/` name.** `validate_publication` (`refs.rs:352-353`) calls
   `validate_local_branch_ref` unconditionally, which reserves `tags/` outright and then requires `heads/`.
2. **`verify` requires every ref target to be a Block.** `ensure_block_exists` runs unconditionally at
   `refs/verify/scan.rs:65` and `:221`, and `read_typed(id, ObjectType::Block)` hard-errors on a `Tag`
   target — which §6.6 *requires* a tag ref to have.

**Root cause:** `grep RefKind::Tag` across `prikk-store/src` returns **zero production call sites**. Both
validators assume "every ref is a branch pointing directly at a block," an assumption that held by
construction because nothing had ever published or verified a tag.

**Blocked on:** the two fixes in §2 and §3 below being designed and reviewed, plus the constraint in §4.
There is no shippable slice — `tag create` cannot succeed and `tag list` would have nothing to list.

**Independence.** Authored and reviewed by the architect. Design review missed both blockers by checking
that `publish` *exists* rather than that it *accepts a tags/ name*; see §5.

**Requirement.** `specs/prikk-app-requirements-v1.2.md` §6.6 (Tagging).
**Gate.** Product **M1** owns the object model, which is complete. This RFC adds the missing surface. Tag
creation is recorded as deferred at `rfcs/IMPLEMENTATION-STATUS.md:484` and `:557`, so it is a known gap
rather than an oversight.
**Touches.** `prikk-cli` (new `tag` subcommands) and tag ref publication in `prikk-store`.
**No new object type, no payload change, no format change.**

## Problem

§6.6 requires three things:

> - Tags must be immutable tag objects.
> - Moving a tag must create a new ref state or require explicit policy, but must not mutate the original tag object.
> - Tag refs must point to tag objects, not directly to arbitrary mutable data.

**The object model for all three already exists and is identity-pinned.** No command exposes any of it.

## 1. What this requires — existence *and* admissibility

*Revised 2026-07-30. The original table checked whether each mechanism existed. Both blockers passed that
test and failed a question it did not ask: **does the mechanism accept what this RFC intends to put through
it?** Every row now answers both.*

| Needed | Exists? | Accepts a tag? |
|---|---|---|
| Tag object type | Yes — `ObjectType::Tag`, code `0x05` (`vectors/hard.rs:43`) | Yes |
| `TagPayload` | Yes — `payload/tag.rs:9`, canonically encoded | Yes |
| Committed tag identity vector | Yes — one row in `vectors/snapshot.txt` | Yes |
| Format allowlisting | Yes — `format.rs:25` | Yes |
| Persisted directory `tag` | Yes — `persisted_object_types()` | Yes |
| `RefKind::Tag` | Yes — `payload/refs.rs:15` | **Zero production call sites.** Test fixtures only |
| **Ref publication (`publish`)** | Yes — `refs.rs:101` | **NO — Blocker 1.** `validate_publication` rejects `tags/` unconditionally |
| **Ref verification** | Yes — `verify_refs` | **NO — Blocker 2.** Every ref target is assumed to be a Block |
| **A tag ref-name validator** | **NO** — `validate_local_branch_ref` is the only ref-name validator in `prikk-store` and is branch-specific by construction | n/a — must be written, see §3 |

## The timestamp decision — RESOLVED, option A

### `TagPayload.created_at` is documented as authoritative, and this project has ruled it cannot be

`payload/tag.rs:17` reads:

```rust
/// Authoritative creation timestamp.
pub created_at: u64,
```

But DC-34 §"RefUpdate time policy" ruled the opposite for the analogous field:

> For the current schema, `created_at == 0` is the canonical no-clock sentinel. It is not an event-time
> claim. … A real authoritative event timestamp requires a versioned schema and a persistence design that
> retains the exact signed update across interruption.

`RefUpdatePayload.created_at` must be zero in every production write, format-2 verification rejects non-zero,
and `MILESTONES.md:95` tracks it. **There is no trusted clock in this project.**

So a tag cannot carry an authoritative timestamp either — whatever it records is a claim by whoever ran the
command, unverifiable by any reader. The doc comment on `TagPayload` predates DC-34's ruling and contradicts
it.

**Three options, and this RFC must pick one at design review rather than leave it to the implementer:**

| Option | Consequence |
|---|---|
| **A — write zero, as RefUpdate does** | Consistent with DC-34, honest about the absence of clock authority. Costs users a "when was this tagged" answer that the ref log's ordering partly supplies. **Recommended** |
| B — write client-asserted time, documented as untrusted | Gives users a timestamp, but stores an unverifiable claim inside an identity-bearing object, and contradicts the field's own doc comment either way |
| C — design clock authority | Out of scope by an order of magnitude. DC-34 explicitly defers it to "a versioned schema and a persistence design" |

**Decision: A, accepted 2026-07-30.** `tag create` writes `created_at = 0`. `TagPayload`'s doc comment is
corrected to describe it as a no-clock sentinel, matching DC-34's language for `RefUpdate`. Authoritative tag
time follows whenever clock authority does, and not before.

Rationale recorded because the field's name will keep inviting the opposite: a timestamp inside a signed,
content-addressed object reads as attested even when documentation says otherwise, and this project has
already paid once for a field that looked authoritative and was not (`MILESTONES.md:95`, the RefUpdate
timestamp erratum).

**The doc-comment correction must be comment-only.** `Tag`'s type code and a payload row are already pinned
(`vectors/hard.rs:43`, `vectors/snapshot.txt`); touching encoding would move a committed ObjectId.

## Design

### 1. `prikk tag create <name> --target <ref|block> [-m <message>]`

Build a `TagPayload`, persist it as a signed `ObjectType::Tag` object, then publish a tag ref pointing at it
with `RefKind::Tag`.

**Tag refs point at the tag object, never at a block** — §6.6's third clause. The tag object points at the
block. Two hops, deliberately.

Reuse, do not reimplement: `publish` for the CAS publication, `validate_local_branch_ref`'s sibling for tag
ref-name validation if one exists — **and if it does not, that is a finding to report, not a validator to
invent** (the existing one is branch-specific by name).

Fail closed when: `<name>` fails ref-name validation; the tag already exists; `--target` does not resolve.

### 2. `prikk tag list`

Enumerate tag refs and report name plus target block. Deterministic ordering.

**Reuse DC-60's `list_ref_pointers`** (`refs.rs:177`) and filter on `RefKind::Tag`, rather than adding a
second enumerator. DC-61's obligation 3 established that `by-id/` is the complete pointer set and that
`list_ref_pointers` is the only enumerator besides `verify`'s.

### 3. Moving a tag

§6.6: moving "must create a new ref state or require explicit policy, but must not mutate the original tag
object."

**Out of scope for this RFC.** Creation and listing first; moving carries a policy question — whether it is
permitted at all, and under what authority — that deserves its own increment rather than a flag on `create`.
Record it as deferred with that reason, not as an oversight.

### 4. Signing

Tag objects and tag ref states are both maintainer-signed, on the same terms as `seal`
(`signature.rs:49-50`, `Maintainer = 2`). Reuse `maintainer_signer_from_env`; add no signing path.

## 2. Blocker 1's fix — kind-aware validation, centrally

`validate_publication` gains a `RefKind` branch: existing `validate_local_branch_ref` for
`RefKind::Branch`, a new tag validator for `RefKind::Tag`.

**The kind is derivable from the publication.** The implementation report suggested there is "nothing to
branch on," which is the one part of its analysis to correct: `publication.rs:128` already does

```rust
let ref_state = RefStatePayload::decode_canonical(&publication.ref_state.canonical_payload)?;
```

so `RefStatePayload.kind` is available, and `validate_publication` already calls `validate_strict()` on that
same envelope.

**Rejected: moving ref-name validation out of `validate_publication` to its callers.** That converts one
central fail-closed guard into N, and the first caller to forget it gets no error.

`tags/` therefore stops being universally reserved and gains a parallel accepted-name rule. **`remotes/` and
`rollback/` stay reserved** — they have no surface and no validator, and this RFC does not touch them.

## 3. Blocker 2's fix — one extra hop, at both call sites

The target-type check becomes kind-aware: for `RefKind::Branch` the target must be a `Block`; for
`RefKind::Tag` the target must be a `Tag` object whose `target_block_id` is a `Block`.

That is not a workaround — it is what §6.6 specifies. The current code predates any tag existing.

**Both call sites must be handled:** `refs/verify/scan.rs:65` (pointers) and `:221` (ref-log records).
Handling only the first leaves verification inconsistent between pointers and log records, which is worse
than handling neither because it would pass the obvious test.

This is `verify`'s integrity-classification core — the code DC-60's ruling called "the correctness core of
every ref publication." It is specified here rather than left to implementation for that reason.

## 3a. The tag ref-name validator

Written under this RFC, not deferred. Mirror `validate_local_branch_ref` with the prefix requirement
inverted — `tags/` required; `heads/`, `remotes/`, `rollback/` reserved — and **keep its control-character,
traversal, reserved-name, and case-collision rules.**

**Severity note, so the rules are calibrated rather than copied.** Ref names are hashed to filenames via
`ref_name_storage_key` = `to_hex(sha256(ref_name))`, so a hostile ref name is **not** a filesystem-traversal
vector the way a worktree path is. The real exposures are display, log content, and **uniqueness** —
`tags/V1` and `tags/v1` hash differently and would coexist as distinct refs while a user reasonably treats
them as one. That is why the case-collision rule carries over.

A structural-only check (prefix present, suffix non-empty) is **not** acceptable as an interim: it would give
the tag namespace weaker rules than branches for no reason but sequencing.

## 4. Constraint — DC-61 touches the same decode site

**DC-61 threads schema-aware decoding through 10 call sites, one of which is `publication.rs:128`** — the
exact decode §2's fix builds on.

The two increments touch that line for different reasons: DC-61 makes the decode schema-aware for its closed
field; DC-63 makes `validate_publication` branch on the kind that decode yields. They are compatible but not
independent.

**This RFC does not resolve the interaction.** Whichever lands second must reconcile at that site, and its
implementation must state how. DC-61 does *not* touch `validate_publication` or `verify` — its criterion 4
requires them unmodified — so the collision is confined to this one decode call.

Recorded as a constraint rather than resolved because resolving it would mean designing against DC-61's
implementation before it exists, which is the error §5 describes.

## 5. Why design review missed both

Recorded because the fix is a method change, not just a code change.

The original prerequisite table asked "does this mechanism exist." Both `publish` and `verify_refs` exist, so
both rows read "Yes," and the RFC concluded tags were "surface work over proven internals." **The internals
had never been proven for tags** — one grep for `RefKind::Tag` in production code would have shown zero
call sites.

This is the fifth variant of one failure in this program: prerequisites unverified (DC-56 v1, DC-59);
capability already shipped (DC-60 v1); invariants violated by the created state (DC-60, DC-61 v1); cited code
not read closely enough (DC-56 v2); and here, **mechanism confirmed present but never checked for
admissibility.**

§1's table now carries an "Accepts a tag?" column for exactly this reason, and future RFCs reusing existing
machinery should ask the same question of every row.

## Non-goals

- **No tag moving or deletion** — §3 above; each needs its own increment.
- No clock authority. Option C is explicitly excluded.
- No annotated-versus-lightweight distinction. `TagPayload.message` is already optional; that is the whole
  of it.
- No remote or shared tags — §6.11, product M5.
- No new object type, payload field, or format change.
- No change to `branch` subcommands.

## Risks

**`SystemTime::now()` written anyway.** The decision is settled, but the field is *named* `created_at` and —
until the comment is fixed — *documented* as authoritative. An implementer following the type rather than the
RFC will reach for a real clock. Still the single most likely defect here, which is why the handoff leads with
it and why the comment correction is part of the same increment rather than a follow-up.

**Tag ref-name validation reusing branch validation.** `validate_local_branch_ref` is branch-specific by
name. If it encodes `heads/` assumptions, reusing it for `tags/` would either wrongly reject valid tag names
or wrongly accept invalid ones. **Check before reusing**; report if no tag-appropriate validator exists.

**A committed identity vector already exists for `Tag`.** That is helpful — but it means any change to
`TagPayload` encoding, including the doc-comment correction if it touched code, would move a pinned ObjectId.
The doc fix must be comment-only.

## Acceptance criteria

1. **Blocker 1 fixed:** `validate_publication` branches on `RefKind`; a `tags/` publication succeeds and a
   malformed one fails closed. `remotes/` and `rollback/` remain reserved — tested.
2. **Blocker 2 fixed at both call sites** — `scan.rs:65` and `:221`. A well-formed tag verifies clean; a tag
   ref whose target is not a `Tag` object, and a tag object whose `target_block_id` is not a `Block`, both
   fail closed — each tested.
3. **The tag ref-name validator exists** per §3a, including the case-collision rule. A structural-only check
   does not satisfy this.
4. **The DC-61 interaction at `publication.rs:128` is reconciled and the reconciliation stated** — whichever
   increment lands second says how.
5. **The two evidence tests from the blocker report become permanent regression tests**, asserting the
   blockers stay fixed rather than being deleted once they pass.
6. `created_at` is written as **zero** on every tag, and `TagPayload`'s doc comment is corrected — comment only, no encoding change. (The decision itself was discharged at acceptance; this criterion is now its implementation.)
2. `tag create` persists a signed `ObjectType::Tag` object and publishes a `RefKind::Tag` ref **pointing at
   the tag object, not the block**; `verify` passes afterward.
3. `tag create` fails closed on an invalid name, an existing tag, and an unresolvable `--target` — each
   tested against constructed state.
4. Tag ref-name validation is either an existing tag-appropriate validator, or the absence of one is reported
   as a finding rather than worked around.
5. `tag list` enumerates via `list_ref_pointers` filtered on `RefKind::Tag`, with deterministic ordering, and
   adds no second enumerator.
6. **No identity artifact changes**: `vectors/snapshot.txt` — which already contains a `Tag` row — plus
   `vectors/hard.rs`, `state_root/tests/vectors.rs`, `text_span/vectors.rs`, all byte-identical.
7. `verify`, `publish`, and `doctor` unmodified — evidenced by the diff. Tags are an ordinary use of existing
   machinery; needing to change it would mean the design is wrong.
8. Full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9, plus test counts before and after per rule 10.

Criterion 6 is the one to watch: `Tag`'s type code and a payload row are already pinned, so this increment
operates inside an identity contract that already exists rather than establishing one.
