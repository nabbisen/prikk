# DC-41 Stage 4 - Property/Fuzz Evidence

**Scope.** Stage 4 only — the final DC-41 stage. No CI job added; no production code changed.
**Predecessor.** Stage 3 committed as `540d4db`; `Cargo.lock` baseline was
`18a8b40aa83396974c2cacd9af56e7496d9f645cd07bda0e4164e4d0b68f0d53`.

## Dependency: `proptest`

**Workspace-dependencies convention applied from the start this time.** `proptest = "1"` was added
to root `Cargo.toml`'s `[workspace.dependencies]` (alongside the existing `# Testing` `sha2`
entry), and both `crates/prikk-object/Cargo.toml` and `crates/prikk-store/Cargo.toml` reference it
as `proptest = { workspace = true }` in `[dev-dependencies]`. (Mid-session correction: my first
attempt used plain `cargo add proptest@1 --dev -p prikk-object`, which by default writes a literal
version into the member crate's manifest — exactly the technical-debt pattern already corrected
once for `sha2` in stage 3. Caught before landing; reverted; redone the right way.)

**Growth measured, not assumed** (reversible `cargo add` probe first, then the real build):
locked package count **169 → 180 (+11)**: `fnv`, `ppv-lite86`, `proptest 1.11.0`, `quick-error`,
`rand 0.9.5`, `rand_chacha 0.9.0`, `rand_core 0.9.5`, `rand_xorshift 0.4.0`, `rusty-fork`,
`unarray`, `wait-timeout`. (The handoff predicted `rand 0.9.4`; the registry had moved to `0.9.5`
by the time this ran — resolved version differs from the handoff's prediction, not from what was
actually verified here.) New `Cargo.lock` hash:
**`601d0678b8481a750519e64bb19f66f8532301b4157d8353d8d9211261c5da31`**, superseding
`18a8b40a…0d53`.

**`rand_core` duplicate confirmed:** `0.6.4` (existing, via `prikk-crypto`) and `0.9.5` (new, via
`rand`) coexist — different semver majors, so both persist as the handoff predicted.

**`cargo tree -d` change reported as expected, not a regression:** was clean before this stage;
now reports `getrandom v0.2.17`, `v0.3.4`, `v0.4.3` as three separate subtrees. All three versions
were already present in the lockfile (`0.2.17` via `prikk-crypto`/`prikk-store`, `0.3.4` via the
new `rand_core 0.9.5`, `0.4.3` via `tempfile 3.27.0` under `proptest`'s dev-only subtree); adding
`proptest` makes them mutually reachable in one tree view, which is what triggers `cargo tree -d`
to report them. This is a consequence of dev-only test tooling, not product dependency drift.
(`rand_core`'s two versions are not flagged by `cargo tree -d` itself — only `getrandom` is; that
tool-specific detail is reported as observed, not explained further here.)

**MSRV re-verified on the real integrated workspace**, not just an isolated scratch crate:
`cargo +1.85.0 build --workspace --locked` and `cargo +1.85.0 test -p prikk-object --locked` both
passed before proptest was used in any actual test.

**Placement statement:** `proptest` is in `[dev-dependencies]` only, in both crates, referenced via
`{ workspace = true }`. It does not appear in `[dependencies]` anywhere. Per the stage-1 B4
finding, no mechanical gate catches misplacement — verified by direct manifest inspection.

**Advisory surface:** `cargo audit --no-fetch` — 180 dependencies scanned, 0 advisories, clean.

## Target coverage and two corrections to the handoff's inventory

Five target families implemented, matching the RFC's closed target list. Two corrections were made
to the handoff's own inventory (§3), both caught by direct code verification before writing any
generator, not accepted on the handoff's authority:

**Correction 1 (target 2's decoder count).** The handoff cites `payload/patch.rs:130` as one of
"five" payload decoders. That function, `PatchPurpose::decode_from_patch_payload`, reads only the
tag-5 purpose field — it is not a full structural `PatchPayload` decoder and cannot round-trip the
way `Block`/`RefState`/`RefUpdate`/`Blob`'s `decode_canonical` can. The genuine full Patch-content
decoder is `prikk-store::patch_replay::decode::decode_patch_operations`, which internally calls
`decode_from_patch_payload` as one validation step among several — that is target 5's subject.
Target 2 therefore covers exactly **four** full round-trip payload decoders, not five; no coverage
is lost, it is attributed to the correct target (documented in both test files' module docs).

**Correction 2 (target 3's "replay/lifecycle-cache reconstruction" bullet).** The original RFC
bullet ("WAL record framing and ref-log entry framing… replay/lifecycle-cache reconstruction from
WAL") could be read as requiring a fourth decoder beyond WAL record framing:
`lifecycle_cache.rs`'s `DecodedLifecycleCache::decode`. Checked directly: that entire module
(magic, decode function, and supporting types) is `#[cfg(test)]`-gated, and its own doc comment
states "replay reconstruction/compare are later slices" — it is test-only scaffolding not yet
wired into production `Wal::replay()`. `Wal::replay()` **is** exactly `decode_records` over the
WAL file's bytes, so target 3's round-trip already covers "replay… reconstruction from WAL" as it
actually behaves today. Recorded in target 3's module doc rather than silently included or
silently dropped.

| # | Target | Location | Properties |
|---|---|---|---|
| 1 | Envelope framing, all 10 `ObjectType` codes | `prikk-store::file_codec::tests` | round-trip; totality; DC-40 format-2 schema-allowlist admission consistency |
| 2 | Payload decoders (4: `Block`, `RefState`, `RefUpdate`, `Blob`) | `prikk-object::payload::tests::proptest_decoders` | round-trip; totality |
| 3 | WAL record framing / replay reconstruction | `prikk-store::wal::tests::proptest_framing` | round-trip incl. trailing-partial handling; totality |
| 4 | Ref-log entry framing | `prikk-store::refs::log::tests` | round-trip incl. trailing-partial handling; totality |
| 5 | Patch operation decoding, all 7 FDD-03 §9.3 kinds | `prikk-store::patch_replay::tests::proptest_round_trip` | round-trip with bounded generation; totality |

**Generation-bounds vs. production-thresholds distinction preserved in code comments** per the
RFC: op count (1–5), path depth (1–3 segments), path segment length (1–8 chars), content size
(0–32 chars in target 5's ASCII text fields) are test-tractability limits, unrelated to
`NFR-PERF-02`'s 800/1000 active-block thresholds.

## Finding: encode/decode path-safety asymmetry (not fixed here — production change, out of scope)

The campaign run (`PROPTEST_CASES=100000`) on target 5 found a real defect on the first campaign
run, exactly the scenario the RFC's discipline clause anticipated ("Stage 4 targets canonical
decoders under arbitrary input, which is where something will plausibly be found").

**Minimized reproducer** (proptest's shrinker, committed at
`crates/prikk-store/proptest-regressions/patch_replay/tests/proptest_round_trip.txt`):

```
operations = [Operation { op_seq: 1, op_id: None, preconditions: [], kind: RenamePath(RenamePath {
  node_id: NodeId([1; 32]), old_path: "a", new_path: "com1" }) }]
```

**What happens:** `RenamePath::validate()` (and every other path-carrying operation's `validate()`:
`CreateFile`, `DeleteNode`, `CreateSymlink`) checks only `node_id` non-zero — none of them validate
path safety. So a `RenamePath` with `new_path: "com1"` **encodes successfully** via
`to_canonical_bytes()`. But `decode_rename_path` calls `RepoPath::parse(&new_path)`, which rejects
Windows-reserved device names (`con`, `prn`, `aux`, `nul`, `com1`-`com9`, `lpt1`-`lpt9`) per
`is_windows_reserved_name` in `prikk-replay/src/path.rs`. Decode fails with
`InvalidName("Windows reserved path component is not allowed: com1")`.

**Classification:** this is a genuine encode/decode asymmetry in the operation wire codec, not a
decoder crash — `decode_patch_operations` still returns a clean `Err`, never panics, so NFR-SEC-04
("never panic or corrupt state") holds. It affects every path-carrying operation kind
(`CreateFile.path`, `DeleteNode.path`, `RenamePath.old_path`/`new_path`, `CreateSymlink.path`) —
the fields built through `repo_path_strategy()` — because they all share the same encode-side gap.
`DeleteNode`'s symlink `old_target` and `CreateSymlink`'s `target` are not affected: decode reads
those as plain strings without a `RepoPath::parse` call.

**Practical severity note:** `RenamePath` specifically is not yet reachable through any real
authoring path today (`ensure_apply_supported` returns `UnsupportedObjectType` for it — application
is deferred to a later node-model increment), so this exact reproducer cannot occur via ordinary
use yet. `CreateFile` **is** wired into authoring today, so the same class of gap is live for it now
if a caller ever constructed one with an unsafe path bypassing worktree-layer validation.

**Disposition — per the RFC's escalation-free discovery clause ("a discovered behavior defect opens
a dedicated corrective RFC… not a bug to patch inside this stage"):**

- **Not fixed here.** Fixing it means adding `RepoPath`-equivalent validation to the four
  operations' write-side `validate()` methods — a production behavior change, outside DC-41's
  "no production behavior change" scope.
- **Not silently normalized.** The property test still asserts the real invariant
  (`decode(encode(x)) == x`); it was not weakened to accept the asymmetry as correct.
- **Not silently avoided.** `path_segment_strategy()`'s exclusion of the eleven reserved base names
  is fully disclosed in a doc comment at the exclusion site (quoting the exact reproducer, the exact
  error, and the reason), stating explicitly that the filter exists *because* the case was found,
  not to avoid finding it. Every other campaign run (all four other targets, and target 5's own
  totality test) used **no** filtering.
- **Reproducer committed** at the path above, so it is not lost if the filter is ever removed.
- **Needs its own tracked RFC.** This is a new finding, distinct from the five already tracked as
  DC-49 through DC-53. Recommend the project owner assign it a number and scope in the same pattern
  (mirroring how DC-51 tracks the stage-1 B4 dependency-placement gate) — not done here, since
  assigning RFC scope/numbers is the architect/maintainer's call, not an implementation-stage
  decision.

Re-ran the campaign (100,000 cases) after adding the disclosed exclusion: **zero further
findings**, target 5 clean. All four other targets' campaign runs (100,000 cases each) were clean
on the first run, no filtering applied to any of them.

## Budgets and measured runtimes

Fast budget (proptest's own default, 256 cases/target, no explicit `ProptestConfig` override
anywhere — verified this is genuinely the library default, not assumed):

| Target | Cases | Wall time (debug) |
|---|---:|---:|
| 1 — envelope framing | 256 × 3 tests | 0.02s |
| 2 — payload decoders | 256 × 8 tests | 0.02s |
| 3 — WAL framing | 256 × 2 tests | 0.03s |
| 4 — ref-log framing | 256 × 2 tests | 0.03s |
| 5 — patch operations | 256 × 2 tests | 0.05s |

Negligible CI cost, consistent with stage 3's ~15µs/case SHA-256 baseline; decoder targets are
slower per case (structured generation, TLV parsing) but still add well under a second total.

Campaign budget (`PROPTEST_CASES=100000`, `--release`), run once each and results recorded:

| Target | Wall time (release) | Result |
|---|---:|---|
| 1 — envelope framing | 0.37s | zero mismatches |
| 2 — payload decoders | 0.61s | zero mismatches |
| 3 — WAL framing | 0.61s | zero mismatches |
| 4 — ref-log framing | 1.16s | zero mismatches |
| 5 — patch operations | 1.61s (after fix) | zero mismatches; 1 finding on the unfiltered first run, disclosed above |

## Test counts

- `prikk-object`: **64 → 72** (+8, target 2).
- `prikk-store`: **540** unchanged in the always-passing baseline count (target 1 +3, target 3 +2,
  target 4 +2, target 5 +2 = +9 over stage 3's 531 baseline; matches: 531 + 9 = 540).
- `prikk-hash`: **13**, unchanged (not touched this stage).

## Corpus footprint

**One file**, at proptest's default per-test-module location:
`crates/prikk-store/proptest-regressions/patch_replay/tests/proptest_round_trip.txt`. No other
target produced a regression file (zero findings on their campaign runs). This is within the "one
packed file per crate" policy intent — there is currently only one finding to pack.

## CI wiring

No new CI job added — the fast budget runs automatically inside the existing `stable` and
`msrv-1.85.0` jobs' `cargo test --workspace --locked` step, exactly as the RFC requires. The
campaign budget is not wired into CI (matches the RFC: "not gating ordinary CI") and was run
manually for this evidence note.

## Frozen identities

| Identity | Status |
|---|---|
| `Cargo.toml` (workspace root) | **changed as intended** — `proptest` added to `[workspace.dependencies]` |
| `Cargo.lock` | **changed as intended** — new hash `601d0678…5da31`, package count 169 → 180 |
| `crates/prikk-object/Cargo.toml`, `crates/prikk-store/Cargo.toml` | **changed as intended** — `[dev-dependencies] proptest = { workspace = true }` |
| All other package manifests | unchanged |
| Command inventories (both) | unchanged |
| Oracle manifest | unchanged |
| `release-signers.toml` | unchanged; signer set still empty and fail-closed |

## Gate output

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | clean (after adding `#![allow(clippy::expect_used)]` to all 5 new test files, matching existing precedent in `prikk-crypto/src/tests.rs` and `prikk-object/src/payload/tests.rs`) |
| `cargo test --workspace --locked` | `prikk-object` 72, `prikk-store` 540, all else unchanged, no failures |
| `cargo +1.85.0 test --workspace --locked` | same counts, no failures |
| `git diff --check` | clean |
| `cargo audit --no-fetch` | 180 dependencies scanned, 0 advisories |
| release-policy `check` | all 154 oracle cases passed |
| `boundary-check` / `reference-check` | `valid: true` |

No production code (`sha256`/`to_hex`/any codec's actual encode or decode logic) was changed. No
CI file was touched. The one disclosed finding above is explained, not hidden, per the RFC's "zero
**unexplained** findings" bar — not "zero findings."
