# DC-63 Tag Surface - Handoff

> # ⛔ WITHDRAWN 2026-07-30 — DO NOT WORK FROM THIS DOCUMENT
>
> Two structural blockers in `prikk-store::refs` make this unimplementable as written, both confirmed:
> `publish` rejects every `tags/` name (`validate_publication` → `validate_local_branch_ref`), and `verify`
> requires every ref target to be a `Block` (`ensure_block_exists` at `scan.rs:65` and `:221`) — which a tag
> ref cannot have, since §6.6 requires it to target a tag object.
>
> Found by the dev team following this document's own standing request: *"if something here contradicts a
> shipped RFC, an accepted requirement, or an existing identity artifact, stop and report it."* It did, they
> did, and it was right. **No shippable slice exists.**
>
> See `rfcs/accepted/DC-63-TAG-SURFACE.md` §§1-5 and
> `.git-exclude/reviewed/prikk-dc63-tag-blockers-ruling-v1.md`.
>
> **Superseded by `implementation-handoff-v2.md`, which is cleared to start.** Work from that.
>
> **Preserve `tag.rs`, the tests, and `TagPayload::decode_canonical`** — all of it carries over once the two
> fixes land. Everything below is retained as the record of what was asked for and is **not** current
> instruction.

---

**Cleared to start.** Accepted by the project owner on 2026-07-30, at
`rfcs/accepted/DC-63-TAG-SURFACE.md`. The one open design question — tag timestamps — was decided at
acceptance; see §1. No gate remains.
**Authored by** the architect.
**Size:** small-to-medium. Two CLI subcommands over an object model that already exists.
**Touches:** `prikk-cli` (new `tag` subcommands), tag ref publication in `prikk-store`, and one doc comment
in `prikk-object`. **No new object type, no payload change, no format change.**

## What this is

`prikk tag create` and `prikk tag list`, closing requirements §6.6.

**The object model is already built and already identity-pinned.** You are adding a surface, not a format:

| Piece | Where |
|---|---|
| `TagPayload` — `name`, `target_block_id`, `message: Option<String>`, `created_at`, `author_key_id` | `prikk-object/src/payload/tag.rs:9`, canonically encoded, exported at `payload.rs:26` |
| `ObjectType::Tag`, code `0x05` | pinned at `prikk-object/src/vectors/hard.rs:43` |
| A committed tag identity row | `prikk-object/src/vectors/snapshot.txt` |
| Format allowlisting | `prikk-store/src/format.rs:25` |
| Persisted directory `tag` | `persisted_object_types()` |
| `RefKind::Tag = 2` | `prikk-object/src/payload/refs.rs:15` |
| CAS ref publication | `refs.rs:101` `publish` |

## 1. Read this first: `created_at` must be zero

`TagPayload.created_at` is currently documented as an "Authoritative creation timestamp."
**That documentation is wrong and you are also fixing it.**

DC-34 §"RefUpdate time policy" ruled: `created_at == 0` is "the canonical no-clock sentinel… not an
event-time claim," and a real authoritative timestamp "requires a versioned schema and a persistence design."
`RefUpdatePayload.created_at` must be zero in every production write, and format-2 verification rejects
non-zero. **This project has no trusted clock.**

So:

- **Write `created_at = 0` on every tag.** Do not call `SystemTime::now()`, do not accept a `--date` flag.
- **Correct `TagPayload`'s doc comment** to describe it as a no-clock sentinel, matching DC-34's language for
  `RefUpdate`.

**The comment fix must be comment-only.** `Tag`'s type code and a payload row are already pinned in
`vectors/hard.rs` and `vectors/snapshot.txt`. Touching encoding would move a committed ObjectId — the DC-55
class of defect.

This is called out first because the field is *named* `created_at` and, until you fix the comment, *documented*
as authoritative. Following the type instead of this handoff is the most likely way to get this increment
wrong.

## 2. `prikk tag create <name> --target <ref|block> [-m <message>]`

Build a `TagPayload`, persist it as a signed `ObjectType::Tag` object, then publish a tag ref pointing at
**the tag object**, with `RefKind::Tag`.

**The ref points at the tag object, never directly at a block.** That is §6.6's third clause — "tag refs must
point to tag objects, not directly to arbitrary mutable data." Two hops: ref → tag object → block. Getting
this backwards satisfies nothing and looks like it works.

Reuse, do not reimplement:

- `refs.rs:101` `publish` for the CAS publication
- `maintainer_signer_from_env` for signing. Tag objects and tag ref states are both maintainer-signed —
  `signature.rs:49-50`, `Maintainer = 2`, "publishing/sealing a block or ref state". Add no signing path

**Fail closed when:** the name fails ref-name validation; the tag already exists; `--target` does not resolve.

### Ref-name validation — check before reusing, and report if nothing fits

`validate_local_branch_ref` is **branch-specific by name** and may encode `heads/` assumptions. Reusing it for
`tags/` could wrongly reject valid tag names or wrongly accept invalid ones.

**Check what it actually does. If there is no tag-appropriate validator, report that as a finding rather than
inventing one.** Ref-name validation is path-safety-adjacent — NFR-SEC-03 covers absolute paths, `..`, reserved
names, symlink escape, and case collisions — so a new validator written casually is a security surface, not a
convenience. That is acceptance criterion 4, and "there isn't one" is a valid answer to it.

## 3. `prikk tag list`

Enumerate tag refs, report name and target block, deterministic ordering.

**Reuse DC-60's `list_ref_pointers`** (`refs.rs:177`) filtered on `RefKind::Tag`. Do **not** add a second
enumerator. DC-61's obligation 3 established that `by-id/` is the complete pointer set and that
`list_ref_pointers` is the only enumerator besides `verify`'s `read_pointers` — every other path resolves refs
by name.

If DC-61 lands first and adds closed-ref filtering there, coordinate rather than duplicating the filter logic.

## 4. Not in scope

- **No tag moving and no tag deletion.** §6.6 permits moving "or require explicit policy" — that policy
  question gets its own increment. Do not add a `--force` or `--move` flag.
- No annotated-versus-lightweight distinction. `message` is already `Option<String>`; that is the whole of it.
- No remote or shared tags — §6.11, product M5.
- No clock authority, no `--date`.
- No changes to `branch` subcommands.

## Traps

- **`SystemTime::now()`.** Covered in §1; the most likely defect.
- **Changing `TagPayload`'s encoding** while fixing its comment. A pinned ObjectId would move.
- **Pointing the tag ref at the block** instead of the tag object.
- **Inventing a tag ref-name validator** instead of reporting that none exists.
- **Adding a second ref enumerator** rather than filtering `list_ref_pointers`.
- **Needing to change `verify`, `publish`, or `doctor`.** Tags are an ordinary use of existing machinery. If
  one of them must change, the design is wrong — stop and report.

## Definition of done

`tag create` persists a signed `ObjectType::Tag` object with `created_at = 0` and publishes a `RefKind::Tag`
ref pointing at that object; the three fail-closed conditions hold; ref-name validation either reuses a
tag-appropriate validator or its absence is reported; `tag list` enumerates via `list_ref_pointers` filtered on
`RefKind::Tag` with deterministic ordering; `TagPayload`'s doc comment corrected, comment-only; `verify`,
`publish`, and `doctor` unmodified.

## Submit with

The diff; confirmation that **`created_at` is zero on every written tag**, asserted by test rather than by
inspection; confirmation that the tag ref resolves to a tag object and the tag object to the block, tested as
two hops; test results for each fail-closed condition constructed as real state; the ref-name validation
finding if there is one; confirmation that `vectors/snapshot.txt`, `vectors/hard.rs`,
`state_root/tests/vectors.rs`, and `text_span/vectors.rs` are **byte-identical** — this matters more than
usual here, since `Tag` already has pinned identity artifacts; explicit confirmation that `verify`, `publish`,
and `doctor` are unmodified; test counts per touched crate before and after; and the full gate set from
`rfcs/EXECUTION-ORDER.md` §6 rule 9, run on a **clean checkout of the commit** and stated as such.

## Standing request

Three increments in this program were redesigned or scoped down because implementation found something design
review missed — DC-57, DC-60, and DC-61. Each report was worth more than the code would have been. If
something here contradicts a shipped RFC, an accepted requirement, or an existing identity artifact, stop and
report it.
