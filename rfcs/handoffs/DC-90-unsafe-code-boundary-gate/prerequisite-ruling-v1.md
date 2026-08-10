# DC-90 — Prerequisite Investigation Ruling v1

**Reviewing:** `.git-exclude/review-request/prikk-dc-90-prerequisite-questions-v1.md`.

**Investigation accepted. Cleared to design**, with one correction that changes the design (§2) and one
consequence of that correction that nobody has drawn yet (§3) — and §3 is the reason this increment
still needs to exist at all.

## 1. Verified, including the part that mattered

I ran my own probe rather than accepting the transcript. A scratch crate outside the workspace, one
`unsafe` block with only a function-level `///` doc comment above it and one with a `// SAFETY:` line
immediately preceding:

- Toolchain confirmed: `clippy 0.1.97 (8bab26f4f6 2026-07-14)`, `rustc 1.97.1` — their version claim is
  accurate.
- The uncommented block fails; **the properly commented one does not**. The lint discriminates on the
  actual rule, and their finding that a function-level doc comment does not satisfy it reproduces
  exactly.

`cargo-geiger`'s characterisation is right too — statistics for a human, not a pass/fail gate — and
declining to add it is correct.

§4.2's independent re-measurement matches, including the `prikk-crypto` presentation asymmetry, which
they confirmed by reading all eight roots rather than trusting mine. Good.

§4.4's enumeration is the best part of the report. FFI-ABI correctness, comment *content* versus
*presence*, and staleness are each stated with the review obligation that has to cover them, and the
`// SAFETY: trust me` example makes the limit concrete instead of abstract. That is criterion 5
substantially answered before design.

## 2. Correction: the gate set runs clippy; it does not run this lint

§4.1 concludes the SAFETY-comment rule is *"already enforced by a tool this project's gate set already
runs on every commit."* **The tool runs. The lint does not.**

My probe: `cargo clippy -- -D warnings` against the uncommented block produced **zero** matches for
"unsafe block missing a safety comment." Only the explicit
`-D clippy::undocumented_unsafe_blocks` fires it. `undocumented_unsafe_blocks` is a `restriction`-group
lint — allow-by-default, and not reached by `-D warnings`.

Their body text is aware of this — it says the lint must be added "to the one exception crate's own lint
table (or passing it workspace-wide)". The summary line overstates it, and the distinction is not
cosmetic, because of §3.

## 3. The consequence nobody has drawn: the rule must not be opt-out by the crate it constrains

If the SAFETY-comment rule is enabled in **the exception crate's own `Cargo.toml`**, then the one crate
in the workspace permitted to write `unsafe` **can switch off its own guard by deleting one line** —
and every gate in the set still passes, because `-D warnings` never checked it.

That is a gate that protects everything except the thing it exists to protect.

**Ruling — the shape to design toward:**

1. **`undocumented_unsafe_blocks = "deny"` goes in the root `[workspace.lints.clippy]` table**, beside
   the `unwrap_used` / `expect_used` / `indexing_slicing` entries already there. Workspace-wide it is a
   no-op everywhere `forbid(unsafe_code)` already holds, exactly as they observed.
2. **`release-policy` must verify that every member still inherits it** — `[lints]` / `workspace = true`
   present in every member manifest. It already has to read that table for the `forbid` rule, so this is
   the same check, not a new one.
3. **A crate dropping workspace lint inheritance is itself a gate failure.** That is the rule which makes
   the arrangement self-guarding: the exception crate cannot escape the SAFETY-comment requirement
   without failing a check that does not depend on the lint it is trying to escape.

This is what "under control" has to mean here. A control the controlled party can remove is a
convention.

## 4. Rulings on the four questions

**4.1 — accepted, with §2's correction.** The finding stands and it is the report's headline: the
expensive half of this increment does not need building. Enabling an existing lint plus one
inheritance check replaces a Rust-source parser in `release-policy`. That is a genuinely better outcome
than the RFC anticipated, and it was found by testing rather than reasoning.

**4.2 — accepted.** The `prikk-crypto` asymmetry is a real decision and §3 resolves it as a side effect:
**the manifest is authoritative**, because manifest inheritance is what both the `forbid` rule and the
new lint rule ride on. Source-level `#![forbid(unsafe_code)]` attributes become redundant belt-and-braces
rather than a second source of truth. **Do not require them, do not strip them** — adding a rule that
sweeps six crates' existing attributes out is scope this increment has no reason to take.

**4.3 — accepted.** No new dependency. `cargo-geiger` correctly declined.

**4.4 — accepted, and it sets the standard for criterion 5.** One addition: the staleness limit is the
one most likely to bite, because it degrades silently and a green gate looks identical either way. State
it in the tool's own documentation in those terms, not only as a list entry.

## 5. Conditions on the design

1. **§3's self-guarding arrangement**, or a reported reason it cannot be built that way.
2. **Negative controls per rule** (RFC criterion 2) — and one of them must be specifically: *the
   exception crate attempts to opt out of workspace lint inheritance, and the gate fails.* That is the
   §3 rule; it is the one worth proving hardest.
3. **Criterion 3 stands**: an allowlist of zero exempt crates must be a valid checked state today, not a
   special case that starts working once something is added.
4. **What the gate cannot see** goes in the tool's own documentation with its review obligation — §4.4's
   list is the content; it needs a home a contributor reaches.

## 6. Standing

- **DC-90: cleared to design and implement**, under §5.
- **DC-88** was ruled separately today and collapsed to a small restatement; **it no longer blocks
  DC-87 Stage 2, and Stage 1's seam refactor is released from hold.** Both are available work.
- **DC-87 Stage 2** still waits on DC-90 landing before any `unsafe` line, and on its own design
  answering how `atomic_replace`/`promote`/`durable_append` are satisfied without directory durability.
