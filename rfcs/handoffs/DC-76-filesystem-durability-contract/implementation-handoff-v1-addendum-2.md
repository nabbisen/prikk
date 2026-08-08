# DC-76 Handoff v1 — Addendum 2: one blocking repair, and a gate gap that is mine

**Date:** 2026-08-09. **Authored by** the architect. **Review:**
`.git-exclude/reviewed/DC-76-implementation-review-v1.md`.

## 1. B1 — blocking: the commit breaks the non-Linux CI job

CI's `non-linux build` job (`.github/workflows/ci.yml:62-63`) runs
`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` **natively on macOS and
Windows.** Under that command cross-targeted to `x86_64-apple-darwin`:

- **`ec1cd63~1` (parent): clean.**
- **`135af27` (yours): two errors** — `function publish_immutable_file is never used`, `trait
  DurabilityContract is never used` — and `prikk-store` fails to compile as both lib and lib-test.

`DurabilityContract` is declared unconditionally in `contract.rs`, while its only implementor is behind
`#[cfg(target_os = "linux")]`. Off Linux the trait, and through the same indirection
`publish_immutable_file`, become unreachable. Under `-D warnings` that is a hard error.

**On the repair — yours to choose, but one direction is worse than it looks.** Gating the trait to Linux
would make it compile, and I think it is the wrong fix: **the point of this increment is a
platform-neutral statement of what the store requires.** A contract that disappears on the platforms it
exists to enable defeats itself. The trait *should* be present on macOS, waiting for an implementor.
Keeping it visible and allowing the dead code off Linux **with the reason stated** is a true statement
about the world rather than a suppression.

**Nothing else is in question.** Repair, then re-review.

## 2. The gap that let this through is mine, not yours

**You ran the canonical gate set correctly and completely.** `EXECUTION-ORDER.md` §6 rule 9 lists exactly
nine gates and **does not include cross-target clippy**. DC-74's eleven were above the rule, not required
by it.

**I have amended rule 9**: any increment touching `#[cfg(target_os)]`-gated code now also runs Windows
and macOS clippy with `-D warnings`. This increment restructures the Linux-gated filesystem module, which
is the highest-risk possible place for platform-conditional dead code — and my definition did not require
the check that catches it.

## 3. What I want on the record, because it is the best evidence discipline this project has seen

**Nine negative controls attempted; one did not fail, and you reported it as a finding.** A control that
fails to fail is the easiest thing in this discipline to drop quietly, and dropping it would have been
invisible to me unless I happened to re-run that exact one.

**I tested G9 against the kernel rather than take it on trust**: passing `0o100644` to `chmod` yields
`0644` with the file still regular. **Your finding is correct**, your correction of the overclaiming doc
comment is right, and keeping the mask for a future platform is the right call.

**G6's invalid first attempt, reported with its root cause and nothing claimed from it**, is the same
discipline. So is correcting two of your own doc comments rather than leaving them flattering.

**I re-ran G1 independently** — stripping `NOFOLLOW` from `directory.rs` makes
`required_directory_rejects_symlink_component` fail. The security-critical guarantee is genuinely pinned.

## 4. Verified and not in question

892 tests zero failures; fmt, Linux clippy, diff-check and all three policy gates pass; **no `target_os`
gate widened** and none introduced as `unix`/`target_family`; no manifest change; **no pre-existing test
body modified.**
