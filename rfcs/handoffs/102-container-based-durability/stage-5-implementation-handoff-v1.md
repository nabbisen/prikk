# RFC 102, Stage 5 — Implementation Handoff v1

**Authorized by the project owner 2026-08-15.** Design: `design-v1.md` **§14** (scope, derived after
Stage 4 merged), and §2's container rules.
**Stage 6 (compaction) is not authorized.**

Stage 4 merged at `94219cf` with green three-platform CI — all eight jobs, including the macOS mutation
suite.

## 1. Stage 5 is not "trust", and that is the first thing to know

§7 recorded this stage as three words: *"Stage 5 — trust."* Scoping it found the name was wrong.

RFC 102 §9 criterion 2 — **"No durability-bearing write uses `atomic_replace`"** — is a whole-RFC claim.
Seven production `write_file_atomically` calls survive Stage 4. Five are durability-bearing; **trust is
two of them.** Compaction covers none. So `active.rs:122` and `received.rs:107` belonged to no stage at
all, and the staging as written would have ended with the RFC's own criterion 2 unmet.

**Your scope is the remaining durability-bearing replacements.** See §14.1's table for the full
classification, including *why* `commit_index.rs:80` and `lifecycle_cache/incremental.rs:175` are
excluded — both are rebuildable caches, established from their own code rather than their names.

## 2. Two defects found while scoping — both in your scope, both already ruled

Read §14.2 and §14.3 in full. In brief:

**`FORMAT` is written before the containers it certifies** (`layout.rs:138` vs `:146`). An interrupted
`init` leaves a repository reading as valid format-4 with containers absent. I probed it — deleted all 16
container files — and `status`, `verify` (**all 12 stages `evaluated`**) and `doctor` all exit 0. Ruled:
**`FORMAT` must be written last.**

**Every append creates the file first.** `open_append_regular` (`fsutil/anchored/regular.rs:110-118`)
calls `open_new_regular` and falls back only on `EEXIST`. Every container append routes through it. On
Windows this is a new-name event per append — the exact hole this RFC exists to close. Ruled:
**`durable_append` must require an existing file**, matching `durable_truncate` at `anchored/linux.rs:61-64`.

**Neither is a live defect** — Windows mutation does not exist (DC-37). Both are traps the Windows
implementation would inherit by default.

## 3. Step 0 first — report before any production code

§14.4 lists five items. Answer them from the code, not from this document:

1. **Trust's shape.** Maintainer keys are one file per key id — the per-name surface Stage 4 faced with
   refs. Does the ref-container answer transfer, or does trust's read pattern need something else?
2. **Whether `active.rs` and `received.rs` belong here or in a Stage 7.** A reasoned split is acceptable;
   silence is not.
3. **The `FORMAT`-last reordering**, and what re-`init` on a partially initialized repository must do.
4. **The `durable_append` strictness change**, and every caller depending on create-on-append today.
   Enumerate them. Note §14.4 item 4's correction about `wal.rs:174` before you assume it is one.
5. **How the two cache exemptions get asserted** rather than described.

Three stages running have each found something in Step 0 that would have been expensive later.

## 4. What must not change

- **`required = 1`.** A block needs *one* trusted signature, never a threshold
  (`trust.rs:2-4`, DC-78 §D2, DC-11). Containerizing trust must not turn this into a count.
- **Trust already fails closed, hard.** `load_maintainer_trust_policy` reads the policy and every key it
  names through `read_file_required`; one missing key fails the whole load. **Read §14.1.1** — I had this
  backwards at first, and the correction changes what you must prove. Do not write a test that trust loss
  "fails safely"; it already does, loudly. **Prove the state survives.**
- **DC-72's collision and reserved-name rejection for maintainer trust key ids.** A collision here could
  silently change which key a repository trusts.
- **DC-95's classification**, on every path this stage touches.
- **DC-38's invariant** and `ensure_no_incomplete_publication`'s chokepoint, unchanged from Stage 4.
- **No `atomic_replace` on any container path.**

## 5. Acceptance criteria

1. **Every new container name created at `init`** — enumeration, as Stages 3 and 4 proved it.
2. **No durability-bearing write uses `atomic_replace`** — and with this stage, that is true of the
   *repository*, not just of one stage's paths. Criterion 2 closes here or it names what remains and why.
3. **The two cache exemptions asserted by a test**, not by prose.
4. **`FORMAT` written last at `init`**, proven by an interrupted-init state that is *detected* rather than
   silently accepted.
5. **`durable_append` refuses an absent file**, with every legitimate caller migrated to an explicit
   create-at-`init` path.
6. **Trust state survives** what previously lost it — the §14.1.1 framing, not a fail-closed test.
7. **DC-95's classification survives.**
8. Full gate set per `EXECUTION-ORDER.md` §6 rule 9, plus **green three-platform CI**.
9. **`docs/src/reference/` reflects what this stage ships** — the criterion Stage 4 added, carried
   forward. *"No statement in `docs/src/reference/` is false about what this stage ships."*

## 6. Standing

- **Report counts before and after** per rule 10. Its baseline was re-measured 2026-08-15 and pinned to
  `f2edb11`: `prikk-store` 690, `prikk-object` 80, `prikk-replay` 44, `prikk-hash` 14, `prikk-crypto` 7,
  `prikk-release-policy` 83; 179 locked packages. **An increment that changes any of these updates that
  line in the same commit** — the previous baseline was stale in five of seven figures because five
  increments each assumed someone else would.
- **Work on a branch.** Stage 4's core reached `main` unbranched and left the mainline red for a day
  while every local gate passed. Branch → push → green CI → merge.
- A stop-and-report remains a complete outcome.
- Stage 5 merges before Stage 6 is scoped.
