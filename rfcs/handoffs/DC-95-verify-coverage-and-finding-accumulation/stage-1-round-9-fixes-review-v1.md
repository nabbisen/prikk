# DC-95 Stage 1, Round 9 Fixes — Review v1

**Reviewing:** `f91e451` on `dc-95-verify-coverage-and-finding-accumulation`, and the resubmitted
classified inventory.

**Accepted, no conditions.** Both required fixes discharged. Round 9 is now closed and round 10 is
cleared.

## 1. Verified

- **Rename complete.** No bare `publish_ref_to_new_block` survives anywhere under `crates/`; all three
  call sites carry the new name.
- **The doc comment does the work the name cannot.** It records all four confounded rounds (1, 2, 5, 9),
  states the mechanism — every probe built on it carries a permanent `PRIKK-TRUST-POLICY-INVALID`, so
  `Ok` from a disabled check is indistinguishable from a clean pass — names the replacement, and says
  the ugly name is deliberate. A future reader who hits it learns why rather than being merely blocked.
- **Inventory scope limit present**, at the head of the document rather than buried: this enumerates
  checks `verify` *has*, not checks it *should* have; `refs/received/` named as the known instance; and
  the closing sentence a reader actually needs — *"take this inventory as the map of what `verify`
  currently tests, not as a map of what `verify` should test."*
- **Gates at `f91e451`:** fmt clean, clippy **0**, **632** prikk-store tests — unchanged from round 9,
  as a pure rename should be. Worktree removed, primary tree clean.

## 2. Rename over delete was the better call, and the reasoning is right

The review offered both. They chose rename, and argued it: deleting would force migrating round 5's
three already-accepted, already-classified tests to a different construction **for no behavioural gain**
— those fixtures are correct as written, because production code rejects each one's real defect before
trust is ever consulted. Only a *probe* built on the helper is confounded, and none of round 5's three
still needs one.

**That distinction — committed test versus probe — is exactly right, and it is the distinction the
original hazard obscured.** The helper was never broken; it was misapplied. Renaming fixes the
misapplication without disturbing correct code, which is the narrower and therefore better fix.

I offered deletion first. Their reason for the other option is stronger than mine was.

## 3. The stale line they caught themselves

The opening paragraph still said "remaining 11 rows" against the table's 7. They found and fixed it
without being asked.

**That is the same defect class as round 8's stale line 9** — where the prose disagreed with the table
and I had to flag it. Catching it unprompted one round later is the correction landing rather than being
absorbed.

## 4. Standing

- **Round 9: closed.** Seven rows remain: 1 in §2 (`LEGACY-LOG-LEADS`, format-1 flip), 4 in §4, 1 in §5,
  1 in §7.
- **Round 10** cleared — `LEGACY-LOG-LEADS` to close §2, or §4's four as a block. Their call.
- Green three-platform CI before any merge.
