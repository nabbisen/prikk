# RFC 108 increment 3a — widen `Wal`/`ActiveLock` names, derive the relative path

**Authority:** RFC 108 §D3, ACCEPTED 2026-08-27. **Base:** `dc14843` or later `main`.
**Under `003-landing-work-on-main.md`** — commit locally on `main`, do not push, do not tag.

**This is a prerequisite, not the increment you were expecting.** §1 says why increment 3 is split.

---

## 1. Why this comes before doctor goes plural

Increment 3 was scoped as `doctor`'s repair path across actives, plus `active.rs`'s ref-name
metadata, plus the deferred `wal.rs:124` cleanup. **Measuring it turned up a blocker that makes the
"cleanup" a prerequisite rather than a nicety:**

```
layout.active_session_names()  -> Result<Vec<OsString>>     (increment 2)
Wal::for_layout(layout, name: &str)                          (increment 1, unchanged)
ActiveLock::acquire(layout, name: &str)                      (increment 1, unchanged)
```

**A plural `doctor` must feed enumerated names into the WAL and the lock, and it cannot.** The only
ways across that boundary today are a `to_string_lossy()` — **the exact defect increment 2 removed,
which silently drops a lock** — or widening these two signatures.

**So the deferred cleanup is now load-bearing.** `wal.rs:124`'s
`PathBuf::from(format!("active/{name}/queue.wal"))` cannot accept a name that is not valid UTF-8 at
all; it is not merely inconsistent with `lock.rs:24`'s derivation, it is a wall.

**Doing this first keeps increment 3b's control clean.** 3b changes mutation behaviour and carries
§D3.3's crash-safety requirement; it should not also be carrying a signature change whose own control
is "nothing changed."

## 2. The change

**Widen `Wal::for_layout` and `ActiveLock::acquire` to accept the same kind of name
`active_session_dir`/`active_queue_wal_path`/`active_lock_path` already accept**, so an `OsString`
from `active_session_names()` reaches both without a lossy step. `&str: AsRef<Path>`, so
`DEFAULT_ACTIVE_NAME` and every current caller should need no edit — **establish that rather than
assuming it; increment 2's equivalent claim held, but check it here.**

**Replace `wal.rs:124`'s reconstructed relative path with a derivation.** `lock.rs:24` shows the
shape: `layout.repository_relative(&path)?`.

### 2.1 The complication you must adjudicate, because it is not free

**`ActiveLock::acquire` returns `Result` and can use `?`. `Wal::for_layout` is infallible —
`#[must_use] -> Self` — and `repository_relative` returns `Result`.** Making `for_layout` fallible
ripples to roughly fifty-eight call sites across forty-six files.

Three shapes, and **the choice is yours with reasoning on the record**:

- **Make `for_layout` fallible.** Honest, and the ripple is mechanical — but it changes a
  widely-used constructor's contract for a path that cannot realistically fail.
- **Compute `relative` lazily**, where a `Result` is already in hand.
- **Give the layout an infallible relative-path builder** for active sessions — `Path::new("active")
  .join(name).join("queue.wal")`, byte-exact, no `format!`, no UTF-8 round-trip, and the layout stays
  the authority for both halves of the path.

**The criterion is which one leaves the layout as the single authority over both `path` and
`relative` without making a constructor lie about what can fail.** I lean toward the third and am not
ruling it.

## 3. What must not change

- **On-disk layout.** `default` remains the only name any caller passes. `init` unchanged.
- **No consumer goes plural.** `doctor`, `active.rs`, and `verify` keep reaching one name. **That is
  3b.**
- **No test assertion changes its expected value.** Same rule as increments 1 and 2 — call sites may
  change, expectations may not. **If one must, behaviour moved: stop and report.**
- **`unlock`'s enumeration is already correct** and must stay byte-exact.

## 4. Controls

1. **A non-UTF-8 name reaches `Wal` and `ActiveLock`.** This is the point of the increment: construct
   both against a `bad\xFFname` session and prove the paths are byte-exact, not mangled. **Gate it
   `#[cfg(target_os = "linux")]`** — increment 2 turned `main` red by using `#[cfg(unix)]` here, and
   APFS returns `EILSEQ` for such a name. **Do not repeat that.**
2. **The derivation is load-bearing.** Corrupt only the new relative-path derivation and show tests
   failing. The prior probe on the old `format!` failed ~20 tests across active/doctor/bundle/refs/
   fsutil; **something comparable should still fire.**
3. **Zero call-site edits, or an exact count with the reason.** §2's claim, established.
4. **Full gate set against the exact final commit.**
5. **Per-job cross-platform CI.** Name it as unavailable locally rather than claiming it.

## 5. A finding to carry into 3b, recorded here so it is not lost

**Increment 2 silently changed when `doctor --repair` refuses, and neither the report nor my review
caught it.** Measured at `dc14843`:

```
victim=refs/locks  healthy=false  repair -> Err: "doctor repair refused because repository
                                                  verification has errors"
victim=cache       healthy=false  repair -> Err: (same)
victim=none        healthy=true   repair -> Ok(repaired)
```

`repair_repository` refuses when `doctor_repository` is unhealthy, and increment 2's new
required-directory check is what now makes those repositories unhealthy. **Before it, a missing
`refs/locks` left the repository "healthy" and repair proceeded.**

**The new behaviour is right** — repairing a WAL inside a structurally damaged repository is exactly
when to stop. **The message is wrong:** it says *"repository verification has errors"*, but the error
came from a doctor-level check that is not part of `verify_repository` at all. A reader will go
looking at `verify` output and find nothing.

**Do not fix it here.** 3a's control is "nothing changed," and this is user-facing text. **It is 3b's,
and 3b's handoff will require it.**

## 6. The report

To `.git-exclude/review-request/`. Include §2.1's adjudication with its reasoning, every control
quoted, the zero-or-N call-site count, the full gate set against the exact final commit, and
**anything in this handoff that was wrong** — including my fifty-eight/forty-six figures, which I took
from increment 1's records rather than recounting.
