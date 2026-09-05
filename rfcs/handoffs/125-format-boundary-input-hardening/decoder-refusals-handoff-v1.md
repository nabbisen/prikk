# Four places the decoder accepts what the encoder would never write — implementation handoff

**Authority:** `rfcs/done/125-format-boundary-input-hardening.md`.
**Base:** current `main` (`679a884`). **Under `003-landing-work-on-main.md`.**

**One increment, four refusals.** They share a governing rule (DC-54's encode/decode symmetry), a
subsystem, a test strategy, and **one compatibility question that must be answered once for all four**
(§6). Splitting them would re-argue the same principle four times.

---

## 1. The shape

Each site lets the decoder be more permissive than the encoder, and in each case what saves the
system today is a guard *somewhere else*. **None is exploitable now. All four are one refactor away
from mattering**, and the audit's phrase for the first is the right frame for all of them:
*fragile-by-distance.*

## 2. Modes: any `u32` decodes, only two are ever written — **Medium**

`patch_replay/decode/operations.rs` reads `mode` as a bare `u32`
(`4 => mode = Some(field.read_u32()?)` for `CreateFile`; `ChangePerm` carries `old_mode` at tag 6 and
its new mode alongside). Materialization applies it: `worktree.rs` →
`fsutil/anchored/linux.rs:114`, `fchmod(mode & 0o7777)` — **which admits setuid, setgid and sticky.**

**What saves it today** is `state_root.rs:163-180`:

```rust
(NodeKind::TextFile | NodeKind::BinaryFile, StateRootContent::Blob(_), REGULAR_MODE | EXECUTABLE_MODE) => Ok(()),
(NodeKind::Symlink, StateRootContent::Symlink(_), 0) => Ok(()),
_ => Err(...)
```

— a guard at **seal** time, in a different subsystem, for a different purpose; plus the fact that
received refs cannot be materialized (`patch_replay.rs` refuses `remotes/`). **Two plausible future
changes re-open it independently, and neither would look like a security change to whoever made it.**

**Do:** refuse a non-canonical mode at operation decode, and/or normalize at
`set_regular_file_mode_required`. **Enumerate every mode-carrying field yourself** — `CreateFile`,
`ChangePerm`'s old and new, and anything else the operation set has grown. **Do not trust the two I
named**; see §7.

## 3. No length caps in the repo-path grammar — **Medium**

`validate_repo_path` (`prikk-object/src/path.rs:27-114`) is thorough about *shape* — absolute paths,
backslashes, colons, control characters, `.`/`..`, trailing dot or space, Windows device names,
`.prikk` as first component — and contains **no length check of any kind** (confirmed: no `MAX`,
no `255`, no `4096` anywhere in the file).

So a 300-byte component enters signed history and then **cannot be materialized** — `NAME_MAX` 255
on Linux, 260 by default on Windows — surfacing as a raw OS error at checkout on a repository that
verifies clean. **An availability defect that arrives long after the commit that caused it.**

**Do:** cap component and total path length in `validate_repo_path`, so the refusal happens at
authoring and — by DC-54 symmetry — on receipt too. Record the numbers in
`docs/src/reference/path-safety.md`, which owns this grammar. **State why each number is what it is**;
a cap with no derivation is a magic constant the next person will "fix".

## 4. Duplicate singular TLV fields decode last-wins — **Low**

`payload/block.rs:78-96` assigns singular fields (`state_merkle_root`, `snapshot_blob_ref`,
`mainline_parent_id`, `merge_baseline_block_id`, `kind`) with `= Some(...)` inside the field loop, so
a repeated tag silently overwrites: **two distinct byte-strings decode to one logical value.**
`payload/refs.rs:78-108,203-217` has the same shape.

**The strict precedent is in the same crate**, twelve lines away:
`PatchPurpose::decode_from_patch_payload` (`payload/patch.rs:150-160`) keeps a `seen_purpose` flag and
returns `"duplicate PatchPurpose field"`. It was simply never applied uniformly.

Identity covers the raw bytes, so this is not a substitution attack — it is a **canonicalization gap,
and those are the class worth closing early in a content-addressed system.**

**Do:** `seen` flags for singular fields in every decoder, matching that precedent's shape and error
wording.

## 5. Two `unreachable!`s on paths fed by external bytes — **Low**

- `patch_replay/decode.rs:290` — `unreachable!("kind tag is constrained to 10..=16 above")`
- `verify/objects.rs:207` — `unreachable!("record_outcomes only ever holds Evaluated or Failed")`

Both invariants hold today, and both are maintained by an **adjacent** match arm rather than by the
site relying on them — the same fragile-by-distance shape as §2.

**Do:** return `MalformedData`. **A panic on attacker-influenced bytes is a denial-of-service shape
even when the invariant is currently true**, and this workspace's own standard is that external input
never panics — the decoder-totality proptests exist to assert exactly that.

## 6. The one question to answer once, with fixtures

**Can tightening a decoder refuse history that already exists?**

§2's answer is *"no, because the seal boundary has always refused non-canonical modes"* and §3's is
the weaker *"no, because nobody has authored such a path"*. **Both are arguments, and this project
does not ship arguments where it can ship evidence.**

**Verify against real committed fixtures**, not by reasoning: the `format_transition` tests already
run against real committed repository fixtures, and the G1 fixture exists. Show that the tightened
decoder still reads them. **If any existing fixture would be refused, stop and report it** — that
turns a hardening increment into a format-compatibility decision, which is not yours or mine to take
inside this handoff.

## 7. A standing instruction, because I have been wrong about this three times this week

**Every site list in this handoff is a floor, not a ceiling.** My "three sites" was two in the DC-44
sweep; my "five sites" was six in RFC 122's; my "every released tag" was eight of forty-four in
RFC 127's. **Enumerate each class yourself and report what you searched**, and if you find a fifth
decoder with last-wins singular fields or a third `unreachable!` on a byte-fed path, **that is the
finding — name it rather than folding it in silently.**

## 8. Constraint

**Every tightening must be encode/decode symmetric** (DC-54). A refusal added only at decode would
let this project author history it then refuses to read — the exact inversion of the defect being
fixed. `patch_replay/tests/dc54_encode_decode_symmetry.rs` is where that property is already tested.

## 9. Controls

1. **Each of the four refusals demonstrated red before green** — a crafted input that the decoder
   accepts at base and refuses on your commit, per class.
2. **§6's fixture evidence**, stated as a result: which fixtures, and that each still decodes.
3. **The decoder-totality proptests extended** to cover the new refusals
   (`payload/tests/proptest_decoders.rs`, `patch_replay/tests/proptest_round_trip.rs`) — totality
   must still hold: the new paths return errors, never panic.
4. **§7's enumeration**, as a result: what you searched for each of the four classes and what you
   found.
5. **`path-safety.md` updated** with the caps and their derivation; `mdbook build` clean.

## 10. Gates

Full set from `EXECUTION-ORDER.md` §6 rule 9 against the final commit, **clippy as a single
invocation per target with the exit code captured explicitly**, plus `mdbook build`. Cross-target
clippy judged from your own diff.

**No CI control** — that is the architect's at push time.

One commit on `main`, local, **no push, no tag**.

## 11. Out of scope

NFC/NFD path normalization (deferred and documented at the site). Symlink target validation
(FDD-04 §5.4a; symlinks are fail-closed at three independent layers today). Any change to what modes
prikk *writes*. Anything in RFC 121, 126, or 130.
