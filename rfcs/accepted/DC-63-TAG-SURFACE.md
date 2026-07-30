# RFC (proposed) - DC-63 Tag Surface

**Status.** **Accepted by the project owner on 2026-07-30.** Acceptance carried the timestamp
recommendation — **option A, write the no-clock sentinel** — since the RFC stated it would proceed on A
absent an objection and none was raised. Criterion 1 is thereby discharged; see §"The timestamp decision".
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

## What this requires that does not exist yet

*Mandatory section, per the pattern established across DC-56, DC-59, DC-60, DC-61.*

| Needed | Exists? |
|---|---|
| A tag object type | **Yes** — `ObjectType::Tag`, code `0x05` pinned at `prikk-object/src/vectors/hard.rs:43` |
| A tag payload | **Yes** — `TagPayload` (`payload/tag.rs:9`): `name`, `target_block_id`, `message: Option<String>`, `created_at`, `author_key_id`. Canonically encoded, exported at `payload.rs:26` |
| A committed tag identity vector | **Yes** — one row in `vectors/snapshot.txt`, plus the type-code assertion in `hard.rs` |
| Format allowlisting | **Yes** — `prikk-store/src/format.rs:25` includes `ObjectType::Tag` |
| Persisted storage | **Yes** — `Tag` is in `persisted_object_types()` with directory `tag` |
| A tag ref kind | **Yes** — `RefKind::Tag = 2` (`payload/refs.rs:15`) |
| Ref publication with CAS | **Yes** — `refs.rs:101` `publish` |
| **Any command that creates a tag** | **NO.** This RFC's entire content |

**Unlike DC-60, nothing here already ships a creation path.** I checked `rfcs/done/` — there is no tag RFC,
and `IMPLEMENTATION-STATUS.md:484` lists "tag or remote ref creation" among the not-implemented items. Unlike
DC-61, no format change is needed: the payload exists and is already identity-pinned.

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

1. `created_at` is written as **zero** on every tag, and `TagPayload`'s doc comment is corrected — comment only, no encoding change. (The decision itself was discharged at acceptance; this criterion is now its implementation.)
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
