# RFC 141 increment 1 — the release-evidence producer

**RFC:** `rfcs/accepted/141-publication-through-ci.md` — **accepted in full by the project owner
2026-09-06.** §5's obligations and §4's reading of DC-35's authority composition are settled input.
**Base:** `main` at `2757fe0`.

**This increment touches no registry, no credential, and no workflow.** It produces a document. **§2
is the part to read twice — the split it describes is what makes this testable at all.**

---

## 1. What to build

A `prikk-release-policy` subcommand that emits a **`release-evidence-v1`** document for a release,
conforming to `release/schemas/release-evidence-v1.schema.json` — **the schema that already exists and
that the oracle already carries 73 cases for, and that no release has ever produced.**

**Closing that gap is the whole point.** Nine releases have shipped with `complete`-looking outcomes
and no evidence document at all.

## 2. The split: a pure document builder, and thin observation

**Two of the three SHA-256 values DC-35 requires cannot be obtained offline.** Per crate:

| Value | Where it comes from |
|---|---|
| `staged_sha256` | the local `.crate` file `cargo package` produces — **offline** |
| `registry_checksum` | the sparse index — **network** |
| `fetched_sha256` | bytes downloaded after the crate is registry-visible — **network** |

**So the subcommand is a pure function from *observations* to a *document*, and the observation
gathering is a separate, thin layer.** The builder takes a set of per-crate observations (any of the
three values possibly absent) and produces the document; it never fetches anything itself.

**This is the same shape RFC 139 increment 1 used for its extractor, and for the same reason:** a pure
transform can be tested exhaustively against fixtures, and a function that reaches the network can
only be tested against whatever the network happened to return.

**Do not build the network layer in this increment beyond what you need to exercise it once.** The
document is the deliverable.

## 3. What the document must carry

Top level, all required, `additionalProperties: false` — the schema will reject anything else:
`schema_version`, `sequence`, `version`, `overall_status`, `prior_snapshot`, `tag`, `archive`,
`crates`, `release_page`, `pages`, `governance`, `attempts`.

**Per crate** (`$defs/crate`): `name`, `version`, `exact_internal_requirements`, `publish_level`,
`staged_sha256`, `registry_checksum`, `fetched_sha256`, `checksum_equality`, `published`,
`registry_visible`.

**Three things to get right, because each has a wrong answer that validates:**

1. **The three checksums are `null` when not observed, and `checksum_equality` is then
   `"not-observed"`** — not `"match"`. The enum is `match` / `mismatch` / `not-observed`, and a
   producer that defaults to `match` because nothing contradicted it would emit a document asserting
   an equality nobody checked. **That is worse than no document**, and it is the single most damaging
   thing this increment could ship.
2. **`publish_level` is the topological level, and it must be derived, not hardcoded.**
   `cargo_metadata` is already a dependency of this crate. **A hardcoded list is how `check_members`'
   own allowlist went stale** — RFC 139 increment 1 hit exactly that. Derive it from the workspace
   dependency graph so adding a crate cannot silently produce a wrong level.
3. **`sequence` is a three-digit string starting at `001`**, and `prior_snapshot` is `null` only for
   `001`; otherwise it names the previous file and **its observed SHA-256**. The filename pattern is
   fixed by the schema: `prikk-<x.y.z>-release-evidence-<NNN>.json`.

**`overall_status`** is `pending` / `partial` / `complete` / `superseded`. **A document may only claim
`complete` when every crate has all three checksums observed and equal, is `published`, and is
`registry_visible`.** Derive this; never accept it as an argument.

**`attempts`** is cumulative and append-only: `sequence` (integer from 1), `time` (RFC 3339),
`operation`, `result` (`succeeded`/`failed`/`not-applicable`). **A later snapshot retains every prior
attempt in order** — including failed ones. Dropping a failed attempt to make a document look clean
defeats the artifact.

## 4. Existing material to use rather than reinvent

- **`release/fixtures/release-evidence-complete.json`** — a real fixture in this shape. Note it
  carries **7 crates** (0.18.0 era) and placeholder all-zero checksums; today's workspace has **8**.
  Use it to understand the shape, not as a template to copy.
- **The oracle already validates these documents** (`tools/release-policy/src/policy/evidence.rs`,
  reached from `policy.rs`). **Your output must pass it.** Wire that as a test rather than checking by
  eye.
- **`jsonschema` and `serde_json` are already dependencies** of this crate. No new third-party surface
  is needed, and none should be added.

## 5. Out of scope

- **Publishing anything.** No `cargo publish`, no credential, no registry write.
- **The workflow and the Trusted Publishing binding.** RFC 141 increments 3 and 4, and increment 3
  additionally waits on an owner action.
- **Retrofitting evidence for the nine past releases.** RFC 141 §6 leaves this open deliberately;
  **do not invent evidence for a release you did not observe.**
- **The `partial`-state incident procedure.** DC-35 specifies it; this increment records the state,
  it does not implement the response.
- **Any change to `release-signers.toml`.** It must not be touched — it sits on a different authority
  leg (RFC 141 §4) and is owner-blocked.

## 6. Controls

1. **A `pending` document with nothing observed validates**, carries three `null` checksums per crate
   and `"not-observed"`, and **does not claim `complete`.**
2. **A fully-observed, all-equal document validates and claims `complete`.**
3. **One mismatched checksum forces `partial`, not `complete`** — and the affected crate reads
   `"mismatch"`. Assert both, since a producer could get the crate row right and the overall status
   wrong.
4. **`publish_level` is derived correctly for today's eight crates**, and a test proves it comes from
   the graph — add a synthetic member or reorder, and confirm the level follows. **A test that
   asserts today's eight literal levels would pass against a hardcoded list**, which is exactly what
   must not ship.
5. **Sequence and predecessor linkage:** `001` has `prior_snapshot: null`; `002` names `001` and its
   real SHA-256. **Perturb the predecessor hash and confirm rejection.**
6. **Attempts are append-only:** building `002` from a state carrying `001`'s attempts retains all of
   them, in order. Drop one and confirm the test fails.
7. **The oracle accepts your output.** Feed a produced document through the same validation path the
   73 existing cases use.

**Each control must be seen to fail before it passes.** Report the perturbation per control, as the
last two rounds did.

## 7. Gates

The full set, verbatim from `rfcs/EXECUTION-ORDER.md` §6 rule 9:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test --workspace --locked`
- `cargo +1.85.0 test --workspace --locked`
- `cargo +1.85.0 check --workspace --all-targets --locked`
- `git diff --check`
- `cargo audit --no-fetch`
- `RUSTDOCFLAGS="-D rustdoc::private_intra_doc_links" cargo doc --workspace --no-deps`
- release-policy `check`, `boundary-check`, `reference-check`

**A new subcommand changes this tool's own command surface.** `release/release-policy-command-inventory-v1.json`
exists and `check`'s oracle cases cover this tool — **run `check` early**, and if the inventory needs
the new subcommand, that is part of the work, not a surprise at the end.

**Cross-target clippy only if your own diff introduces `#[cfg(target_os)]`/`#[cfg(unix)]`/
`#[cfg(windows)]`.**

## 8. No `CHANGELOG.md` entry

`prikk-release-policy` is `publish = false` and ships to nobody. **Ruled here rather than left
unsaid**, per the standing rule that every handoff either names the entry or rules it out.

## 9. Reporting

`.git-exclude/review-request/`. Include:

- **the per-control perturbations**, as in the last two rounds;
- **how `publish_level` is derived**, and what you did to prove it is not effectively hardcoded;
- whether the command inventory needed updating;
- **anything in the schema you could not satisfy honestly.** The schema was written in 2026-07 against
  a 7-crate workspace and has never been exercised by a real producer — **if a required field has no
  truthful value available at production time, say so rather than inventing one.** That finding would
  be worth more than a clean report.
