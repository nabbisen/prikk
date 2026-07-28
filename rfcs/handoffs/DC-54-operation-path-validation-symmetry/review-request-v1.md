# DC-54 - Review Request (Post-Commit Evidence Review)

**Requested by:** developer (implementer role), 2026-07-28.
**Review type requested.** Post-commit evidence review — not a pre-commit implementation review.
DC-54 was implemented and committed on explicit project-owner instruction before any independent
review of the implementation occurred. This request exists to close that gap: confirm the committed
state matches the accepted design and holds up under independent verification, same bar as every
other implementation review this increment sequence has had.
**Baseline commit.** `35ae275` (last commit reviewed as HEAD before DC-41 stage 4 and DC-54 landed).
**Candidate commits (both already on `main`).**

| Commit | Subject | Review status |
|---|---|---|
| `2824695` | `test(store,object): add DC-41 stage-4 property/fuzz coverage` | Already reviewed and accepted **before** commit (`prikk-dc41-stage4-property-fuzz-implementation-review-v1.md`, verdict Accept). Committed content is unchanged from what was reviewed — out of scope for this request, included only for continuity. |
| `e8f780a` | `fix(object): validate operation paths at encode, closing DC-54's asymmetry` | **Not yet reviewed. This is the subject of this request.** |

**Diff scope for review:** `git diff 2824695..e8f780a` (14 files changed, 792 insertions(+), 147
deletions(-)).

## Design authority

`rfcs/accepted/DC-54-OPERATION-PATH-VALIDATION-SYMMETRY.md`. **Note on how this reached
`accepted/`:** not through an independent architect design review. The author's own
design-completion self-critique (`.git-exclude/reviewed/prikk-dc54-design-completion-self-critique-v1.md`)
explicitly issued no verdict and stated design acceptance was "the project owner's or another
reviewer's call." The project owner then directly authorized acceptance and implementation in this
session, exercising that call themselves rather than routing through a separate architect design
review round. Recorded here so the reviewer knows this candidate has had **no independent review of
any kind** yet — neither design nor implementation.

## What changed (`e8f780a`)

1. **Crate-boundary move.** `validate_repo_path` (+ private helpers `validate_component`,
   `is_windows_reserved_name`) moved from `crates/prikk-replay/src/path.rs` to new
   `crates/prikk-object/src/path.rs`, resolving a `prikk-object -> prikk-replay` dependency cycle
   the self-critique found in the original design draft. `prikk-replay::RepoPath::parse` now calls
   the moved function and re-exports it, so `prikk_replay::validate_repo_path` /
   `prikk_store::validate_repo_path` keep compiling unchanged for every existing caller.
2. **Encode-side validation.** `CreateFile`, `DeleteNode`, `RenamePath` (both `old_path` and
   `new_path`, independently), and `CreateSymlink`'s `validate()` now call the moved
   `validate_repo_path`, alongside the pre-existing `node_id.is_zero()` check. Error taxonomy:
   `InvalidName` propagates unwrapped, uniform across all four kinds.
3. **Untouched by design:** `DeleteNode.old_target`, `CreateSymlink.target` (opaque by accepted
   DC-40 design), the decoder, the `RepoPath` grammar, the FDD-03 §9.3 wire format.
4. **Test matrix:** `prikk-object::payload::tests::path_validation` (4 tests, encode-only matrix)
   and `prikk-store::patch_replay::tests::dc54_encode_decode_symmetry` (3 tests, including a
   symmetry proof and a test pinning the exact DC-41 reproducer to fail at encode).
5. **`crates/prikk-store/src/patch_replay/tests/proptest_round_trip.rs`:** the DC-41 stage-4
   disclosed exclusion filter (Windows-reserved path names) is removed entirely; the round-trip
   property now treats a legitimate encode-side rejection as a skip (`let Ok(bytes) = ... else {
   return Ok(()); }`) rather than assuming encode always succeeds.

## Specific points requesting independent scrutiny

- **The crate-boundary move itself** (§1 above) — the self-critique flagged this as "the part of
  this RFC most deserving of independent scrutiny," since it touches a boundary DC-19/DC-20
  deliberately drew (`prikk-replay` as the lexical-leaf owner of `RepoPath`). Confirm the move is
  genuinely pure lexical validation (no `RepoPath` type, no lifecycle semantics) and that leaving
  `RepoPath` itself + `validate_no_path_collisions` in `prikk-replay` is the right line.
- **RenamePath's two-field independence.** Confirm `old_path` and `new_path` are truly validated
  independently (test: `rename_path_rejects_a_bad_old_path_even_when_new_path_is_valid`), not just
  that the DC-41 reproducer's specific shape (`new_path` bad, `old_path` valid) happens to pass.
- **Existing-repository compatibility claim.** Evidenced in the implementation-evidence note via two
  direct citations (`worktree_patch/node_authoring/worktree_files.rs:72`,
  `node_authoring.rs:340`) — confirm those citations are accurate and sufficient, not merely
  plausible.
- **Identity-neutrality claim.** Evidenced by "every existing FDD golden vector and canonical
  snapshot test still passes with identical bytes" — confirm this is a sound proof method for "no
  existing ObjectId changed," not just circumstantial.
- **The symmetry proof's actual strength.** `dc54_encode_decode_symmetry.rs` compares
  `RenamePath::validate()`'s error string against `RepoPath::parse()`'s error string for the same
  input — confirm this proves what it claims to prove (both call the same underlying function by
  construction) rather than being a tautological test.
- **Frozen identities.** `Cargo.lock` claimed unchanged (`601d0678…5da31`, 180 packages) since this
  increment adds no dependency — worth an independent `sha256sum` / `grep -c` check rather than
  trusting the evidence note's own numbers.

## Evidence already produced

- `rfcs/handoffs/DC-54-operation-path-validation-symmetry/implementation-evidence-v1.md` — full gate
  output, test counts (`prikk-object` 72→76, `prikk-store` 540→543), unfiltered 100,000-case campaign
  run twice (clean both times), compatibility/identity-neutrality evidence.
- `rfcs/handoffs/DC-54-operation-path-validation-symmetry/implementation-handoff-v1.md` — the
  architect-authored implementation handoff this candidate was built from.

## What this request does not cover

- DC-41 stage 4 (`2824695`) — already reviewed and accepted pre-commit; unchanged since.
- DC-51, DC-49/50/52/53, or any other queued increment — untouched by this commit.
- Any release-lane action. The lane remains **parked**; nothing in this request activates it, and
  this is an implementation-correctness review, not a release readiness claim.

## Requested outcome

Either an **Accept** (matching the bar every other implementation review in this sequence has
applied) or specific findings with repair guidance, same format as prior implementation reviews
(`prikk-dc41-stage*-implementation-review-v1.md`).
