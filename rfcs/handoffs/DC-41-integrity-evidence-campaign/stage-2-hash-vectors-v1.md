# DC-41 Stage 2 - Hash Vectors (Implementation Handoff)

**Authority.** Elaborates the accepted RFC's stage-2 acceptance bar
(`rfcs/accepted/DC-41-INTEGRITY-EVIDENCE-CAMPAIGN.md`, "Stage 2 - Hash vectors"). It adds no requirement
beyond that bar; everything marked *recommended* is a design proposal the implementer may decline with a
recorded reason.
**Authored by** the architect in the function-designer role. Stage-2 implementation review remains
independent, because implementation is authored by a different developer.
**Predecessor.** Stage 1 committed as `fb4153c`
(`rfcs/handoffs/DC-41-integrity-evidence-campaign/crash-matrix-coverage-v1.md`).
**Scope.** Stage 2 only. No dependency, no CI change, no production code. `sha2` arrives in stage 3.

---

## 1. Why this stage exists

`prikk-hash` has exactly **2** tests (empty, `"abc"`) for the primitive underpinning every ObjectId, state
root, ref-name path, and signature preimage. Neither exercises a padding-block transition, which is where
a SHA-256 implementation actually goes wrong. Stage 2 closes that with fixed vectors; stage 3 adds the
systematic randomized differential.

## 2. Step 1 — extract tests before adding any

`crates/prikk-hash/src/lib.rs` is 166 lines; the inline `#[cfg(test)] mod tests { … }` occupies
**`:147-166`**, against the project rule requiring separate test modules.

Do this as a **pure move, in its own commit or clearly separable hunk**, so the reviewer can confirm
no behaviour change before reading the additions:

1. Replace `:147-166` in `lib.rs` with:
   ```rust
   #[cfg(test)]
   mod tests;
   ```
2. Create `crates/prikk-hash/src/tests.rs` containing the two existing tests verbatim, including
   `use super::{sha256, to_hex};` — `super` still resolves to the crate root, so the import is unchanged.
3. Confirm `cargo test -p prikk-hash` still reports **2 passed** before adding anything.

Then add the new vectors to `src/tests.rs`.

## 3. Step 2 — the vectors

**Provenance is the point of this stage.** Expected values must never be produced by `prikk-hash` itself;
a self-generated expectation is tautological and would defeat the stage entirely.

### 3.1 Canonical published vectors (source of truth: FIPS 180-2 / RFC 6234)

| Len | Input | Expected SHA-256 |
|---:|---|---|
| 0 | `""` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 3 | `"abc"` | `ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad` |
| **56** | `"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"` | `248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1` |
| 112 | `"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu"` | `cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1` |

The 56-byte entry is the highest-value vector available: it is **exactly** the first padding-block
transition (448 bits) *and* it has a canonical published value, so it needs no independent computation.
Rows 0 and 3 are the two already present — keep them.

### 3.2 Boundary vectors (independently computed with Python `hashlib`, **not** `prikk-hash`)

Input is `b"a"` repeated *n* times.

| n | Why this length | Expected SHA-256 |
|---:|---|---|
| 55 | last length whose padding still fits in block 1 | `9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318` |
| 56 | first length forcing a second block (**mandated**) | `b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a` |
| 63 | one byte below exact block size | `7d3e74a05d7db15bce4ad9ec0658ea98e3f06eeecf16b4c6fff2da457ddc2f34` |
| 64 | exact block size; padding wholly in block 2 | `ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb` |
| 65 | first genuine multi-block (**satisfies the >64 requirement**) | `635361c48bb9eab14198e76ea8ab7f1a41685d6ad62aa9146d301d4f17eb0ae0` |
| 119 | *recommended* — 55 + 64: same transition, one block later | `31eba51c313a5c08226adf18d4a359cfdfd8d2e816b13f4af952f7ea6584dcfb` |
| 120 | *recommended* — 56 + 64: second-block padding spill | `2f3d335432c70b580af0e8e1b3674a7c020d683aa5f73aaaedfdc55af904c21c` |

**Why 119/120 are worth adding beyond the mandated set.** 55/56 exercise the padding transition at the
*first* block boundary. 119/120 exercise the identical transition one block later, which is a different
path through the length-accumulator and block-loop logic. An off-by-one in multi-block padding can pass
55/56 and fail 119/120. Cheap to include; decline with a recorded reason if you disagree.

### 3.3 Method validation (how the computed values were checked)

The `hashlib` method was validated against all four canonical published vectors in §3.1 before being used
to compute §3.2 — all four matched. So the independent tool is demonstrably correct on the published set,
which is what licenses its use for the lengths that have no famous published string.

## 4. Pre-verification signal (de-risking, not a substitute for the tests)

I ran all 11 vectors above against the current `prikk-hash` through a throwaway integration test, then
removed it: **all 11 agree**. Two consequences:

- Stage 2 should be a clean pass; if any vector fails, suspect the test wiring before the implementation.
- Combined with DC-40's 11 state-root vectors reconstructed under `hashlib`, `prikk-hash` now has
  independent cross-implementation agreement on 22 distinct inputs spanning both padding boundaries. That
  is meaningful prior evidence for stage 3, and it is **not** a substitute for stage 3's randomized
  differential — 22 fixed inputs cannot cover the input space.

Record this in the stage-2 evidence note as prior signal, not as stage-2 output.

## 5. Traps

- **Do not** generate any expected value with `prikk-hash`, directly or by copying a failure message's
  "actual" into the expectation. This is the one mistake that silently voids the stage.
- **Do not** combine the extraction move and the vector additions into one indivisible hunk — the reviewer
  needs to confirm the move is behaviour-neutral.
- **Do not** touch `Cargo.lock` or add any dependency. `sha2` is stage 3. `Cargo.lock` must remain
  `0cd51cbd…c616`.
- **Do not** edit `.github/workflows/ci.yml`. No CI change in stage 2; the existing jobs already run
  `prikk-hash` tests via `cargo test --workspace --locked`.
- **Do not** change `sha256`, `to_hex`, or any production code. If a vector fails, that is a **stop-work
  hash-mismatch escalation** under the RFC's escalation clause, not a bug to patch inside stage 2.
- Label provenance per vector in the test source (a short comment: published vs independently computed),
  so a later reader can tell which values are externally anchored.

## 6. Definition of done

- `crates/prikk-hash/src/tests.rs` exists; `lib.rs` has no inline `mod tests { … }` body.
- Mandated set present and passing: **55, 56, 63, 64** plus at least one **>64** multi-block, plus the two
  retained existing vectors.
- Recommended additions (65 / 119 / 120 / the published 56-byte and 112-byte strings) either present or
  declined with a recorded reason.
- Per-vector provenance labelled in source.
- `prikk-hash` test count reported before/after (**2** at baseline).
- `prikk-store` count unchanged at **531** (stage 1's committed figure), proving no collateral movement.
- Frozen identities unchanged: `Cargo.toml`, `Cargo.lock`, all package manifests, both command
  inventories, oracle manifest, `release-signers.toml`.
- Gates green: `cargo fmt --all -- --check`;
  `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
  `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`; `git diff --check`;
  release-policy `check`, `boundary-check`, `reference-check`.

## 7. Submit for implementation review with

- The diff, with the extraction move separable from the vector additions.
- A short evidence note: vectors added, provenance per vector, before/after test counts, and any declined
  recommendation with its reason.
- Gate output per §6.
- An explicit statement that no production code, dependency, CI file, or frozen identity changed.

---

**Boundaries.** Stage 2 grants no authority to add dependencies, edit CI, bundle stages, add the platform
matrix, move DC-41 to `done/`, or take any release-lane action. The release lane is **parked**; nothing
here activates it. DC-39 and DC-40 remain unshipped M1 increments, and the 0.17.7 no-go for production,
repository-format stabilization, and public preview stands.
