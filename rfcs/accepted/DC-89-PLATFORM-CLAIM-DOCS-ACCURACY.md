# RFC (accepted) - DC-89 Platform Claim Documentation Accuracy

**Status.** **ACCEPTED by the project owner 2026-08-10.** Small and mechanical; cleared to implement
directly — §3's scope is the work, there are no prerequisite questions.
**Independence.** Author-reviewed — the standing ceiling. **Note that most of the incorrect text is the
architect's own**, which is a reason for care, not for skipping the increment.
**Arises from.** The DC-87 mode-shape review, 2026-08-10. The architect's accept condition named one
file; the same claim turned out to live in eight places across seven more.
**Target.** 0.20.0.

## 1. The problem

DC-81 landed `MacosDurability` on 2026-08-09 and DC-82 made it a peer implementor. Since then the
reference documentation has told readers, in two different forms, that repository mutation is Linux-only.
Both forms are false:

**Capability claims** — mutation *requires* Linux:

- `architecture.md:106` — "**Mutation is Linux-only** — 93 `target_os = "linux"` gates across
  `prikk-store`'s anchored filesystem module."
- `architecture.md:132` — table row, "Mutation is Linux-only | Being addressed, contract first".
- `durability-recovery.md:19` — "Repository mutation currently requires Linux anchored relative
  no-follow operations…".

**Evidence claims** — mutation is only *exercised* on Linux:

- `durability-recovery.md:82`, `:193`; `concurrency-locking.md:28`, `:191`; `path-safety.md:40`;
  `data-model.md:14`; `trust-threat-model.md:18`; `repository-layout.md:26` — variations on
  "Repository *mutation* is exercised by project gates on Linux only."

The second family is also false, and separately so: DC-81 added the `macOS mutation test suite` job
(`.github/workflows/ci.yml`), which runs `cargo test --workspace --locked` on `macos-latest` on every
push. Confirmed against the workflow, not inferred.

`platform-support.md` was corrected under DC-87's review condition and is **not** in scope here.

## 2. Why this is not DC-87's

DC-87's condition applied because that increment was editing `platform-support.md` and writing accurate
text beside a falsehood in the same file. That rationale does not reach seven files it never touched,
and folding a documentation sweep into a mode-semantics change would make DC-87's commit
unattributable — the argument this project has used to split DC-82 from DC-81, DC-86 from DC-78, and
§3.6's read-path finding out of DC-87 itself.

## 3. Scope

1. Correct every claim in §1 to name Linux **and** macOS, matching the shape
   `platform-support.md:11-19` now uses.
2. **`architecture.md:106` needs rewriting, not re-counting.** The count has drifted (95 against the 93
   stated), but the deeper problem is that after DC-82 the gates are `any(linux, macos)` at the module
   level with per-platform arms beneath, so counting `target_os = "linux"` occurrences no longer
   measures what the sentence uses it to prove. Replace the metric or drop it; do not just update the
   number.
3. **Grep for the claim, do not work from §1's list.** §1 is what one sweep found. Treat it as a
   starting point and confirm the set independently — that is the exact failure that produced this
   increment.

## 4. Acceptance criteria

1. No page in `docs/src` states or implies that mutation requires Linux, or that mutation is exercised
   only on Linux.
2. `architecture.md:106`'s claim is restated in terms that remain true as platforms are added, or
   removed.
3. The set of corrected sites is **independently derived**, and any site found beyond §1's list is
   reported as such.
4. `mdbook build docs` clean; `git diff --check` clean; all three release-policy checks green.
   Compile gates are not required for a docs-only change, but **`reference-check` is** — it reads
   documentation references.
5. **No claim is corrected past what is true.** Windows mutation does not exist; nothing here may imply
   it does, and DC-87's own status must not be pre-announced by a documentation edit.

## 5. Non-goals

- `platform-support.md` — already corrected under DC-87.
- Any change to what the software does. Documentation only.
- Any statement about Windows mutation, which remains unimplemented and blocked on DC-88.
- `.github/workflows/ci.yml`'s own comments, which are not user-facing documentation. If they are stale
  too, report it rather than fixing it here.
