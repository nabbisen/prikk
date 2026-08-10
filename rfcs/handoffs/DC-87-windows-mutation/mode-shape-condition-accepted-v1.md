# DC-87 — Mode-Shape Review Condition Accepted

**Reviewing:** `1e10a09` on `dc-87-windows-mutation`, on top of the reviewed `37334da`.

**Condition discharged. The mode-carrying shape is accepted outright** — the only thing left before
merge is the standing green-CI-on-all-three-platforms rule.

**And my §4 condition was scoped wrong. Same mistake as two increments ago. §2 is that.**

## 1. The correction

`platform-support.md:11-19` now reads correctly: both real implementors named, `MacosDurability`
attributed to DC-81/DC-82, and "no reviewed equivalent" scoped to platforms beyond those two. The
"non-Linux caller receives a clean runtime error" sentence is now "a caller on any other platform,"
which matters — a macOS caller does not reach that path at all.

Carrying the `fcntl_fullfsync` difference into the paragraph, with the ~180x measurement, goes past what
I asked and is the right call: a reader deciding whether to mutate on macOS should meet that fact on the
platform-support page, not only in `FINDINGS.md`.

Gates re-run by me at `1e10a09`: `mdbook build docs` clean, `git diff --check` clean, all three
release-policy checks green including `reference-check`. Docs-only, single file, no Rust source touched
— their decision not to re-run the compile gates is correct here, but `reference-check` reads
documentation references, so I ran it rather than assuming.

## 2. My §4 was scoped to one file, and the claim lives in eight places

I wrote "nothing else on the page needs to move." That was true of that page. **I never asked whether
the same false claim lived on other pages.** It does — eight more occurrences across seven reference
pages, every one of them false since DC-81 merged on 2026-08-09:

| File | What it still says |
|---|---|
| `architecture.md:106` | "**Mutation is Linux-only** — 93 `target_os = "linux"` gates" |
| `architecture.md:132` | table row "Mutation is Linux-only \| Being addressed, contract first" |
| `durability-recovery.md:19` | "Repository mutation currently requires Linux anchored relative no-follow operations" |
| `durability-recovery.md:82`, `:193` | "Linux-only exercised gates" |
| `concurrency-locking.md:28`, `:191`, `path-safety.md:40`, `data-model.md:14`, `trust-threat-model.md:18`, `repository-layout.md:26` | "Repository *mutation* is exercised by project gates on Linux only" |

The second family is a different claim from the first — it is about *test evidence*, not capability —
and it is also false: DC-81 added the `macOS mutation test suite` job, which runs
`cargo test --workspace --locked` on `macos-latest`. I confirmed that against `ci.yml` rather than
inferring it.

**`architecture.md` is mine.** I wrote that page, including the "Linux-only" sentence and the gate count.
The count has drifted too — 95 now against the 93 it claims — but re-counting is the smaller problem:
after DC-82 the gates are `any(linux, macos)` at the module level with per-platform arms beneath, so
counting `target_os = "linux"` occurrences no longer measures what the sentence uses it to prove. That
sentence needs rewriting, not a new number.

**This is the second time in this cycle I have checked the file in front of me and generalized from it.**
The first was DC-85, where I said no existing documentation statement had become false; `merge-plan.md:24`
had, and the dev team found it. I am recording the pattern rather than the individual slip: when an
increment makes a claim stale, the question is never "is this file still accurate," it is "where else is
this claim made." That is a repository-wide grep, and it costs seconds.

## 3. Not this increment's to fix

The rationale for §4's condition was proximity — they were editing that file and writing accurate text
beside a falsehood in it. That rationale does not extend to seven files they never touched, and bundling
a docs sweep into a mode-semantics change would make DC-87's commit unattributable, which is the
argument I have used against bundling four times this cycle. It would be inconsistent to abandon it
because the extra work happens to be mine.

Opened as **DC-89**, proposed alongside this. It is mechanical and small, and it should move quickly:
these are user-facing reference pages telling people mutation does not work on a platform where it does.

## 4. Standing

- **Mode-carrying shape: accepted.** No further conditions.
- **Merges after a green CI run on all three platforms**, per the standing rule.
- Stage 1's seam refactor: unstarted, cleared.
- Stage 2: blocked on DC-88 and on the owner's `unsafe`-surface decision.
