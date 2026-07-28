# DC-41 Stage 3 - Hash Differential (Implementation Handoff)

**Authority.** Elaborates the accepted RFC's stage-3 acceptance bar
(`rfcs/accepted/DC-41-INTEGRITY-EVIDENCE-CAMPAIGN.md`, "Stage 3 - Hash differential") and its
"Dependency and lockfile plan". It adds no requirement beyond that bar; items marked *recommended* are
design proposals the implementer may decline with a recorded reason.
**Authored by** the architect in the function-designer role. Stage-3 implementation review remains
independent, because implementation is authored by a different developer.
**Predecessor.** Stage 2 committed as `d5bd096`.
**Scope.** Stage 3 only. No CI change, no production code, no `proptest` (stage 4).

---

## 1. Correction to the RFC's premise — stage 3 is far lower-risk than assumed

Both the RFC and my own stage-2 handoff framed stage 3 as *"the first dependency change, isolated so the
`Cargo.lock` transition is reviewable on its own."* **That premise is factually wrong, and the correction
matters for how this stage is built and reviewed.**

`sha2` is **already a production transitive dependency** of this workspace. `ed25519-dalek` depends on it
(for Ed25519's internal SHA-512), and prikk's lockfile already pins **`sha2 0.10.9`**.

I verified the consequences with a reversible probe against the real repository (`cargo add sha2@0.10
--dev -p prikk-hash`, then restored; `Cargo.lock` back to `0cd51cbd…`):

| Expected effect | Measured |
|---|---|
| New packages entering the graph | **0** — locked package count 169 → **169** |
| `Cargo.lock` change | **+3 lines only**: `dependencies = [ "sha2" ]` added to the `prikk-hash` entry |
| `crates/prikk-hash/Cargo.toml` change | +4 lines: a `[dev-dependencies]` block with `sha2 = "0.10"` |
| Advisory-surface growth | **none** — `cargo audit` already scans `sha2 0.10.9` |
| Version resolution | `sha2 = "0.10"` resolves to the already-locked `0.10.9`; no new version enters |

So stage 3 is **not** a dependency-tree expansion. It is a three-line dependency-*edge* addition to an
already-present crate. Keep the isolation and the reviewed re-freeze — the discipline is still correct —
but do not expend effort managing a risk that does not exist, and do not let a reviewer believe they are
verifying a new-crate introduction.

## 2. MSRV is pre-verified — do not rediscover it

The RFC requires MSRV compatibility be confirmed *before* the dependency is proposed. I did this in an
isolated scratch crate outside the repository:

| Version | `cargo +1.85.0 build` | `cargo +1.85.0 test` |
|---|---|---|
| `sha2 0.10.9` (resolved from `"0.10"`) | **PASS** | **PASS** |
| `sha2 0.11.0` | PASS | not tested |

**Use `sha2 = "0.10"`.** Reasons: it is the mature, widely deployed RustCrypto line (the RFC asks for a
*widely audited* reference); and it is already the locked version, so it adds nothing to the graph. The
0.11 line would introduce new packages (`const-oid`, `hybrid-array`) for no benefit here.

Still re-run `cargo +1.85.0 test --workspace --locked` yourself on the real candidate — my check was of
`sha2` in isolation, not of the integrated workspace.

## 3. Placement

Direct in the crate, not `[workspace.dependencies]` — matching the existing precedent, where
`tools/release-policy` declares `tempfile = "3"` in its own `[dev-dependencies]`. Workspace-level
declarations are used for shared production dependencies.

```toml
# crates/prikk-hash/Cargo.toml
[dev-dependencies]
sha2 = "0.10"
```

**`[dev-dependencies]` only, never `[dependencies]`.** Per my DC-41 B4 finding, **no mechanical gate
catches misplacement**: the DC-45 package-listing check inspects packaged *file paths*, and
`boundary::check_dependencies` guards only the tool↔product edge over *local* crates. A crates.io crate
misplaced into `[dependencies]` would ship to every consumer of `prikk-hash` undetected. State the
placement explicitly in the evidence note so the reviewer checks the manifest directly.

## 4. Independence of the oracle — state this honestly in the evidence note

Because `sha2` is already in the runtime trust path (via `ed25519-dalek`), the RFC's discipline clause —
*"differential dependencies… must not enter object identity or runtime trust paths"* — deserves an
explicit note rather than a silent pass.

The differential remains sound:

- it compares two genuinely **independent implementations** of SHA-256 — prikk's first-party code and
  RustCrypto's — and their co-presence in one dependency graph does not correlate their correctness;
- `ed25519-dalek` uses `sha2`'s **SHA-512**, a different algorithm from the SHA-256 under test;
- adding a dev-dependency edge does not make the oracle a production dependency of `prikk-hash`, which is
  what the clause exists to prevent.

Say this in the evidence note. A reviewer who discovers `sha2` in the production graph unaided will
reasonably question whether the clause was honoured.

## 5. Randomized input generation — recommended design

You need reproducible randomness. **Do not add `rand`**: it is absent from the lock and would introduce
new packages (`rand`, `rand_chacha`, and friends), which is exactly the growth stage 3 otherwise avoids.
`rand_core 0.6.4` is present but provides traits, not a seedable generator.

*Recommended:* a SplitMix64 generator written inline in the test module — about ten lines, no dependency,
reproducible by construction, so "fixed seeds" is true by design rather than by discipline.

```rust
struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}
```

**Self-check reference values.** Seeded with `0x243F_6A88_85A3_08D3` (the leading fractional bits of pi —
a nothing-up-my-sleeve constant), the first six outputs must be:

```
[0] 0x2cb0f69f4abea221    [3] 0xdbafb150deb12800
[1] 0x9417034723148989    [4] 0x7e789b2e6c442cb6
[2] 0xdd555950609dfe03    [5] 0xf41e5636c7e4f8c4
```

Assert these in a small unit test. If your generator disagrees, the differential's "10,000 cases" would be
10,000 cases of something other than what you documented.

## 6. Input length distribution — state it, do not just code it

The bar requires a **stated** distribution including empty, sub-block, exact-block-boundary, and
multi-block-spanning lengths. *Recommended* shape, weighted toward the boundaries where SHA-256 padding
logic actually fails:

| Band | Lengths | Share |
|---|---|---|
| Empty | 0 | ≥1 case (guaranteed, not probabilistic) |
| Sub-block | 1-54 | ~25% |
| First-boundary neighbourhood | 55-57, 63-65 | ~25% |
| Multi-block | 66-1024 | ~25% |
| Later-boundary neighbourhood | 119-121, 127-129, 183-185 | ~25% |

Guarantee length 0 explicitly rather than hoping a uniform draw produces it. Record the distribution in
the evidence note, not only in code — a reviewer must be able to judge coverage without reading the
generator.

**Budget:** ≥10,000 cases per CI run. SHA-256 over inputs of this size is microseconds, so this should add
negligible CI time — **measure it and report the figure**, so stage 4's much larger budgets can be planned
against a real number.

## 7. Lockfile re-freeze is a deliverable, not a side effect

Even though the change is three lines:

- record the **new** `Cargo.lock` SHA-256 in the evidence note, and state plainly that it **supersedes**
  `0cd51cbdc98210bc745dd6a7190fbcde30b35dfea4d1cd66b7f0b8459527c616` as the baseline identity that
  subsequent reviews verify;
- report the locked-package count before/after (expect **169 → 169**) — a change here means something
  unexpected resolved, and should stop the candidate;
- confirm `cargo audit --no-fetch` exits zero;
- re-confirm the seven product package listings still exclude test-only tooling under the DC-45 boundary
  check.

## 8. A mismatch is not a bug to fix

If any case disagrees, that is **immediate stop-work with architect/security escalation**. It would mean
ObjectIds, state roots, ref-name paths, and signature preimages computed from such inputs are
non-standard — repository-format-invalidating, not an ordinary defect. Do **not** patch `sha256`, narrow
the distribution, adjust the seed, or encode the observed value.

Calibration: `prikk-hash` currently has cross-implementation agreement on 22 distinct inputs (11
state-root vectors from DC-40, 11 hash vectors from stage 2). A mismatch is unlikely — stage 3 exists
because "unlikely" is not "verified," and the escalation path must be real rather than ceremonial.

## 9. Definition of done

- `sha2 = "0.10"` in `crates/prikk-hash/Cargo.toml` `[dev-dependencies]` only.
- Differential test in `crates/prikk-hash/src/tests.rs` (or a submodule of it), ≥10,000 cases per run,
  fixed seed(s), stated distribution, zero mismatches.
- PRNG self-check test present if the inline-generator recommendation is taken.
- `Cargo.lock`: +3 lines, package count 169 → 169, **new hash recorded** as the superseding baseline.
- `prikk-hash` test count reported before/after (**11** at stage-2 baseline); `prikk-store` unchanged at
  **531**.
- Frozen identities otherwise unchanged: `Cargo.toml`, all other package manifests, both command
  inventories, oracle manifest, `release-signers.toml`.
- CI runtime delta for the differential measured and reported.
- Gates green: `cargo fmt --all -- --check`;
  `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
  `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`; `git diff --check`;
  `cargo audit --no-fetch`; release-policy `check`, `boundary-check`, `reference-check`.

## 10. Submit for implementation review with

- The diff, with the manifest change clearly visible.
- An evidence note stating: the sha2-already-present situation (§1) and the independence reasoning (§4);
  the seed(s) and distribution (§5, §6); case count and measured runtime; before/after `Cargo.lock` hash
  and package count; before/after test counts.
- Gate output per §9.
- An explicit statement that `sha2` is in `[dev-dependencies]` and not in `[dependencies]`.

---

**Boundaries.** Stage 3 grants no authority to add `proptest`, edit CI, bundle stages, add the platform
matrix, move DC-41 to `done/`, or take any release-lane action. The release lane is **parked**; nothing
here activates it, and architect recommendations are explicitly non-authoritative for activation. DC-39
and DC-40 remain unshipped M1 increments, and the 0.17.7 no-go for production, repository-format
stabilization, and public preview stands.
