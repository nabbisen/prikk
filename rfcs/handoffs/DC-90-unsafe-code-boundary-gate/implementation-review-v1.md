# DC-90 — Implementation Review v1

**Reviewing:** `baa4b38` on `dc-90-unsafe-code-boundary-gate`, off `main`.

**Verdict: ACCEPT, conditional on one word changed in two places (§2).** The design is right, the
tests are the right tests, and the documentation is better than I asked for. But the gate as built can
be bypassed by the exact party it exists to constrain, and I proved it rather than deduced it.

## 1. What is right, verified rather than accepted

**The self-guarding arrangement is real.** They verified Cargo refuses `[lints] workspace = true`
alongside a local override, so a crate has exactly two options — full inheritance or full opt-out — with
no third path that inherits everything except the one lint it wants to escape. Building the probe rather
than assuming it was correct.

**The inheritance rule fires against the real tree.** My own negative control: comment out
`workspace = true` in `prikk-hash/Cargo.toml`, run `boundary-check` —

```
"valid": false
"category": "unsafe-boundary"
"detail": "prikk-hash: does not inherit workspace lints and is not in the exemption list"
```

The named message, not a generic failure.

**Criterion 3 is genuinely met.** `real_tree_passes_unchanged` and `baseline_tree_passes` exercise
`check()` with the real, empty exemption list; there is no skip-when-empty branch. Zero exemptions is a
checked state, not a dormant one.

**The test set covers each rule with its own message**, including the case I named as the one worth
proving hardest — `exempt_crate_opting_out_without_redeclaring_the_lint_fails` — plus its positive
counterpart, `non_exempt_crate_opting_out_with_the_lint_redeclared_still_fails` (which closes the
obvious wrong reading of the rule), and both fail-closed manifest cases. Injecting the exemption list
into `check_member` rather than mutating the real constant is the right call.

**§4.4's documentation exceeds the condition.** The staleness limit is stated in the terms I asked for —
*"the gate stays green while the guarantee it implies quietly stops being true"* — and
`architecture.md`'s new section points a reader at the module doc rather than paraphrasing it into a
second source of truth. It also states, correctly, that `forbid` governs code prikk writes and not code
it depends on, which is the correction from the DC-87 analysis carried into user-facing docs.

**Gates re-run by me at `baa4b38`:** fmt clean, clippy clean, `cargo test --workspace --locked` green,
`prikk-release-policy` 82 tests, `boundary-check` valid including the new category. The process note
about the stray `git stash pop` is accurate — `stash@{0}` is intact and the working tree is clean; I
checked. Disclosing it was right.

## 2. The condition: `"deny"` cannot hold this boundary. It must be `"forbid"`.

**The whole point of this increment is that the controlled party cannot remove the control. As built,
it can — in one line, in its own source.**

The exempt crate's escape route:

1. Opt out of workspace lint inheritance — legitimate, and the only way to write `unsafe` at all.
2. Locally re-declare `undocumented_unsafe_blocks = "deny"` — **satisfies this new gate exactly.**
3. Add `#![allow(clippy::undocumented_unsafe_blocks)]` at its crate root.
4. Every gate in the set passes. The SAFETY-comment requirement is gone.

I did not reason this out and assert it. I built it:

**Manifest `deny` + source `allow`** — the lint never fires. `cargo clippy` exits 0. The only output is
an unrelated `missing_safety_doc` warning.

**Manifest `forbid` + source `allow`:**

```
error[E0453]: allow(clippy::undocumented_unsafe_blocks) incompatible with previous forbid
  = note: `forbid` lint level was set on command line (`-F clippy::undocumented_unsafe_blocks`)
error: could not compile `bypass` (lib) due to 1 previous error
```

**This increment already knew the principle and applied it to the wrong half.** DC-87's investigation
established — and this module's own reasoning descends from — that `forbid` cannot be overridden by an
inner `allow`, *"the whole difference between `forbid` and `deny` in Rust."* That is exactly why
`unsafe_code = "forbid"` is robust. The lint that **guards** `unsafe_code`'s exception was then set at
`deny`, which is the level that can be overridden. The guard is weaker than the thing it guards.

**The fix, and I verified it is safe before requiring it:**

- Root `Cargo.toml`: `undocumented_unsafe_blocks = "forbid"`.
- `unsafe_boundary.rs`: `SELF_GUARDING_LEVEL = "forbid"`.

With both applied to the real tree: `cargo clippy --workspace --all-targets --all-features --locked
-- -D warnings` clean, `cargo build --workspace --locked` clean, `boundary-check` still `"valid": true`.
A `forbid`-level *clippy* lint does not disturb plain `rustc` builds or `cargo test` — I checked that
separately on a scratch crate, because "forbid a tool lint" is the kind of thing that could plausibly
break a non-clippy build and does not.

**One test to add with it:** a case asserting the gate rejects a re-declaration at `"deny"`. Today
`check_member` compares against `SELF_GUARDING_LEVEL`, so flipping the constant makes that behaviour
follow automatically — but the *reason* `deny` is insufficient deserves to be pinned by a test and
named in the module doc, or the next reader will see two levels that look equivalent and pick the
familiar one. The module doc's "what makes this self-guarding" paragraph is where that sentence belongs.

## 3. Not conditions

**`check_member` returns OK for an exempt crate that still inherits.** Correct — inheritance means
`forbid(unsafe_code)` applies, so the exemption is simply unused. `exempt_crate_keeping_full_inheritance_passes`
pins it. No change.

**Rejecting the `{ level = "..." }` table form of a lint level.** Deliberate, documented, and right for
a project that writes plain strings everywhere. Accepting both forms would be more permissive than the
tree needs.

**The separate third-party allowlist for a future exempt crate**, kept distinct from `ALLOWED_THIRD_PARTY`
rather than layered onto it. That was the RFC's intent and it is implemented as intended.

## 4. Standing

- **Merges after §2 and an ordinary CI run.** This touches no filesystem-backed state, so the
  three-platform rule does not bind it; the ordinary run is enough.
- **DC-90 landing is what unblocks DC-87 Stage 2's first `unsafe` line.** Getting the level right
  matters more here than the schedule: a boundary that can be removed by the crate it constrains would
  have shipped as a control and functioned as a convention.
