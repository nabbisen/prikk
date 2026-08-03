# DC-71 Prerequisite Questions — §3 Answered Before Designing the Fix

Per the RFC's §4 acceptance criterion 1 and the handoff's §5 definition of done: *"§3's three
questions answered and reported before designing."* Question 1 (portable read-only: requirement or
aspiration?) was already ruled by the owner in the accepted RFC itself — *"Yes. Cross platform
support is required."* This document answers the remaining three.

## 1. Which targets must build?

`x86_64-pc-windows-gnu` (cross-compiled locally, via the mingw-w64 toolchain already used for DC-70)
and `x86_64-apple-darwin` (check-only locally, via `rustup target add`; no macOS SDK available on
this machine for linking) — both verified clean with `cargo clippy --workspace --target <triple>
--locked -- -D warnings` after the fix, zero warnings, on top of a clean `cargo check --workspace
--target x86_64-apple-darwin --locked` for the link-unavailable target.

**CI does not use these exact triples.** It builds on GitHub's native `windows-latest` (x86_64) and
`macos-latest` (Apple Silicon / `aarch64-apple-darwin`, GitHub's default since the macOS 14 image —
verified, not assumed, since `macos-latest` silently changed architecture once before) runners
instead of cross-compiling, avoiding the cross-linking friction DC-70 already hit once. The fix
itself is `#[cfg(target_os = "...")]`, not target-triple-specific, so the two triples verified
locally and the two runner architectures CI actually uses are expected to behave identically — but
that expectation is exactly what CI now checks continuously, rather than something asserted once and
trusted. `docs/src/reference/platform-support.md` records this as a real coverage gap (arm64
Windows, x86_64 macOS are neither locally verified nor CI-gated), not a known difference.

## 2. Is `prikk-store` the only affected crate?

Yes — checked directly, not inferred from the one finding already named. `cargo clippy -p
prikk-replay -p prikk-object -p prikk-hash -p prikk-crypto -p prikk-error --target
x86_64-pc-windows-gnu --locked -- -D warnings` passed clean *before* any fix was made to
`prikk-store`, and `cargo clippy --workspace --target x86_64-pc-windows-gnu --locked -- -D
warnings` (the whole workspace, including `prikk-cli` and `tools/release-policy`) passed clean
*after* fixing only `prikk-store`. No other crate touches `target_os`-specific code.

## 3. What does "read-only command" mean concretely?

Traced, not assumed, by following every CLI command's implementation to whichever of
`crates/prikk-store/src/fsutil`'s thirteen mutation functions (`ensure_root`,
`write_file_atomically`, `write_worktree_file_atomically`, `append_file_required`,
`truncate_existing_file_required`, `truncate_file_empty_required`, `create_new_file_required`,
`remove_file_required`/`remove_file_if_present_required`/`remove_worktree_file_required`,
`promote_file_required`, `publish_immutable_file`, `ensure_directory_required`,
`sync_directory_required`) it does or does not reach, **including transitively** — `rollback-draft`
calls none of them directly in its own file, but reaches `append_file_required` through
`Wal::append_patch`. The full, durable table (25 rows: 15 read-only, 10 mutation, one boundary
command with 7 sub-modes split 4 read-only / 3 mutation) is
`docs/src/reference/platform-support.md`, cross-linked from the README and from every reference page
that previously claimed "Linux is the only platform exercised by project gates" (a claim that was
about mutation specifically and needed correcting now that read-only commands are CI-gated
cross-platform too).

One command's classification surfaced a second, unrelated finding: `worktree-status` is genuinely
read-only by the trace, but currently unreachable against any repository built through ordinary
`commit`/`seal` use — it requires snapshot-block state nothing in the CLI produces. Found running
the CI fixture end to end locally before trusting it to real CI, per the handoff's own standing
request ("run what you publish"). Recorded in `MILESTONES.md`, excluded from the CI fixture's
demonstrated command list, not fixed — a `checkout`/`worktree_status` design question, not a
DC-71 non-Linux-conformance question.
