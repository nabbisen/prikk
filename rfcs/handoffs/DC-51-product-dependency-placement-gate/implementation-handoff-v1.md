# DC-51 Product Dependency Placement Gate - Implementation Handoff

**Cleared to start.** DC-51 was accepted by the project owner on 2026-07-28 and now lives at
`rfcs/accepted/DC-51-PRODUCT-DEPENDENCY-PLACEMENT-GATE.md`. No gate remains — begin implementation.
**Authored by** the architect (function-designer role). Implementation review remains independent, since
developers implement and the architect reviews.
**Design amendments folded in (read these, they change the check):** `[target.*]` dependency tables are in
scope, and dependency **renaming** (`package = "..."`) is rejected outright — a key-only check would let
`getrandom = { package = "proptest" }` through under an allowlisted key.
**Size:** small — one new check, one per-crate allowlist, ten test cases.
**Touches:** `tools/release-policy` only. No product code, no manifest, no CI.

## Why this exists

Nothing mechanically stops a third-party crate being placed in a **product** crate's `[dependencies]`
instead of `[dev-dependencies]`. It would ship to every consumer of a published crate, and every current
gate passes: the DC-45 package-listing check reads packaged **file paths**, and
`boundary::check_dependencies` filters on `package.source.is_none()` so crates.io packages are outside its
model. DC-41 stage 3 added `sha2` and stage 4 adds `proptest` — all dev-only, all currently protected by
review discipline alone.

## Implementation targets

`tools/release-policy/src/boundary.rs` already has the pieces:

- `PRODUCTS: [(&str, &str); 7]` (`:12-20`) maps each product crate to its manifest path — reuse it, do
  not redefine the list.
- `CATEGORY_ORDER: [&str; 8]` (`:21`) fixes error-category ordering — **add the new category here** or
  deterministic sorting breaks.
- `mod package; mod publication;` (`:1-2`) — add a sibling module rather than growing `boundary.rs`.
  The module name is `placement`; see **Naming** below for why not `dependencies`.

The check:

1. For each of the seven `PRODUCTS` manifests, parse `[dependencies]`, `[build-dependencies]`, **and every
   `[target.*.dependencies]` / `[target.*.build-dependencies]` table**. All of these ship.
2. Permit any key beginning `prikk-` in any crate. For third-party keys, fail unless the key appears in
   that crate's row:

   | Crate | Permitted third-party |
   |---|---|
   | `prikk-error`, `prikk-hash`, `prikk-object`, `prikk-replay`, `prikk` | *(none)* |
   | `prikk-crypto` | `ed25519-dalek`, `getrandom` |
   | `prikk-store` | `getrandom`, `rustix` |

   Per-crate rather than one global list, so `ed25519-dalek` appearing in `prikk-cli` is caught too —
   right dependency, wrong crate.
3. **Ignore `[dev-dependencies]` entirely**, including under `[target.*]` — it is the sink this gate
   protects. Constraining it would break `sha2` and `proptest`.
4. Emit one error per violation, `<crate>:<dependency>`, matching the existing `package-contents`
   convention.
5. **Reject dependency renaming outright.** A dependency entry whose table carries a `package = "..."`
   field is a violation in a product crate, regardless of the key. Cargo lets
   `getrandom = { package = "proptest", version = "1" }` declare proptest under an allowlisted key — a
   key-only check would pass it. Renaming has no legitimate use in these seven crates and it defeats
   key-based auditing, so forbid it rather than trying to resolve effective names.
6. **Fail closed** if a manifest cannot be read or parsed — a skipped manifest is an unchecked crate.

`toml` is already a dependency of the tool, so no new dependency is needed. `tools/release-policy` itself
is out of scope — it is `publish = false` and already covered by the tool↔product edge check.

**Naming.** Use module `boundary/placement.rs` and category `dependency-placement`. Do **not** name it
`dependencies` — `check_dependencies` already exists in `boundary.rs` guarding the tool↔product edge, and
two things named "dependencies" doing different jobs is a confusion trap.

## Traps

- **Do not** use `cargo_metadata`'s resolved graph for this. It reports the *resolved* dependency set,
  where dev and normal dependencies are hard to separate per-crate and workspace inheritance is already
  flattened. Read the manifests directly — the question is literally "which TOML table is this key in."
- **Do not** make the allowlist configurable or inventory-driven. Exact and source-level is the point, and
  it matches the DC-45/DC-47/DC-48 pattern: adding a genuine production dependency should cost a reviewed
  amendment.
- Handle both value shapes. A dependency entry may be a string (`ed25519-dalek = "2"`) or a table
  (`getrandom = { workspace = true }`, `rustix = { version = "1", features = ["fs"] }`). The key is what
  the allowlist matches against — **but the table is not ignorable**: per check step 5, a table carrying
  a `package` field is a violation on its own. Read the key for the allowlist, and inspect the table for
  `package`.
- Adding a category to `CATEGORY_ORDER` changes report ordering; update any test that pins the full
  ordering.

## Required tests

Table-driven against temporary manifests, except the first:

| Case | Expect |
|---|---|
| **Current real tree** | **passes unchanged** — regression guard, asserted against the repository, not a fixture |
| `sha2` in a product `[dependencies]` | **fail**, naming crate and dependency |
| `proptest` in `[dev-dependencies]` | pass — the sink stays open |
| `ed25519-dalek` in `prikk-crypto` | pass — allowlisted for that crate |
| `ed25519-dalek` in `prikk-cli` | **fail** — right dependency, wrong crate |
| `prikk-object` in `prikk-store` | pass — workspace-internal |
| Third-party under `[build-dependencies]` | **fail** |
| Third-party under `[target.'cfg(unix)'.dependencies]` | **fail** |
| `getrandom = { package = "proptest", version = "1" }` in `prikk-store` | **fail** — renamed dependency under an allowlisted key |
| Unreadable / unparseable manifest | **fail closed** |

The `prikk-cli` and `[target.*]` cases are the ones that justify the design over the simpler global-list
version — do not drop them.

## Definition of done

- New check wired into the boundary command, category registered in `CATEGORY_ORDER`.
- All ten test cases above, including the real-tree regression guard.
- `boundary-check` still `valid: true` on the current tree.
- No product manifest changed by this increment.
- Test count reported before/after for `prikk-release-policy`.
- Frozen identities unchanged (`Cargo.lock`, `Cargo.toml`, all product manifests, both inventories,
  oracle manifest, `release-signers.toml`).
- Full gate set green (see `rfcs/EXECUTION-ORDER.md` §6.8).

## Submit with

Diff; evidence note stating the allowlist chosen and why each entry is on it; all ten test outcomes;
confirmation the real tree passes; gate output; explicit statement that no product manifest changed.

**After this lands,** standing rule §6.6 in `rfcs/EXECUTION-ORDER.md` (dependency placement is
review-enforced) can be downgraded to defense-in-depth, and future dependency-adding candidates no longer
need manual manifest inspection as their primary control.
