# DC-85 — Review Amendments Accepted

**Reviewing:** `a8f3f61` on `dc-85-merge-from-received-ref`, on top of the reviewed `54d52a7`.
**Responds to:** the dev team's `prikk-dc-85-review-amendments-v1.md`.

**Both discharged. §5.1 (green macOS CI before merge) is the only condition still open.**

## §5.2 — self-merge guard symmetry

Correct, and slightly better than what I asked for. Rather than patching the comparison,
`execute_merge` now returns the canonical string from the same `if`/`else` that builds the
target, so there is exactly one place the name is derived and no second raw-argument path
for a future normalization to diverge from. The received arm keeps the raw string, which is
right — `validate_received_ref` returns `()` and defines no canonical form, so there is
nothing else it could carry.

Logic-neutral, as claimed: 602 prikk-store lib tests unchanged, and I re-ran both DC-85
CLI tests at `a8f3f61` (2 passed).

## §5.3 — docs, and a correction to my own review

**They found something I missed, and it matters.** My §5.3 said "nothing there is now
*false*." That was true of `merge.md`, which is the only file I checked. It was not true of
`merge-plan.md:24`, which read "Ref selectors resolve only through the current local branch
target block" — a statement DC-85 made incorrect. I characterized the gap as purely
additive when part of it was a correction. They caught it and fixed it rather than layering
new text on top of a false line, which is exactly the right instinct.

The three files:

- `merge.md` — new "Merging from a received ref" section. It states the asymmetry (`--from`
  accepts `remotes/`, `--into` never will), explains why the gate exists in terms of the
  induction rather than as a bare rule, and gives the guidance I asked for against
  reflexively running `trust maintainer add` to clear the refusal. The framing — "trusting a
  maintainer key means trusting every block that key has ever sealed or ever will" — puts
  the cost of the decision in front of the operator, which is the point. The cross-reference
  anchor resolves (`## Merging from a received ref` at line 60).
- `merge-plan.md` — the false line corrected, plus §6.3's caveat.
- `merge-evidence.md` — the equivalent addition for both selectors.

`mdbook build docs` is clean. (The `mdbook-mermaid` 0.5.0-vs-0.5.4 version warning is
pre-existing and unrelated to this change.)

## Gates, re-run by me at `a8f3f61`

fmt clean; clippy `--workspace --all-targets --all-features --locked -D warnings` clean;
`cargo test --workspace --locked` green; `cargo +1.85.0 test --workspace --locked` green;
`git diff --check` clean; `cargo audit --no-fetch` 179 dependencies, nothing flagged;
release-policy `check` 154 oracle cases passed, `boundary-check` and `reference-check` both
`"valid": true`.

## Standing

§6.1 (authenticate before parse), §6.2 (revocation-design constraint), and §6.3 (previews
can show an unexecutable plan) remain recorded as noted — none were conditions and none are
affected by this commit.

`dc-85-merge-from-received-ref` is accepted for merge the moment a macOS CI run on the
branch comes back green.
