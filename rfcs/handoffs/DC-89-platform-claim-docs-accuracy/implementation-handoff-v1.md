# DC-89 Platform Claim Documentation Accuracy — Handoff v1

**Cleared to implement.** Accepted 2026-08-10, `rfcs/done/DC-89-PLATFORM-CLAIM-DOCS-ACCURACY.md`.
No prerequisite questions — §3 is the work.

## 1. What this is

Since DC-81 merged on 2026-08-09, the reference documentation has told readers that repository mutation
is Linux-only. It is not. Two distinct false claims, in eight places across seven pages, listed in the
RFC's §1.

**Most of the incorrect text is mine.** `architecture.md` in particular is a page I wrote. Correct it
without deference — if a sentence reads badly once it is true, rewrite the sentence.

## 2. The one instruction that matters

**Criterion 3: derive the set of affected sites yourself. Do not work from §1's list.**

§1 is what a single grep found. Treating it as complete would repeat the exact failure that produced
this increment — I checked one file, said "nothing else needs to move," and the claim turned out to be
in eight more places. **If you find a site beyond §1's list, that is a result worth reporting
explicitly**, not a silent addition to the diff.

Search for the claim, not for the strings I happened to quote. The two families read quite differently:
"mutation requires Linux" and "exercised by project gates on Linux only" share no common phrase.

## 3. The two that need judgment rather than substitution

**`architecture.md:106`** — "**Mutation is Linux-only** — 93 `target_os = "linux"` gates across
`prikk-store`'s anchored filesystem module." The count has drifted (95 now), but do not just update the
number. After DC-82 the gates are `any(linux, macos)` at the module level with per-platform arms
beneath, so counting `target_os = "linux"` occurrences **no longer measures what the sentence uses it to
prove.** Replace the metric or drop it. Criterion 2 asks for a statement that stays true as platforms
are added — a hardcoded count is the opposite of that, which is my error to have written twice over.

**The "exercised by project gates on Linux only" family** is an *evidence* claim, not a capability
claim, and it needs its own correct answer rather than the capability fix pasted over it. What is
actually true: the `macOS mutation test suite` job (`.github/workflows/ci.yml`) runs
`cargo test --workspace --locked` on `macos-latest` on every push. Check the workflow yourself and state
what it really covers — I confirmed that job exists and runs the full suite, but I have not audited
whether every claim's surrounding context is satisfied by it.

## 4. Limits

- **Documentation only.** No behaviour change, no source change.
- **`platform-support.md` is out of scope** — already corrected under DC-87's review condition.
- **Nothing may imply Windows mutation exists.** It does not, and DC-87 Stage 2 is blocked on DC-88.
  Criterion 5 is there because an over-eager correction is as wrong as the stale claim.
- **`ci.yml`'s own comments are not user-facing documentation.** If they are stale too — I have not
  checked — report it rather than fixing it here.

## 5. Gates

`mdbook build docs`, `git diff --check`, and all three release-policy checks. Compile gates are not
required for a docs-only change, but **`reference-check` is** — it reads documentation references, and
a link corrected carelessly will fail it.

Green CI before merge as always; this one touches no filesystem-backed state, so the three-platform rule
does not bind it — the ordinary CI run is enough.

## 6. Sequencing

- **DC-88** is live and is the priority. This is small; fit it around that.
- Touches only `docs/`, so it will not collide with DC-88's work.
- **DC-87's mode fix** (`1e10a09`) is accepted and waiting on a three-platform CI run. Not yours to
  progress.
- **DC-87 Stage 1's seam refactor** stays on hold behind DC-88.
