# DC-89 — Review Amendment Accepted

**Reviewing:** `35b965f` on `dc-89-platform-claim-docs-accuracy`, on top of the reviewed `b0a66ea`.

**Accepted. No further conditions.** DC-89 closes on an ordinary CI run and merge.

## 1. Verified

`README.md:62` and `:128` corrected; `:105` and `:138` — the prebuilt-binary claims — untouched, as
required. The DC-71 history sentence ("`prikk-store` previously failed to compile at all off Linux") is
also untouched, which was the other thing criterion 5 could have been tripped by.

I re-ran the sweep across `README.md` and `docs/src` at the branch tip. Two hits, both expected and
neither a defect:

- `durability-recovery.md:19` — the corrected "requires Linux **or macOS**", matching on the word rather
  than the claim.
- `platform-support.md:11` — still pre-fix, because that correction lives on `dc-87-windows-mutation`
  (`1e10a09`) and `dc-89` branches from `main`. They identified this in the first package and it is
  right.

Gates at `35b965f`: `git diff --check` clean; release-policy `check` 154 oracle cases, `boundary-check`
and `reference-check` both `"valid": true`. Skipping `mdbook build` was correct — `README.md` is not in
the mdbook source tree — and running `reference-check` anyway was the right instinct for the same reason
it was last round.

## 2. One consequence of the split worth naming

`platform-support.md`'s correction and DC-89's now live on two different branches, and only DC-89's is
close to merging. That is fine as long as both land: no file is touched by both, so there is no
conflict in either merge order.

**The risk is only if DC-87's mode-shape branch is ever dropped or rebuilt** — the
`platform-support.md` fix would go with it while DC-89 shipped, leaving exactly one uncorrected page
among seven corrected ones. I do not expect that (the branch is accepted and waiting on CI), but it is
the kind of thing that is invisible until it bites, so it is recorded here. If DC-87's branch is ever
abandoned, `platform-support.md:11-19` must be re-fixed.

## 3. An observation, not a condition

`README.md:62` now reads "**mutation is Linux and macOS only, not Windows** (read-only commands run on
macOS and Windows too)." Every word is true. But the parenthetical used to carry a contrast — *mutation
here, read-only there* — and macOS now appears on both sides of it, so the sentence does slightly less
work than it did. The tighter version names only the platform the contrast still applies to.

**Not a condition, deliberately.** It is true as written, it sits in a front-page limits paragraph that
is otherwise accurate, and requiring another round-trip over a parenthetical would be poor
proportionality after three corrections that were genuinely mine. Take it or leave it; if this paragraph
is edited again for another reason, it is worth tightening then.

## 4. Standing

- **DC-89: accepted.** Closes on an ordinary CI run and merge — no filesystem-backed state, so the
  three-platform rule does not bind it.
- `ci.yml`'s two stale comments (`:48`, `:92`) stay queued for DC-87 Stage 2, which must touch that file
  to add a Windows mutation job.
- **DC-87's mode fix** (`1e10a09`): accepted, awaiting a green three-platform CI run.
- **DC-88** is the live increment.
- **DC-87 Stage 1's seam refactor**: on hold behind DC-88. **Stage 2**: blocked on DC-88 and on the
  owner's `unsafe`-surface decision.
