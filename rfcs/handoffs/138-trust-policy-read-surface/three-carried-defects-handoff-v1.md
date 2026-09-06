# Three carried defects — one round

**Authority:** `ROADMAP.md`'s "Proposed ordering" item 1, refreshed and approved 2026-09-06.
**Filed under RFC 138** because all three were surfaced by that RFC's round or its review: **A** was
named in the RFC 138 implementation report, **B** was found while writing RFC 138 itself (§3), and
**C**'s edge was created by the RFC 138 commit and caught by RFC 130's gate the next day.
**Base:** `main` at `6fadfa6`.

> **Filing note, recorded rather than hidden.** This was first written to
> `rfcs/handoffs/carried-defects-20260906/` and **pushed with `boundary-check` failing** —
> `rfc-naming` refuses a handoff directory that names no RFC. The architect ran the gate, saw
> `"valid": false`, and committed in the same chained command without reading it. **The gate was
> right about more than the name**: RFC-000 requires every handoff to have a related RFC, and a
> batch citing only a schedule row had no clear authority. Corrected forward, not rewritten.

Three unrelated small items, batched because none deserves a round of its own. **They are independent
— if one turns out to be larger than described, land the other two and report rather than holding all
three.**

**C is not a function move. It is a graph change, and the gate you built yesterday is the instrument
that will tell you what it did.** Read §3 before starting it.

---

## A. `docs/src/reference/commands.md` is missing two shipped commands

**`prikk key` and `prikk setup` are absent from the master command inventory.** Both shipped in
**0.33.0**; the page has zero mentions of either. RFC 138's `trust maintainer list`/`check` are
already there (`:17-18`), so the gap is RFC 135's alone.

**This is a documentation defect already released**, which is why item 1 sits before the cut rather
than after it — 0.34.0 should not ship it a second time.

Add them in the page's existing idiom, including their flags. **Check the rest of the page for other
absences while you are in it** — that inventory has now been found incomplete twice (the previous
round found `trust maintainer remove` missing and completed the line it was editing), so a sweep is
warranted rather than a targeted insert. **Report what the sweep found, including "nothing else".**

## B. `policy: required=1` is printed as if read

`main.rs:295` and `setup.rs:106` both `println!("policy: required=1")` — **a literal, from a policy
that has no such field.** `MaintainerTrustPolicy` holds a `Vec` and nothing else; trust is any-of-N by
construction. The line is *true* and is *printed in the voice of a query*, which is the defect.

**Ruled: replace it with a derived count, and do not restate the trust semantics inline.**

Something of the shape `adopted maintainer keys: <n>`, where `<n>` comes from the policy rather than
from a constant. **Do not** add "any one may sign" or similar: that reads as ref authority, which RFC
138 §4.2 spent a constraint preventing, and **`prikk trust maintainer list` already carries that note
properly** — the line's job is now to say how many, not what trust means.

**Fan-out to update in the same commit:** `docs/src/guide/first-run.md` mirrors `setup`'s output. The
0.33.0 `CHANGELOG.md` entry also quotes it — **leave that alone**; it correctly records what 0.33.0
printed.

## C. Relocate `maintainer_trust_policy_or_empty` — and report what it does to the graph

RFC 130's `DECLARED_CYCLES` entry 5 says plainly that `trust ↔ recognition_claim` exists *"because a
convenience helper was called by its current address instead of being relocated"*, and recommends
moving `maintainer_trust_policy_or_empty` from `recognition_claim.rs` into `trust.rs`.

### C.1 It is more than three call sites

Production references to `recognition_claim::maintainer_trust_policy_or_empty`:

| Site | Already depends on `trust`? |
|---|---|
| `trust.rs:248` (the wrapper RFC 138 added — becomes the definition) | — |
| `tag_travel.rs:202` | **yes** (1 existing `crate::trust::`) |
| `seal_from_accepted.rs:62,162` | **yes** (1 existing) |
| `patch_exchange/accept.rs:24,253` | **no** — this one gains a new edge |
| `sync_negotiation/tests.rs:19,42,61` | test-only, excluded from the graph |

### C.2 The graph consequences, which you must measure rather than assume

**This removes one edge and may add another.** `trust -> recognition_claim` goes. `patch_exchange ->
trust` is **new**, because that module does not currently reference `trust` at all. The other two
callers already do, so they add nothing.

**Two allowlist entries are expected to go stale, and the reverse binding you shipped will fail until
they are removed:**

- **`DECLARED_CYCLES` entry 5** (`trust ↔ recognition_claim`) — its `trust -> recognition_claim` leg
  disappears.
- **`DECLARED_HUBS`'s `trust` entry** — `trust` crossed `HUB_THRESHOLD = 6` on fan-out 6, and that
  count included the edge being removed. At fan-out 5, `min(fan_in, fan_out)` drops below the
  threshold.

**That is the gate working on its first real use, not an obstacle.** Removing the entries is part of
the change, not a workaround for it.

**What you must not do is assume that list is complete.** Run `boundary-check` after the move and
**report its output verbatim** — including any consequence not predicted here. `patch_exchange ->
trust` is a new edge into a module inside the SCC; whether it closes a new cycle is a question for the
gate, not for this handoff.

### C.3 If the graph result is worse than the defect

If the relocation creates a **new cycle** rather than removing one, **stop and report.** Entry 5's
removal statement is a recommendation, not an instruction to make the graph worse, and this handoff
does not authorize declaring a new cycle to make a cleanup land.

## Controls

1. **A**: `prikk key` and `prikk setup` appear in `commands.md`, and the sweep result is reported.
2. **B**: the printed count is derived — adopt a second maintainer key and confirm the number changes.
   A constant that happens to read `1` is the defect, not the fix.
3. **B**: `first-run.md`'s mirrored output matches what `setup` now prints, checked by running
   `setup` rather than by reading the diff.
4. **C**: `boundary-check` is green after the move **and** the two predicted stale entries are gone —
   confirm by re-adding one and watching it fail, then removing it again.
5. **C**: the before/after edge delta, reported.

## Gates

The full set from `rfcs/EXECUTION-ORDER.md` §6 rule 9, run as the last action against the exact final
commit: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features --locked
-- -D warnings`; `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`;
`cargo +1.85.0 check --workspace --all-targets --locked`; `git diff --check`; `cargo audit
--no-fetch`; `RUSTDOCFLAGS="-D rustdoc::private_intra_doc_links" cargo doc --workspace --no-deps`;
release-policy `check`, `boundary-check`, `reference-check`.

**Quote every command.** Report `boundary-check` separately — C changes what it sees. Report the test
count per crate that moves.

## `CHANGELOG.md`

**Required, under `## Unreleased`.** B changes output a user sees; A changes published documentation.
C is internal and needs no line of its own. **This is asked for explicitly because two releases have
now shipped a user-facing change undocumented, both times because the handoff did not ask.**

## Reporting

Per `.git-exclude/tasks/dev-team/003-landing-work-on-main.md`: commit locally on `main`, do not push,
report to `.git-exclude/review-request/`. **One commit per defect is preferred** — they are
independent, and C is the one that might not land.
