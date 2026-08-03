# DC-70 Prerequisite Questions — §2/§3 Answered Before Any Workflow Was Written

Per the handoff's §5 definition of done: *"§2's four questions answered and reported before a
workflow is written."* This document answers all four, with citations, and states plainly where
one answer surfaced a blocker outside this increment's authority to resolve alone (§1 below), and
where another surfaced a real, previously-unverified defect (§3 below).

## 1. Does the release-evidence model extend to N artifacts? — blocked, escalated separately

**Short answer: not without touching a subsystem this increment does not have standing authority
to change.** Full reasoning, blast radius, and the request for a ruling are in
`.git-exclude/review-request/prikk-dc-70-design-question-v1.md` (not committed, per this project's
convention for design escalations). Summary:

`release/schemas/release-evidence-v1.schema.json`'s `archive` property (lines 139-178) is a single
object, `additionalProperties: false`, with no array wrapper — confirmed the only consumers
(`tools/release-policy/src/policy/evidence.rs`'s `tag_or_artifact_invalid` and `complete_valid`,
and the Python ground truth `release/policy_check/evidence.py`) read it via a single
`.get("archive")`, never iteration. Extending it to describe N binaries is a real schema change,
exactly as the handoff warned.

But `release/oracle/` and `release/policy_check/` are not ordinary source — DC-45 (accepted, the
governing RFC for this whole subsystem) states explicitly: *"All five remain tracked through Rust
implementation review, cutover, and the first Rust-gated 0.19.0 release; changing their frozen
behavior requires its own review"* (`rfcs/accepted/DC-45-RELEASE-POLICY-TOOLING-CONSOLIDATION.md:433-434`),
and *"The complete corpus remains intact through cutover"* (line 285). The repository is at 0.18.1,
pre-cutover. Extending the schema would cascade into 73 `release-evidence` oracle cases built by
mutation off ten base fixtures, the reason-map and coverage-inventory exactness checks, and five
duplicated `python_baseline_commit`/`profile_contract_commit` identifiers that would no longer
honestly describe the frozen corpus — not a workflow-adjacent edit, a change to reviewed, frozen
differential-testing behavior.

**Not a fix to route around quietly.** This is escalated, not implemented, in the linked document,
which also proposes closing DC-70 partial (deliver everything else; carry criterion 3) as the
default recommendation, matching this project's DC-56 precedent for a criterion a design finding
puts out of this increment's reach.

## 2. Does `release-policy` validate the archive fields, and how?

Yes — confirmed for completeness even though the extension itself is deferred, since this answers
what changing the schema would additionally require. `tools/release-policy/src/policy/evidence.rs`:
`tag_or_artifact_invalid` (lines 100-138) checks the archive's `name`/`checksum_name` against the
expected single-tarball filename for the version; `complete_valid` (lines 214-268) checks
`archive_attached`, `checksum_attached`, `checksum_grammar`, `archive_root` before a release can be
`complete`. Both are hardcoded to one archive object via `serde_json::Value::get("archive")`. Any
future extension (§1) needs matching Rust logic here, not just a JSON-Schema change, mirroring how
`crates` (already an array) is validated separately in the same file.

## 3. Which targets, and does the code actually build on them? — verified by building, not assumed

**Linux only, for this increment — and the reason is not solely DC-37's mutation boundary.**

Trial builds run from a clean `cargo build -p prikk --release --target <triple> --locked`:

- **`x86_64-unknown-linux-gnu`** — compiles and links successfully. (This is also every existing
  gate's host target.)
- **`aarch64-unknown-linux-gnu`** — every crate compiles successfully; the final link step fails
  locally only because this machine has no `aarch64-linux-gnu-gcc` cross-linker installed (`rust-lld:
  error: ... is incompatible with elf64-x86-64`, a toolchain-availability error, not a code error).
  GitHub Actions offers native `ubuntu-24.04-arm` runners, which sidesteps cross-linking entirely —
  recommended over cross-compiling from an x86_64 host.
- **`x86_64-pc-windows-gnu`** — **fails to compile**, not just to link:
  ```
  error[E0432]: unresolved imports `super::regular::open_existing_regular_if_exists`,
  `super::regular::open_new_regular`
   --> crates/prikk-store/src/fsutil/anchored/immutable.rs:11:5
  note: found an item that was configured out
   --> crates/prikk-store/src/fsutil/anchored/regular.rs:62:15
     | #[cfg(target_os = "linux")]
  ```
  Three more of the same shape follow (`io_error` from `anchored.rs:266`, `prepare_directory_required`
  from `directory.rs:226`). `crates/prikk-store/src/fsutil/anchored/immutable.rs`, `regular.rs`, and
  `read.rs` carry **no** `#[cfg(target_os = "linux")]` gate at the file or item level, yet
  unconditionally import helpers that are themselves gated to Linux — so the crate does not compile
  at all off Linux, let alone fail closed at runtime the way `ensure_root`'s
  `unsupported_mutation()` (`anchored.rs:275-279`) is designed to.

**This contradicts DC-37's own stated intent** — `rfcs/accepted/DC-37-REQUIRED-FILESYSTEM-DURABILITY.md`'s
mutation support matrix frames macOS/Windows as *"read-only/diagnostic use only"*, which presumes the
binary builds and runs read-only commands there. It does not, today. **This is a pre-existing defect
in DC-37's implementation, not a DC-70 design decision** — recorded as a new, unowned finding in
`MILESTONES.md`'s finding-ownership table, reported per the handoff's own standing request ("run what
you publish... stop and report"), not fixed here. Fixing it would mean auditing and re-gating three
fsutil files for correct non-Linux stub behavior, which is DC-37/fsutil territory, not a release-CI
increment.

**Decision:** DC-70 builds and publishes `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`
binaries only. Both are mutation-capable per DC-37, so no artifact needs a mutation-limit statement
under criterion 6 — that criterion is satisfied vacuously for this target set, not skipped.

## 4. What does `cargo binstall` actually require?

`crates/prikk-cli/Cargo.toml` (full file, 22 lines) has no `[package.metadata.binstall]` section.
Package name is `"prikk"` (line 2); there is no `[[bin]]` section, so the binary name defaults to
the package name, `prikk`. cargo-binstall resolves GitHub release assets either by guessing a
default naming pattern or, more reliably, via an explicit `[package.metadata.binstall]` block with
`pkg-url`/`pkg-fmt`/`bin-dir` templates (`{ name }`, `{ version }`, `{ target }`,
`{ archive-format }` interpolation). Added in this increment, matched exactly to the asset names the
new release workflow produces, and demonstrated end to end against real published assets — not
merely configured (criterion 4).

## Also — the standing bug report (handoff §6), confirmed, not fixed

`prikk init <path>` fails at its first command when `<path>` does not exist, exactly as reported:
`crates/prikk-cli/src/main.rs`'s `run_init` calls `RepositoryLayout::init(root.clone())`
(`crates/prikk-store/src/layout.rs:63-67`), whose first substantive line is
`MutationRoot::open(&root)?` — `open`, not a creating call. On Linux this reaches
`AnchoredDirectory::open` (`crates/prikk-store/src/fsutil/anchored/directory.rs:117-126`), which
opens the path `O_DIRECTORY | O_NOFOLLOW` with no `O_CREAT`; a nonexistent path returns `ENOENT`,
mapped to an error and propagated straight back to the CLI. `.prikk/` itself would be created later
by `ensure_root`, but that point is never reached. Recorded in `MILESTONES.md`.

## Also — the exact release-authority wording to reuse verbatim

`CHANGELOG.md:68-80` (0.18.0's entry; 0.18.1's entry points back to it as its packaging-only
successor) states the signer-authority gap under the heading *"Release authority — read before
relying on this release."* Every download surface this increment adds (README install section,
release-page body, `cargo binstall` success path) states the same position in the same terms —
binaries carry no more signer authority than the source tarball already carries none of.
