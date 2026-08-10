# DC-87 — Mode-Carrying Shape Implementation Review v1

**Reviewing:** `37334da` on `dc-87-windows-mutation`, based directly on `main`'s tip `cb3d7f3`.
**Scope:** the narrow round's item 2 only. Nothing here touches Stage 2, DC-88, or the `unsafe`-surface
question, and the package correctly says so.

**Verdict: ACCEPT, conditional on one docs fix (§4).** The hazard is closed, and I proved it is closed
by reintroducing it.

## 1. The hazard, verified by negative control

I did not take the four new tests at their word. In a detached worktree at `37334da` I reintroduced
exactly the mistake the ruling was about — replacing `plan_mode_change_if_observed`'s `observed_mode?`
with `observed_mode.unwrap_or(REGULAR_FILE_MODE)`, which is the plausible "simplification" a future
reader might make — and re-ran the suite:

```
unobserved_mode_never_plans_a_change_perm_even_when_baseline_differs ... FAILED
(6 passed, 1 failed)
```

**Exactly one test failed, and it is the one that models the hazard**: an executable baseline against an
unobservable worktree mode. The other three passed, which is what I wanted to see — the suite
discriminates on this specific defect rather than on incidental behaviour.

There is a second, stronger guard the package does not claim credit for: with `WorktreeFileMeta.mode`
now `Option<u32>` and `BaselineFile.mode` still `u32`, the original `if meta.mode != base.mode` **no
longer type-checks**. The hazard is now a compile error in its original form and a test failure in its
plausible rewrite. That is the right pair.

## 2. The consumer sweep, re-derived

I re-ran the search rather than trusting the four-call-site claim. Every `RootFileStat` construction,
every `stat.mode`, and every `meta.mode` in the workspace:

- `commit_index.rs:51` `matches_stat` — compares `size`/`mtime_secs`/`mtime_nanos` only. Confirmed by
  reading it. `Option` flows through with no consequence.
- `worktree.rs:142` — `.and_then(|stat| stat.mode)` makes `current_mode` `None`, the comparison never
  matches, `set_regular_file_mode_required` always runs. `entry.mode` is what reaches disk either way,
  so the change is to whether an optimization fires, not to output.
- `worktree_files.rs:98` — `normalize_file_mode` carrying the `Option` through, `None -> None`.
- `node_authoring.rs` at `405` (compare), `425` (create), `745` (the synthetic stat for `matches_stat`),
  `767` (cache bookkeeping).
- `commit_index/tests.rs:107-121` — three literals updated.

That is the complete set. Nothing was missed.

**The `#[cfg(unix)]` distinction is a good catch of theirs, not mine.** The non-Linux/macOS fallback is
reached by other Unix-family targets too, which *do* have a real mode, so `Some(metadata.mode())` stays
and `None` is reserved for platforms with no POSIX mode at all. The added comment says exactly that.
`None` means "unobservable," never "not computed."

## 3. Gates, re-run by me at `37334da`

fmt clean; clippy `--workspace --all-targets --all-features --locked -D warnings` clean;
`cargo test --workspace --locked` green; `cargo +1.85.0 test --workspace --locked` green;
**606 prikk-store lib tests (602 + 4)**; `git diff --check` clean; `cargo audit --no-fetch` 179
dependencies, nothing flagged; all three release-policy checks; `mdbook build docs` clean (the
`mdbook-mermaid` version warning is pre-existing and unrelated).

**Cross-target clippy for `x86_64-pc-windows-gnu` and `x86_64-apple-darwin`: both clean.** This one was
*required*, not a bonus — the change edits `#[cfg(target_os)]`-gated code in `read.rs`, which is the
exact trigger §6 rule 9's amendment names. It is also the only gate that compiles the `None` arm at all,
so it is load-bearing here in a way it was not for DC-85.

`Cargo.toml` and `Cargo.lock` untouched, confirmed by diff.

## 4. Condition: fix `platform-support.md:11` in this commit

They reported it as a finding and wrote around it. I am converting it to a condition, and the reason is
narrow.

`docs/src/reference/platform-support.md:11` still reads **"Repository *mutation* requires Linux,"** and
the paragraph under it says the primitives "have no reviewed equivalent on other platforms yet." Both
have been false since DC-81 merged on 2026-08-09 — `MacosDurability` is a reviewed equivalent, and
DC-82 made it a peer implementor.

Reporting it was right. Writing the new bullet in deliberately generic language *to avoid touching the
false statement in the same file, in the same commit* is what I am pushing back on. The result is a page
that now tells a reader both things: mutation requires Linux, and here is how mode authoring behaves on
non-Linux platforms that mutate. Adding accurate text beside a false statement is worse than either
fixing it or leaving the file alone.

It is also not an investigation: DC-81 and DC-82 are merged, complete, and recorded. Correct the
sentence and the "no reviewed equivalent" clause to name Linux and macOS. Nothing else on the page needs
to move, and the new bullet's generic framing is good and should stay as written — it will read
correctly once the paragraph above it is true.

## 5. Accepted as reported

- `RootFileStat.mode`/`WorktreeFileMeta.mode` as `Option<u32>`; the `0` sentinel gone. This was the
  ruling's requirement and it is met at the source rather than papered over downstream.
- Extracting `plan_mode_change_if_observed` rather than inlining the branch. It is what made the
  negative control possible and the unit tests meaningful; inlined, neither would exist.
- The doc comment drawing the distinction against `set_permission_bits`'s "returns `Ok` is not evidence"
  standard **in the code**, where the next reader will meet it. That was my §2 wording and it survived
  into the right place.
- New files defaulting to `REGULAR_FILE_MODE`, separated from the existing-file case as a missing
  capability rather than data loss. Correct, and correctly documented.
- `resolve_existing_file`'s `base_mode` parameter so `CommitIndexEntry.mode` records the mode actually
  resolved. Bookkeeping stays coherent; `matches_stat` stays indifferent.
- **Addition 1** — the `commit_index.rs` module doc corrected rather than the code. The replacement goes
  further than I asked by stating *why* the trust condition must stay indifferent to mode's
  `Option`-ness, which is the part that stops a future reader from "fixing" `matches_stat`.
- **Addition 2** — four tests, running on every platform's CI today. The requirement was that this be
  tested rather than testable, and it is.

## 6. Standing

- **Stage 1's seam refactor** (`MutationRoot`'s per-platform authority type) remains unstarted and
  cleared. This package was the mode fix only, and saying so plainly was right.
- **Stage 2** stays blocked on DC-88 and on the owner's `unsafe`-surface decision.
- **Green CI on all three platforms before merge**, per the standing rule — this touches
  filesystem-backed state and platform-gated code.
