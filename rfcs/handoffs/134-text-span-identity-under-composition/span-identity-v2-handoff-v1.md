# RFC 134 — content-unique span identity, Patch schema 3

**Authority:** `rfcs/done/134-text-span-identity-under-composition.md` **§8**, option (a),
**authorized by the project owner 2026-09-04**.
**Base:** current `main`. **Under `003-landing-work-on-main.md`.**

**This adds a schema version to signed history.** Read §8.5 before writing code: the constraint that
matters most is what must *not* change.

---

## 1. What you are building

`EditText` currently disambiguates textually- and anchor-identical spans by **`dup_index`** — the
span's position in a candidate list **rescanned from whatever buffer is in front of it**. Identity
therefore shifts when an earlier edit removes a sibling occurrence.

**v2 deletes `dup_index` and guarantees uniqueness at authoring instead.**

| | v1 (frozen) | v2 (new) |
|---|---|---|
| Anchors | fixed 64 bytes | recorded lengths, ≥ 64 |
| Identity | includes `dup_index` | **no `dup_index`** |
| Disambiguation | positional, at replay | **uniqueness, at authoring** |

## 2. The mechanism, precisely

**Schema.** `admitted_schemas(ObjectType::Patch)` returns `&[1, PATCH_PARENT_IDS_RETIRED_SCHEMA]` with
that constant `= 2` (`prikk-object/src/payload/patch.rs:59`). **Mint schema 3** and admit it alongside.

**Fields.** `EditText` gains **tag 10 `left_anchor_len`** and **tag 11 `right_anchor_len`**, `u32`,
**optional** — written only at schema 3, exactly as tags 7 and 8 are already conditionally written
(`payload/patch/operations.rs:221-233`). **Absent below schema 3, so no existing object's bytes move.**

**Identity.**

```
PRIKK-TEXT-SPAN-v2 ‖ node_id ‖ old_span_hash ‖ left_anchor ‖ right_anchor ‖ left_len ‖ right_len
```

with `PRIKK-TEXT-LEFT-ANCHOR-v2` / `PRIKK-TEXT-RIGHT-ANCHOR-v2` over exactly the recorded lengths.
**Choose the byte encoding of the lengths in the preimage deliberately and say what you chose** —
big-endian `u32` matches `compute_span_id`'s existing `dup_index.to_be_bytes()`.

**Authoring.** Smallest lengths ≥ 64 making the span unique among occurrences of `old_span_text`.
**This always succeeds for a finite file** — extending left eventually reaches the file start and
distinct positions have distinct prefixes. **If you find an input where it does not, stop and report
it: that would refute §8.3 and the design needs revisiting, not a workaround.**

**Resolution.** Occurrences of `old_span_text`, filtered by anchors at the *recorded* lengths, **require
exactly one**, recompute and compare. **v1 resolution is untouched and stays forever.**

## 3. What must not change — the acceptance criteria that matter

**3.1 Every v1 object's bytes, id, and resolution.** RFC 114: *keep every version ever written
decodable, forever, and keep its bytes hashing the way they did.* The frozen identity vectors
(`prikk-object/src/vectors/hard.rs`) must pass **unmodified**. If a vector needs editing, you have
broken the contract, not the vector.

**3.2 Transport and the repository-format gate must not move.** RFC 114 records that DC-53 Stage 2's
`PBNDL001` → `PBNDL002` bump severed the bundle migration path and left every repository below format
6 **unmigratable**. **Demonstrate, do not assume**: build a repository with v1 `EditText` history,
`bundle export` it, `bundle import` it with your build, and verify. Report the transcript.

**3.3 `compute_state_root` inputs.** It hashes the resulting blob, not spans. Nothing here should reach
it.

**3.4 Mixed history replays.** A single node with a v1 `EditText` and a later v2 `EditText` must
replay. Build that case explicitly — it is the one most likely to be missed.

## 4. Two traps

**The Property B allowlist entry stays.** `algebra_properties.rs`'s
`ALLOWLISTED_EVIDENCE_ERROR_REASONS` and its persisted seed are **not** to be removed by this
increment. The generator still builds v1-shaped operations; the entry retires only when the generator
moves to v2 and the case passes *for the right reason*. **Removing it because v2 landed would turn a
recorded finding into a hidden one.**

**`locate_text_span` has nine production call sites** — `patch_replay/apply.rs`,
`lifecycle_cache/replay/effect.rs`, four in `patch_algebra/`, `text_span/authoring.rs`,
`text_span/inverse.rs`. **The algebra oracle (`replay_oracle.rs:231`) and real materialization
(`patch_replay/apply.rs:272`) call it identically by design** — that sameness is what makes the
oracle's prediction sound. **Whatever dispatch you introduce must be shared by both**, or the algebra
will predict something materialization does not do.

## 5. Report

Full gate set from `rfcs/EXECUTION-ORDER.md` §6 rule 9 against your final commit — **not reproduced
here**. State:

1. The length encoding you chose for the preimage, and why.
2. The §3.2 export/import transcript, from a repository containing v1 `EditText` history.
3. The §3.4 mixed-history replay case.
4. Confirmation that `vectors/hard.rs` is unmodified.
5. Any input where authoring could not achieve uniqueness (§2).
6. Every place this handoff's claims proved wrong. **The field tags, the schema constant, and the
   nine call sites are mine.**

Local commits on `main`; **no push, no tag, no publish.**
