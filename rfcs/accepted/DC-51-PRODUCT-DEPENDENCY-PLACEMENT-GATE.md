# RFC (accepted) - DC-51 Product Dependency Placement Gate

**Status.** Accepted by the project owner on 2026-07-28. Raised as blocking finding B4 during DC-41 design
re-review and recorded there as a candidate follow-up increment. Design authored by the architect, then
re-examined by the same author (`prikk-dc51-author-reexamination-and-routing-answer-v1.md`) — which found
two defects now folded in: the `[target.*]` table gap and the `package = "..."` renaming bypass. That
re-examination was explicitly **not** an independent design review; the owner exercised the acceptance call
directly, which is the tier-2 economy that document recommended for a tool-only increment.
**Target milestone.** M2 - post-correction assurance milestone.
**Schedule position.** Independent; startable at any time. Rises in priority as DC-41 stage 4 and DC-43
add further development dependencies.
**Tracks.** DC-41 design re-review v1 finding B4.
**Touches.** `tools/release-policy` boundary command and its tests. No product code, no manifest change,
no CI change.

## Problem

Nothing mechanically prevents a third-party crate from being placed in a **product crate's**
`[dependencies]` instead of `[dev-dependencies]`. Both existing checks miss it:

- the DC-45 package-listing check (`boundary/package.rs`) inspects packaged **file paths**, rejecting
  `release/oracle/` and `tools/release-policy/` entries — it never reads dependency manifests;
- `boundary::check_dependencies` guards only the **tool↔product** edge and filters on
  `package.source.is_none()`, so a crates.io crate is outside its model entirely.

A misplacement therefore passes every gate — `boundary-check`, `reference-check`, `cargo audit`, the full
test suite — and ships as a runtime dependency of a published crate to every consumer. The exposure is
real and growing: DC-41 stage 3 added `sha2` to `prikk-hash` and stage 4 adds `proptest` to `prikk-object`
and `prikk-store`, all dev-only, all currently protected by review discipline alone.

DC-41 correctly documents that placement is review-enforced. DC-51 converts that from a documented
practice into an enforced invariant.

## Design

Extend the boundary command with one default-closed check over the seven product crates.

### Rule: `prikk-*` anywhere; third-party exact **per crate** (amended 2026-07-28)

An earlier draft specified one global name allowlist. That catches a test-only crate misplaced into some
product crate, but **not** a legitimate dependency appearing where it does not belong — `ed25519-dalek`
added to `prikk-cli` would pass a global list. The rule is therefore per-crate for third-party edges:

| Crate | Permitted third-party `[dependencies]` |
|---|---|
| `prikk-error`, `prikk-hash`, `prikk-object`, `prikk-replay`, `prikk` | *(none)* |
| `prikk-crypto` | `ed25519-dalek`, `getrandom` |
| `prikk-store` | `getrandom`, `rustix` |

Any key beginning `prikk-` is permitted in any product crate. Workspace-internal edges are already
governed by the workspace member list and by `check_dependencies`'s tool↔product reachability check, and
they churn legitimately during refactors; making internal churn trigger allowlist amendments would buy no
security and would train people to amend the list reflexively, which is how allowlists rot. Third-party
edges are the actual risk surface and change rarely, so exactness there is cheap and meaningful.

### Tables covered

`[dependencies]`, `[build-dependencies]`, and every `[target.*.dependencies]` /
`[target.*.build-dependencies]` table. The earlier draft omitted the `[target.*]` forms; none exist in a
product crate today, but they are equally real places a **shipping** dependency can live, and a gate that
ignores a valid dependency table has a documented bypass from the day it lands.

**`[dev-dependencies]` is deliberately not covered**, including under `[target.*]`. That is the sink this
gate exists to protect; constraining it would break `sha2` and `proptest` and defeat the purpose.

### Dependency renaming is rejected (amended 2026-07-28)

An earlier draft checked dependency **keys** only. Cargo's `package = "..."` field lets a key differ from
the crate it resolves to, so `getrandom = { package = "proptest", version = "1" }` in `prikk-store` would
declare proptest under an allowlisted key and pass a key-only check. Any dependency entry carrying a
`package` field in a product crate is therefore a violation regardless of the key. Renaming has no
legitimate use in these seven crates and it defeats key-based auditing, so it is forbidden rather than
resolved.

### Integration

New module `boundary/placement.rs` with category `dependency-placement`, inserted in `CATEGORY_ORDER`
immediately after the existing `dependency-boundary`. The name avoids collision with the existing
`check_dependencies`, which guards a different edge. Error detail is `<crate>:<dependency>`, matching the
`package-contents` convention. Manifests are read with the `toml::Value` pattern already used by
`check_tool`; a manifest that cannot be read or parsed **fails closed**, since a skipped manifest is an
unchecked crate.

This mirrors DC-45/DC-47/DC-48's exact-vector pattern rather than introducing inference or configuration:
adding a genuine production dependency becomes a reviewed amendment, which is the intended cost.

`tools/release-policy` is out of scope — it is `publish = false` and already governed by the tool↔product
edge check.

## Non-goals

- No constraint on `[dev-dependencies]`, feature flags, or version ranges.
- No new configuration file or inventory; the allowlist is source-level and exact, like the existing
  command productions.
- No change to product manifests as part of this RFC.

## Acceptance criteria

The check fails closed for a third-party crate placed in any product crate's `[dependencies]`,
`[build-dependencies]`, or `[target.*]` equivalents outside that crate's row, and passes for the current
tree unchanged. Tests cover: the real tree (regression guard, asserted against the repository rather than
a fixture); `sha2` misplaced into a product `[dependencies]` (fails); `proptest` in `[dev-dependencies]`
(passes — the sink stays open); `ed25519-dalek` in `prikk-crypto` (passes) and the same crate in
`prikk-cli` (**fails** — right dependency, wrong crate); a workspace-internal `prikk-*` edge (passes);
third-party under `[build-dependencies]` and under `[target.'cfg(unix)'.dependencies]` (both fail); and an
unreadable manifest (fails closed).

Existing boundary behaviour and error ordering are otherwise unchanged. Because this is a release-policy
control surface, it requires its own implementation review under the DC-45 precedent.
