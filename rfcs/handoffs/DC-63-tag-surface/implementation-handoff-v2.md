# DC-63 Tag Surface - Handoff v2

**Cleared to start.** Supersedes `implementation-handoff-v1.md`, which is **withdrawn** — do not work from
it. Its premise was that tags were surface work over proven internals; they were not.
**RFC:** `rfcs/accepted/DC-63-TAG-SURFACE.md`, revised at `152d215`.
**Authored by** the architect. Your two blockers were confirmed and both fixes are now specified; a fix
design review then corrected three things in my own specs, so read §§2-4 here rather than working from
memory of the ruling.
**Size:** medium. Two `refs.rs` fixes in the ref system's core, a new validator, then the CLI surface you
already wrote.

## What changed since v1

v1 told you to reuse `publish` and `verify` unmodified, and to stop if either had to change. Both have to
change. You were right to stop; that instruction did its job.

**Preserve and reuse your existing work** — `tag.rs`, `dc63_tag_surface.rs`, and
`TagPayload::decode_canonical` all carry over. The two evidence tests become **permanent regression tests**
(criterion 5), not deleted once they pass.

## Order of work

The `refs.rs` fixes come first. `tag create` cannot succeed until both land, so there is no useful way to
test the CLI before them.

1. §2 — Blocker 1, in `validate_coherent_publication`
2. §3 — Blocker 2, one shared helper for both call sites
3. §4 — the tag ref-name validator
4. Re-enable your `tag create` / `tag list`, now that they can run

## 2. Blocker 1 — kind-aware validation, and the placement is not where the ruling first said

**Put it in `validate_coherent_publication`, after the `:131` name-coherence check. Not in
`validate_publication`.**

`publication.rs:127-135`:

```rust
fn validate_coherent_publication(publication: &RefPublication) -> Result<RefUpdatePayload> {
    validate_publication(publication)?;                    // ← ref-name validation lives here today
    let ref_state = RefStatePayload::decode_canonical(&publication.ref_state.canonical_payload)?;
    let update = RefUpdatePayload::decode_canonical(&publication.ref_update.canonical_payload)?;
    let ref_state_id = publication.ref_state.object_id();
    if ref_state.ref_name != publication.ref_name || update.ref_name != publication.ref_name {
        return Err(PrikkError::Integrity("publication ref names do not agree".to_string()));
    }
```

Two reasons. `validate_publication` runs **before** the decode, so branching there decodes the ref-state
payload twice. And it runs **before names are known to agree** — validating `publication.ref_name` against a
kind read from a payload not yet confirmed to describe the same ref is the wrong order.

So: move ref-name validation out of `validate_publication`, into `validate_coherent_publication` after
`:131`, and branch on `ref_state.kind` — `validate_local_branch_ref` for `RefKind::Branch`, your new tag
validator for `RefKind::Tag`.

**This is safe and I verified it: `validate_publication` has exactly one caller** (`publication.rs:127`) and
`validate_coherent_publication` has exactly one (`publication.rs:35`, in `publish_locked`). You are
relocating a check inside one chain, not dropping it from parallel callers.

**A property to test deliberately, because it is a gain rather than a side effect.** Kind-aware validation
makes namespace and kind **mutually enforcing**:

- `kind = Tag` with `ref_name = heads/main` → rejected
- `kind = Branch` with `ref_name = tags/v1` → rejected

Neither is caught today, because the name is checked and the kind ignored. Criterion 1 requires both
directions tested.

`tags/` stops being universally reserved. **`remotes/` and `rollback/` stay reserved** — they have no surface
and no validator. Do not open them.

## 3. Blocker 2 — one shared helper, both call sites, identical logic

Kind-aware target-type check: `RefKind::Branch` → target must be a `Block`. `RefKind::Tag` → target must be
a `Tag` object whose `target_block_id` is a `Block`.

**Both sites take the identical check, and I can now tell you why** — v1's ruling left this open.
`publication.rs:139` requires:

```rust
update.new_target_object_id != ref_state.target_object_id   // → Err
```

They must be **equal**. So a tag's `new_target_object_id` *is* the Tag object id, the same value the ref
state carries. `refs/verify/scan.rs:221` (ref-log records) is validating the identical value as `:65`
(pointers), so it needs identical logic.

**Implement one shared helper and call it from both.** Not two similar checks. The risk was that handling one
site and missing the other passes the obvious test; a shared helper removes the possibility instead of
relying on care.

This is `verify`'s integrity-classification core — the code DC-60's ruling called "the correctness core of
every ref publication." That is why it is specified rather than left to you, and why criterion 2 requires the
shared helper explicitly, not just both sites passing.

## 4. The tag ref-name validator — mirror the real rules, and add nothing

Your AC4 finding was right: `validate_local_branch_ref` is the only ref-name validator in `prikk-store` and
is branch-specific by construction. Write a tag one.

**Mirror its actual rule set**, prefix inverted — `tags/` required; `heads/`, `remotes/`, `rollback/`
reserved:

- ref name non-empty
- reserved namespaces rejected
- prefix present, non-empty suffix after it
- no `\0`, no control characters
- no leading `/`, no trailing `/`, no `//`
- no `.` or `..` path component

**Do not add a case-collision rule.** The ruling told you to keep one; that was my error — I attributed a
rule to `validate_local_branch_ref` that it does not contain. `heads/Main` and `heads/main` both pass today
and coexist as distinct refs.

Adding it to tags alone would make the tag namespace arbitrarily stricter than branches. The concern is real
but it is **NFR-SEC-03's**, unmet for branches too, and now tracked separately in `MILESTONES.md` covering
both namespaces. Not yours here.

**Severity calibration so you can judge additions:** ref names are hashed to filenames via
`ref_name_storage_key` = `to_hex(sha256(ref_name))`, so a hostile name is **not** a filesystem-traversal
vector the way a worktree path is. The exposures are display, log content, and uniqueness.

`check_tag_ref_name`'s structural-only form is not acceptable as the final state.

## 5. Then the CLI, unchanged from v1's intent

`tag create <name> --target <ref|block> [-m <message>]` — build a `TagPayload`, persist a signed
`ObjectType::Tag` object, publish a `RefKind::Tag` ref pointing at **the tag object** (two hops: ref → tag
object → block, §6.6's third clause). `created_at = 0` always; no clock, no `--date`; `TagPayload`'s doc
comment corrected, **comment-only** since its ObjectId is pinned.

`tag list` — `list_ref_pointers` filtered on `RefKind::Tag`, deterministic order, no second enumerator.

Fail closed on: invalid name, existing tag, unresolvable `--target`.

## 6. One constraint you must reconcile

**DC-61 threads schema-aware decoding through `publication.rs:128`** — the decode immediately above where
§2's kind branch goes. Adjacent lines in the same function.

Whichever of DC-61 and DC-63 lands second reconciles there, and **its submission must state how** (criterion
4). DC-61 does not touch `validate_coherent_publication`'s logic or `verify` — its own criterion 4 requires
those unmodified — so the overlap is confined to that decode call.

## Traps

- **Putting the kind branch in `validate_publication`.** Wrong function; §2.
- **Two similar target checks instead of one shared helper.** §3.
- **Adding a case-collision rule** because the earlier ruling said to. §4.
- **Opening `remotes/` or `rollback/`** while un-reserving `tags/`.
- **`SystemTime::now()`**, or touching `TagPayload`'s encoding while fixing its comment.
- **Pointing the tag ref at the block** instead of the tag object.
- **Deleting your two evidence tests** once they pass. They are criterion 5.

## Submit with

The diff; **explicit confirmation that Blocker 2 uses one shared helper called from both `:65` and `:221`**;
namespace/kind mutual-enforcement tested both directions; the tag validator's rule list, with confirmation
it adds no case-collision rule; the two former evidence tests now passing as regression tests; `created_at`
zero asserted by test; the two-hop resolution tested; confirmation that `vectors/snapshot.txt`,
`vectors/hard.rs`, `state_root/tests/vectors.rs`, and `text_span/vectors.rs` are byte-identical; the DC-61
reconciliation statement; test counts per touched crate before and after; and the full gate set from
`rfcs/EXECUTION-ORDER.md` §6 rule 9 run on a **clean checkout of the commit**, stated as such.

## Standing request, unchanged and now twice-vindicated

v1's version of this request is what produced the two blockers, and the fix design review then found three
errors in my own specs. If something here contradicts what the code actually does, stop and report it.
