# DC-51 Product Dependency Placement Gate - Implementation Evidence v1

**Date:** 2026-07-28
**Handoff followed:** `implementation-handoff-v1.md`, cleared to start after project-owner acceptance
(`d7d49c6`).
**Touches:** `tools/release-policy/src/boundary.rs`,
`tools/release-policy/src/boundary/placement.rs` (new),
`tools/release-policy/src/boundary/placement/tests.rs` (new). No product code, no manifest, no CI —
matches the handoff's stated scope.

## What changed

- New module `boundary/placement.rs`, wired in via `mod placement;` and
  `placement::check(root, &mut errors);` in `boundary::run`.
- `CATEGORY_ORDER` extended from 8 to 9 entries; `"dependency-placement"` inserted immediately after
  `"dependency-boundary"`, per the design's naming/ordering resolution.
- The check reuses `PRODUCTS` (not redefined) and reads each of the seven product manifests directly
  with `toml::from_str::<toml::Value>`, matching the existing `check_tool` pattern — no
  `cargo_metadata` resolved-graph use, per the handoff's explicit trap.

## Allowlist chosen and why each entry is on it

Per-crate exact third-party sets, `prikk-*` permitted anywhere (workspace-internal edges are governed
elsewhere and churn legitimately):

| Crate | Permitted third-party | Why |
|---|---|---|
| `prikk-error`, `prikk-hash`, `prikk-object`, `prikk-replay`, `prikk` | *(none)* | No third-party dependency in these crates today |
| `prikk-crypto` | `ed25519-dalek`, `getrandom` | Both currently present in `crates/prikk-crypto/Cargo.toml`'s `[dependencies]` |
| `prikk-store` | `getrandom`, `rustix` | Both currently present in `crates/prikk-store/Cargo.toml`'s `[dependencies]` |

This is the exact set the design's self-critique verified against the tree (§1, "What holds") and
matches what `boundary-check` confirms still passes below.

## Tables covered

`[dependencies]`, `[build-dependencies]`, and every `[target.*.dependencies]` /
`[target.*.build-dependencies]` table. `[dev-dependencies]` — including under `[target.*]` — is
deliberately never inspected; it is the sink this gate protects and currently holds `sha2` and
`proptest`.

## Renaming defense (F1)

Any dependency entry whose table carries a `package = "..."` field is a violation regardless of key,
including a `prikk-*` key — `getrandom = { package = "proptest", version = "1" }` in `prikk-store`
would otherwise pass under the allowlisted `getrandom` key while actually shipping `proptest`. Checked
before the allowlist lookup, so it can't be bypassed by an otherwise-permitted key.

## Failure mode

An unreadable or unparseable manifest is a pushed `dependency-placement` violation
(`<crate>: manifest unreadable` / `<crate>: manifest unparseable`), not a skip — consistent with
`package.rs`'s existing precedent for the oracle manifest, and with the handoff's fail-closed
requirement. `boundary::run` itself does not error on this path; the report's `valid` flag goes false.

## Test outcomes — all eleven cases (ten required plus one)

Table-driven against temporary manifests built from `PRODUCTS`' relative paths (`write_baseline` /
`write_manifest` in `boundary/placement/tests.rs`), except the real-tree regression guard:

| Case | Result |
|---|---|
| Current real tree | pass (`real_tree_passes_unchanged`) |
| `sha2` in a product `[dependencies]` | fail, `prikk-hash:sha2` (`disallowed_third_party_in_product_dependencies_fails`) |
| `proptest` in `[dev-dependencies]` | pass (`dev_dependency_sink_stays_open`) |
| `ed25519-dalek` in `prikk-crypto` | pass (`allowlisted_third_party_in_its_own_crate_passes`) |
| `ed25519-dalek` in `prikk-cli` (crate `prikk`) | fail, `prikk:ed25519-dalek` (`right_dependency_wrong_crate_fails`) |
| `prikk-object` in `prikk-store` | pass (`workspace_internal_dependency_passes_anywhere`) |
| Third-party under `[build-dependencies]` | fail, `prikk-object:sha2` (`disallowed_third_party_in_build_dependencies_fails`) |
| Third-party under `[target.'cfg(unix)'.dependencies]` | fail, `prikk-store:sha2` (`disallowed_third_party_under_target_dependencies_fails`) |
| `getrandom = { package = "proptest" }` in `prikk-store` | fail, `prikk-store:getrandom` (`renamed_dependency_under_allowlisted_key_fails`) |
| Missing manifest file | fail closed, `prikk-error: manifest unreadable` (`unreadable_manifest_fails_closed`) |
| Unparseable manifest content | fail closed, `prikk-error: manifest unparseable` (`unparseable_manifest_fails_closed`, additional coverage beyond the ten required) |

`cargo test -p prikk-release-policy --locked boundary::placement`: 11 passed, 0 failed.

## Real-tree confirmation

```
$ cargo run --locked -p prikk-release-policy -- boundary-check --format json
{"schema_version":"release-policy-boundary-v1","valid":true,"errors":[]}
```

## No product manifest changed

`git status --short -- crates/ Cargo.toml Cargo.lock release/ release-signers.toml` is empty. This
increment touches only `tools/release-policy/src`.

## Frozen identities unchanged

- `Cargo.lock`: `601d0678b8481a750519e64bb19f66f8532301b4157d8353d8d9211261c5da31`, 180 packages —
  identical; no new dependency (`toml` was already a tool dependency).
- All seven product manifests, `release-signers.toml`, both command inventories, oracle manifest:
  untouched (confirmed via the empty `git status --short` above).

## Test counts

`prikk-release-policy`: 46 → **57** (+11: `boundary::placement::tests`).
Workspace total: 543 (unchanged outside `prikk-release-policy`, which is not part of the
`cargo test --workspace` unified count shown by other crates — see gate output below for the
per-crate breakdown).

## Gate output

All green, both toolchains:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo clippy -p prikk-release-policy --all-targets --all-features --locked -- -D warnings`
- `cargo test --workspace --locked` — 543 passed (product crates) + 57 passed (`prikk-release-policy`)
- `cargo +1.85.0 test --workspace --locked` — same counts, both toolchains
- `git diff --check`
- `cargo audit --no-fetch` — 180 crate dependencies scanned, 0 advisories
- `cargo run --locked -p prikk-release-policy -- check` — all 154 oracle cases passed
- `cargo run --locked -p prikk-release-policy -- boundary-check --format json` — `valid: true`
- `cargo run --locked -p prikk-release-policy -- reference-check --format json` — `valid: true`

## Naming and category ordering (D3)

Module is `boundary/placement.rs` (not `boundary/dependencies.rs`), category is
`dependency-placement`, distinct from the pre-existing `check_dependencies` /
`dependency-boundary` tool↔product edge check — the collision the design's self-critique (D3) flagged
and the author re-examination's F2 confirmed was fully resolved in the current handoff text.
