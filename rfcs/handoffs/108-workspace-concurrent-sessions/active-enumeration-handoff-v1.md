# RFC 108 increment 2 — enumerate active sessions, and make the read-only recovery surface plural

**Authority:** RFC 108 §D2/§D3, ACCEPTED 2026-08-27. Increment 1 landed at `a54a560` (accepted, CI
green on all 15 jobs).
**Base:** `a54a560` or later `main`. **Under `003-landing-work-on-main.md`** — commit locally on
`main`, do not push, do not tag.

---

## 1. Why this increment exists, and why it comes before anything user-facing

Increment 1 made `.prikk/active/<name>/` *representable*. Nothing creates a second name, and that is
still true after this increment.

**But representable is enough to expose a latent contract violation, and I found it while reviewing
increment 1.** `unlock::list_held_locks` documents itself as enumerating:

> *"every lock file currently present: the active-session lock, every per-ref lock, and every one of
> the four container locks."*

Two of those three are genuinely exhaustive — per-ref locks by directory listing, container locks via
`LockableContainer::ALL`. **The active-session lock is singular and hardcoded** (`unlock.rs:144`,
`layout.default_active_lock_path()`).

Today that is correct, because one active exists. **The moment anything creates a second one,
`unlock` under-reports a held lock and its own doc comment becomes false — on a recovery surface,
where under-reporting means a wedged workspace cannot be found or cleared.**

**The principle for this increment: make the readers plural before the writers.** If the increment
that finally creates a second active is also the increment that teaches the safety surfaces to see
it, then a recovery blind spot ships and is discovered by someone whose repository is already wedged.
Doing it in this order means the blind spot never exists.

**Nothing here becomes reachable by a user.** This is a correctness change to code that is not yet
exercised, deliberately landed early.

## 2. The change

**One enumeration primitive, and exactly one consumer.**

### 2.1 The primitive

A function that returns the active-session names present on disk. **Three things you must adjudicate
and justify in the report — I am not ruling them:**

1. **Where it lives.** `layout.rs` owns `active_dir()` and `active_session_dir(name)`, and
   enumeration is the inverse of construction — but `active.rs` is the module *about* active
   sessions. **The criterion is the same one that sited RFC 120's gate: which module already owns
   this responsibility, rather than which is convenient.** Note that `layout.rs` does not currently
   import `list_directory`; `unlock.rs` does. That is evidence, not a decision.
2. **Its return shape**, and in particular **ordering**. See §2.3 — this one has a portability
   consequence and I want your reasoning on the record.
3. **Its behaviour when `active/` is absent.** `required_directories` guarantees the directory on a
   valid repository — but **`unlock` is a recovery surface, run precisely when a repository is not
   valid.** A primitive that errors here would make `prikk unlock` fail on a damaged repository,
   which is the one repository it exists for. **Establish what `list_directory` actually does with a
   missing directory rather than assuming**, and state the answer.

### 2.2 The consumer

`unlock::list_held_locks` replaces its single hardcoded active-lock read with an iteration over the
enumerated names. **`unlock.rs:148-158` already contains the pattern to mirror** — the per-ref lock
listing does `list_directory` + an `EntryKind` filter + `read_lock_if_present`. Actives are
directories, so the filter is `EntryKind::Directory` rather than `Regular`.

**Follow that neighbour, do not invent a second technique.** Increment 1 landed with `lock.rs`
deriving a relative path and `wal.rs` rebuilding one by string formatting; two techniques for one job
in a single commit is exactly what I do not want repeated.

### 2.3 Ordering is a real portability hazard here, and this diff will contain no `#[cfg(target_os)]`

**Directory listing order is not guaranteed and is not the same across filesystems or platforms.**
Nothing in `unlock.rs` or `fsutil/anchored/read.rs` sorts anything today.

**This project's standing correction applies exactly here: the absence of `#[cfg(target_os)]` in a
diff does not mean the diff is portable.** A plural `list_held_locks` whose output order follows
readdir will produce different output on Linux, macOS and Windows for the same repository — and
`print_locks` output is what an operator copies a path out of.

**Decide and justify.** If you sort, say what the key is. If you conclude the existing per-ref
listing has the same exposure, **report it as a finding — do not fix it in this increment**; changing
the order of existing output is a behaviour change and §3 forbids one.

### 2.4 `HeldLock.kind` does not distinguish two actives

`HeldLock.kind` is the `kind=` field from the lock body — `"active"` for every active lock, whichever
workspace holds it. With two actives held, `list_held_locks` returns two entries both saying
`"active"`, distinguished only by `path`.

**`print_locks` already emits paths, so this is very likely adequate — but confirm it reads sensibly
rather than assuming.** If it does not, **report it; do not redesign the output here.** Presentation
is a user-facing decision and §4 puts those out of scope.

## 3. What must not change

- **On-disk layout.** `init` still creates `active/default/` and nothing else. `required_directories`
  untouched.
- **`list_held_locks`'s output on a repository with one active must be byte-identical to `a54a560`'s**
  — same entries, same order, same paths. This is the increment's central control.
- **No mutation surface.** `clear_lock`, `find_held_lock`, and every write path are untouched.
- **No test assertion changes its expected value.** Same rule as increment 1: test *call sites* may
  change, test *assertions* should not. **If an existing assertion must change, behaviour moved and
  this increment's premise is broken — stop and report rather than editing the expectation.**

## 4. Out of scope — and each of these is a named later increment, not an oversight

- **`doctor`'s repair path** (`doctor.rs:405/414`) — acquires the active lock and replays the WAL for
  one hardcoded name. **This is increment 3**, because making repair plural means acquiring N locks
  and replaying N WALs: a mutation-path change carrying RFC 108 §D3.3's crash-safety requirement
  (*"a Workspace's WAL recovering independently of every other"*). It needs its own controls and must
  not ride along on a read-only increment.
- **`verify`'s explicit out-of-scope line** (§D3.4) — deferred until after increment 3. It adds a
  user-visible report line and a `--format json` field, and reporting *"N workspaces, not verified
  here"* is not worth shipping while N cannot be anything but 1.
- **`active.rs`'s ref-name metadata** (`active.rs:105/137/156`) — per-active state reached by one
  hardcoded name. Mutation-adjacent; belongs with increment 3.
- **The `wal.rs:124` / `lock.rs:24` relative-path duplication** I recorded reviewing increment 1.
  **Deliberately not bundled here** — increment 2 does not otherwise touch `wal.rs`, and mixing a
  refactor into an increment whose control is *byte-identical output* weakens the control. **Increment
  3 touches that line anyway; it goes there.**
- **Any CLI surface, any way to create a second active, any `Workspace` naming.** Unchanged from
  increment 1.

## 5. Controls

**Two of these exist because of what reviewing increment 1 turned up. Read §5.2 before starting.**

1. **One active on disk → identical output.** `list_held_locks` on a single-active repository returns
   what `a54a560` returned. Establish this against the real prior behaviour, not by inspection.
2. **A second active, planted by hand → both reported.** Nothing in the codebase creates one, so
   construct `active/<second>/` and its lock directly in the test. **Both locks must appear**, and the
   test must fail if the enumeration is reverted to the hardcoded read.
3. **A missing `active/` directory → `unlock` still works.** §2.1(3). The degenerate case, on a
   recovery surface. **A gate that reports success over an unreadable directory is worse than no
   gate** — establish which of "empty result" or "error" is correct and defend it.
4. **The layout pin that does not currently exist.** My increment-1 review found that **exactly two
   tests in the whole store suite pin the on-disk active-session name** — `active/tests.rs:346`'s
   literal and one `unlock` symlink test. That is thin for a directory this arc will make
   user-visible. **Add the pin: a test that fails if `active/<name>/` changes shape**, for a
   non-default name. Do not assume one exists.
5. **Full gate set against the exact final commit**, per EXECUTION-ORDER §6 rule 9.
6. **Per-job cross-platform CI.** §2.3 is the reason. **This touches a directory listing on a
   recovery surface; local green is not evidence.** Name it as unavailable locally rather than
   claiming it — increment 1's report did this correctly.

### 5.2 Do not reuse increment 1's control 1 reasoning

Increment 1's report offered *"the G1 compatibility fixture passes unchanged"* as proof that **"the
on-disk layout genuinely did not move."**

**I re-ran that gate with `DEFAULT_ACTIVE_NAME` mutated to `"probe"`. It passed 5/5.** The G1 gate
pins objects, refs and schema coverage; **it is blind to the active-session directory entirely.** The
conclusion was true, but that control could not have detected its failure.

**Do not carry "G1 passes" forward as evidence about active-session layout in this increment or any
later one.** Control 4 above exists to create the pin that G1 cannot provide.

## 6. The report

To `.git-exclude/review-request/`, and include:

1. **§2.1's three adjudications, with the reasoning** — siting, return shape, missing-directory
   behaviour.
2. **§2.3's ordering decision**, and whether the existing per-ref listing shares the exposure.
3. **§2.4's finding** — whether two `"active"` entries read sensibly in `print_locks`.
4. **Every test assertion you changed, or that none were** (§3).
5. **All six controls, quoted** — including control 1's evidence that output is genuinely unchanged
   and control 2's evidence that the test fails when reverted.
6. **The full gate set against the exact final commit**, after the last edit.
7. **Anything in this handoff that was wrong.** My increment-1 blast-radius table was wrong — you
   recounted and corrected it, which was worth more than the increment itself. **Do that again.** In
   particular I have counted, but not verified line by line, that the hardcoded single-active reaches
   are: `unlock.rs` 1, `doctor.rs` 3, `active.rs` 7, `layout.rs` 8. **Re-derive them.**

**Stop and report rather than proceeding if:** an existing assertion needs a new expected value; the
missing-directory question has no safe answer without changing a mutation path; or making the active
enumeration plural forces a change to `print_locks`' format.
