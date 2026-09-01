# RFC 125 — Four places the decoder accepts what the encoder would never write

**Status.** **COMPLETE, 2026-09-02.** Mode canonicality, path length caps, duplicate singular
fields and the byte-fed `unreachable!` sites landed at `7bf1279`; the duplicate-field class was
extended to the operation-payload decoders at `c6fc625`. CI green on all 15 jobs.

**§6's compatibility question was answered with evidence, not argument** — the whole point of asking
it. `release_compatibility_gate::` (5/5, against a repository written by 0.27.0's own encoder) and
`format_transition` (3/3) both pass against the tightened decoders, twice: after each round.
**No existing history is refused.**

**The class was larger than this RFC said, in two directions.** §2.3 named two files; the
implementation found seven, plus a second decoder inside one of them. Then the amendment found the
class had also been applied one level too **shallow** — the operation-*payload* decoders in
`prikk-store` were untouched, 29 unguarded fields, in the very file the mode work had just edited.
**A site list can be short; a class can also be shallow.** Enumerate by mechanism — every TLV decode
loop, wherever it lives — not by crate.

**One caveat on that mechanism, recorded for the next enumeration:** `next_field()` is narrower than
the defect class. `lifecycle_cache/cache_ladder.rs` and `lifecycle_cache/incremental.rs` decode TLV
without it. Both were already strict, so the narrower grep was sufficient here **by luck, not by
construction.**

Raised by the external architecture audit of 2026-08-31
(`audit-2026-08-31-task-1a.md` §1.1/§1.2, `task-1b.md` §3); all four independently confirmed at
`3a8d730`.

**Tracks.** The format boundary — what `prikk` accepts from bytes it did not write. Governed by
DC-54's encode/decode symmetry principle, which each of these four violates in the same direction.

---

## 1. The common shape

DC-54 established that **encode and decode must call the same validator**, so a hostile or corrupt
input cannot enter through a path the authoring side would have refused. These four sites each let
the decoder be more permissive than the encoder, and in each case the thing that saves the system
today is a guard somewhere else.

**None of these is exploitable now. All four are one refactor away from mattering**, and the audit's
phrasing for the first is the right frame for all four: *fragile-by-distance.*

## 2. The four

### 2.1 File modes: any `u32` decodes; only two are ever written — **Medium**

`patch_replay/decode/operations.rs:23,244-245` accepts an arbitrary `u32` mode on `CreateFile` and
`ChangePerm`. Materialization applies it: `worktree.rs:154-158` → `fsutil/anchored/linux.rs:108-115`
performs `fchmod(mode & 0o7777)`, which admits setuid, setgid, and sticky bits.

**What saves it today:** every history-advancing boundary derives a state root whose `validate_entry`
admits exactly `REGULAR_MODE` (`0o100644`) and `EXECUTABLE_MODE` (`0o100755`) —
`state_root.rs:15-16,163-180` — and received refs cannot be materialized at all
(`patch_replay.rs:259` refuses `remotes/`).

**Why that is not enough:** the guard is at seal time, in a different subsystem, for a different
purpose. Two plausible future changes re-open it independently — materializing a received ref, or
relaxing state-root validation. Neither would look like a security change to whoever made it.

**Fix:** reject non-canonical modes at operation decode, and/or normalize at
`set_regular_file_mode_required`. **Compatibility question this raises:** tightening a decoder can
refuse history that already exists. In practice no such history can exist — the seal boundary has
always refused it — but that argument must be *verified against real fixtures*, not asserted, before
the decoder tightens.

### 2.2 No length caps in the repo-path grammar — **Medium**

`validate_repo_path` (`prikk-object/src/path.rs:27-114`) is thorough about *shape* — absolute paths,
backslashes, colons, control characters, `.`/`..`, trailing dot or space, Windows device names,
`.prikk` as first component — and contains **no length check of any kind** (confirmed: no `MAX`, no
`255`, no `4096` anywhere in the file).

A 100 KB path or a 300-byte component therefore passes validation, enters signed history, and then
**cannot be materialized** on Linux (`NAME_MAX` 255) or Windows (260 default), surfacing as a raw OS
error at checkout on a repository that verifies clean. That is an availability defect that arrives
long after the commit that caused it.

**Fix:** cap component length and total path length in `validate_repo_path`, so the refusal happens
at authoring — encode/decode symmetric, therefore also refused on receipt. Record the numbers in
`docs/src/reference/path-safety.md`, which already owns this grammar.

### 2.3 Duplicate singular TLV fields decode last-wins — **Low**

`payload/block.rs:81-96` and `payload/refs.rs:78-108,203-217` accept a repeated singular field and
keep the last, so **two distinct byte-strings decode to one logical value**. `PatchPurpose::decode_from_patch_payload`
already rejects duplicates — the precedent exists inside the same crate and was simply not applied
uniformly.

Identity covers the raw bytes, so this is not a substitution attack; it is a canonicalization gap,
and canonicalization gaps in a content-addressed system are the class worth closing early.

**Fix:** `seen` flags for singular fields in every decoder, matching the `PatchPurpose` precedent.

### 2.4 Two `unreachable!`s on paths fed by external bytes — **Low**

`patch_replay/decode.rs:290` (*"kind tag is constrained to 10..=16 above"*) and `verify/objects.rs:207`.
Both invariants hold today and both are maintained by an *adjacent* match arm rather than by the
site that relies on them — the same fragile-by-distance shape as §2.1.

**Fix:** return `MalformedData`. A panic on attacker-influenced bytes is a denial-of-service shape
even when the invariant is currently true, and this workspace's own standard elsewhere is that
external input never panics — the decoder-totality proptests exist precisely to assert that.

## 3. Why one RFC and not four

Splitting them would produce four increments that each re-argue the same principle. They share a
governing rule (DC-54), a subsystem (the decode boundary), a test strategy (extend the existing
decoder-totality proptests with the new refusals), and one compatibility question (§2.1's, which
§2.2 shares in weaker form): **can tightening a decoder refuse history that already exists?** That
question should be answered once, with fixtures, for all four.

## 4. Scope

**In:** the four refusals, their tests, the fixture-based compatibility check, and the
`path-safety.md` update.

**Out:** NFC/NFD normalization of paths (deferred and documented at the site); symlink target
validation (FDD-04 §5.4a, and symlinks are fail-closed at three independent layers today); any change
to what modes prikk *writes*.

## 5. Constraint

**Every one of these tightenings must be encode/decode symmetric.** A refusal added only at decode
would let this project author history that it then refuses to read — the exact inversion of the
defect being fixed.
