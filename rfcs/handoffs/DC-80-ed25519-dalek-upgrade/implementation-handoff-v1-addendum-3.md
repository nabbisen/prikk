# DC-80 Handoff v1 — Addendum 3: accepted and merged

**Date:** 2026-08-10. **Authored by** the architect. **Merged at `ad09d5d`** after a green macOS run.
**Review:** `.git-exclude/reviewed/DC-80-implementation-review-v1.md`.

## 1. Criterion 4 discharged where it had to be

I said the §1 probe would not discharge it because it was a throwaway harness. **You ran the same cases
through `prikk-crypto`'s real API, against 2.2.0 first and 3.0.0 second, with identical pass/fail shape.**
That is the only way to demonstrate the silent direction, and it is done.

**And criterion 2 you exceeded.** Sealing with the old binary and verifying with the new one was the
ask; you then sealed a **third generation with the new binary** and verified the mixed-version history
together — two blocks under 2.x, one under 3.x, clean, with correct per-block attribution. **A repository
does not have to pick a side of this upgrade, and that is now shown rather than assumed.**

## 2. My six-package figure was wrong. Yours is right.

Addendum-1 claimed the bump collapses six duplicated packages. **Only `const-oid` does**, and I verified
your correction on both sides: pre-upgrade `sha2 0.10.9` has **two** roots, `ed25519-dalek 2.2.0` **and
`prikk-release-policy`**; post-upgrade only the latter, because `tools/release-policy/Cargo.toml:17` pins
`sha2 = "0.10"` itself.

**I assumed the duplication had one cause without checking every root** — avoidable with the single
`cargo tree -i` you ran. That is the fourth figure of mine you have corrected by measuring.

**Declining to fix it was right** — release-policy's own hygiene, not the signature path. Recorded
unowned, with a note that DC-45's frozen-baseline constraint on release-policy expired with 0.19.0, so
whoever picks it up is no longer blocked.

## 3. The `cargo update` wrong turn was worth writing down

Unscoped `cargo update` pruned the target rows **and** dragged in unrelated churn — a second major `syn`
among it — landing at 184, worse than the scoped 179. **Recording a wrong turn with its measurement is
more useful than the right answer alone**, and this one generalises beyond the increment.

## 4. What is left

**DC-85** (merge from a received ref) is cleared for its four questions and is the only live increment.
**Revocation remains the largest unowned design question** — no way to distrust an adopted key, and no
way to distrust blocks it already sealed.
