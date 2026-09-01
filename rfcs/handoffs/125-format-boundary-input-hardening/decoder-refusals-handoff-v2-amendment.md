# Amendment to `decoder-refusals-handoff-v1.md` — §4 stopped one level too shallow

**v1 stands in full. `7bf1279` stays as it is — nothing in it is reverted, and it is pushed with this
amendment.**
**Architect review of `.git-exclude/review-request/decoder-refusals-report-v1.md`, 2026-09-02.**

---

## 1. What held, verified rather than read

**§7 worked exactly as intended, and the result vindicates it.** v1 named two files for §4; you found
**seven**, plus a second decoder inside one of the two named (`RefUpdatePayload`). I re-derived the
list independently — every file in `prikk-object`/`prikk-store` using `next_field()` — and your table
matches mine on all of them.

**The three claims I most expected to be wrong are right:**

- **`(ChangePerm, ChangePerm)` conflicts regardless of value.** Your `write_conflict_fixture` change
  depends on it, and a misreading would have silently weakened a merge-conflict test. Confirmed in
  `classify.rs`: there are arms for `(ChangePerm, ReplaceBinary)` and `(ChangePerm, EditText)` and
  **none** for `(ChangePerm, ChangePerm)`, so it falls to `_ => conflict(UnknownRelation, …)`.
- **The proptest generation change was necessary, not incidental.** With 2 of 2^32 modes canonical,
  leaving `any::<u32>()` would have made the round-trip property skip essentially every case for
  three operation kinds — silently losing the coverage it looks like it still has. Narrowing to
  `canonical_mode_strategy()` and adding the filtered complement as its own property is the right
  fix, and catching it at all is the good part.
- **The cap derivations are sound and honestly bounded.** 255 = `NAME_MAX`, shared across all three
  platforms. 1024 = macOS `PATH_MAX`, the floor of the two totals prikk can guarantee — with
  Windows' legacy 260 explicitly *not* claimed, because it depends on worktree-root depth and cannot
  be bounded from a repository-relative length. **Stating what the cap does not guarantee is the
  part that makes it trustworthy.**

**§5's triage is better than converting all nine.** Each retained `unreachable!` is justified by a
guard in the *same* function or the two lines above it — not fragile-by-distance — and you named them
rather than folding them in. `patch_inverse.rs:442` and `sync_negotiation/summary.rs:225` I checked;
both hold.

**§6's fixture evidence, re-run here:** `release_compatibility_gate::` 5/5 and `format_transition`
3/3 against the tightened code. **No existing history is refused.** Gates: fmt clean, clippy exit 0,
**1462/1462** stable and MSRV, 57/57, boundary and reference `valid: true`.

## 2. Required — §4 was applied one level too shallow

Your enumeration table lists `patch_replay/decode.rs` for `Operation.op_seq`. **It stops at the
wrapper.** `patch_replay/decode/operations.rs` — the file this increment edited to add mode
canonicality — decodes every operation *payload*, and **not one of its seven decoders guards a
singular field**:

| Decoder | Unguarded singular assignments |
|---|---:|
| `decode_edit_text` | 7 |
| `decode_delete_node` | 6 |
| `decode_create_file` | 4 |
| `decode_rename_path` | 3 |
| `decode_change_perm` | 3 |
| `decode_create_symlink` | 3 |
| `decode_replace_binary` | 3 |
| **total** | **29** |

`grep -n seen` over that file returns nothing. Its only two `.is_some()` uses (lines 87, 114) are
`DeleteNode`'s **discriminator** checks — mutually-exclusive field-set validation for the
symlink-vs-file branches — not duplicate-field guards.

So `decode_create_file` reads `4 => mode = Some(field.read_u32()?)` with no guard: **a repeated tag
silently overwrites, and two distinct byte-strings decode to one logical operation** — v1 §4's defect
verbatim, in the largest single cluster of it, in the file you were already editing.

**Not a bypass, and say so in your report rather than overstating it.** The mode check runs after the
loop, so a duplicate cannot smuggle a non-canonical value past it; `RepoPath::parse` likewise runs on
the surviving value. **This is the canonicalization gap, not an escape** — which is exactly what
v1 §4 said the class is, and why it is worth closing anyway in a content-addressed system.

**Do:** the same `seen` guard, same wording, all 29 fields. Repeated-by-design fields stay repeated —
enumerate which those are and say so, as you did for `parent_block_ids`/`patch_ids`.

## 3. Why v1 pointed at the wrong depth, recorded because it generalises

v1 §4 named `payload/block.rs` and `payload/refs.rs` — both in `prikk-object`. **The operation
payload decoders live in `prikk-store`, a different crate**, and I did not look there when writing
the class. §7 told you my lists were floors; **it did not tell you that a class can also be shallow
rather than short**, and that is the shape here: the enumeration was complete for the crate I was
thinking about.

**Enumerate by mechanism, not by crate** — every `next_field()` loop, wherever it lives.

## 4. Controls

1. **The 29 sites fixed**, with the repeated-by-design fields named as a result.
2. **Red before green for at least one per-operation decoder** — a hand-crafted `CreateFile` payload
   with a duplicated tag, accepted at `7bf1279` and refused on your commit. **Show which value won at
   base**, as you did for `BlockPayload`'s `kind`; that is what makes "last-wins" a demonstrated fact
   rather than a reading.
3. **The decoder-totality proptests extended** to the operation decoders, same as you did for the
   payload ones — the new refusals return errors and never panic.
4. **Your own re-enumeration by mechanism** (§3): every `next_field()` loop in the workspace, and
   whether each is now guarded. **If an eighth file exists, name it.**

## 5. Gates

Full set against your final commit, clippy as a single invocation per target with the exit code
captured explicitly. **Re-run `release_compatibility_gate::` and `format_transition`** — this touches
decoders that read committed fixtures, and §6's evidence must still hold after the change.

**No CI control** — that is mine at push time.

One commit on `main`, local, **no push, no tag**. **RFC 125 closes when this lands.**
