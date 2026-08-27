# RFC 108 increment 1 — generalise `active/<name>/`

**Base:** current `main` (`c2a965e`). **Under `003-landing-work-on-main.md`.**
**RFC 108 is ACCEPTED** on the design at D1–D5. **This is D5's recommended first increment.**

**No CLI surface. No Workspace concept exposed. No behaviour change.** When this lands, prikk does
exactly what it does today, by the same paths, with the name `default` supplied rather than assumed.

---

## 1. What this is, and a correction to my own framing

D5 called this "a mechanical change." **That understates it, and I measured after writing it:**

| symbol | production call sites | test call sites |
|---|---|---|
| `default_active_dir` | 6 | 4 |
| `default_queue_wal_path` | **2** | **43** |
| `default_active_lock_path` | 2 | 6 |

**Plus roughly fourteen hardcoded `"active/default"` string literals**, including
`wal.rs:121`'s `PathBuf::from("active/default/queue.wal")`.

**The production surface is genuinely small — ten call sites across three symbols.** The test surface
is not, and **that is the real work**: 43 of the 45 `default_queue_wal_path` uses are tests asserting
against a path that is about to stop being the only one.

## 2. The change

**`.prikk/active/<name>/` is already the shape.** `active_dir()` is the parent; `default` is one child.
**Nothing in the layout needs inventing** — only the choice of name needs to stop being hardcoded.

- **`Wal::for_layout`** hardcodes both the path and the relative literal. It should take the active
  name; **the layout stays the authority on paths** (D3.1), so the name is a parameter, not new
  path-building logic inside `Wal`.
- **`ActiveLock::acquire`** hardcodes the default lock path the same way. **It generalises with the
  same change** — `lock.rs:67` already shows a second, ref-scoped constructor, so more than one
  granularity is an accepted idea here.
- **Keep `default` as the only name in use.** Every existing caller passes it. **Do not add a way to
  create a second active** — that is a later increment and a user-facing decision.

**Adjudicate the shape**: a named constant, a newtype, or a plain `&str` parameter. **Say which and
why.** A newtype that makes an invalid active name unrepresentable is worth considering, but **do not
build validation this increment does not need.**

## 3. What must not change

- **The on-disk layout.** `.prikk/active/default/` must be byte-identical afterwards. **Every existing
  repository must open, verify, and mutate unchanged** — the G1 fixture is the control.
- **`required_directories()`'s output**, which is what makes `refs/tmp`-style absences detectable.
- **Behaviour, anywhere.** If any test's *expectation* has to change, that is a signal you changed
  behaviour — **stop and report it.** Test *call sites* may change; test *assertions* should not.

## 4. Out of scope

- **Creating, naming, or listing a second active.**
- **Any CLI surface**, including help text.
- **The Workspace concept** — nothing user-visible should learn the word.
- **`verify`'s reporting** (D3.4), crash-safety work (D3.3), and lifecycle operations. Later.

## 5. Controls

1. **The G1 compatibility fixture passes unchanged** — a `0.26.0`-vintage repository still opens,
   verifies, and reports the same schema coverage. **This is the control that proves the on-disk layout
   did not move.**
2. **The full suite passes with no assertion edited** — quote the count, and **list every test whose
   assertion you changed, if any.** The expected answer is none.
3. **A second active is representable but unused** — show that the mechanism accepts another name
   without anything creating one.
4. **`boundary-check`, `reference-check`, and the naming gate pass**, and the **cross-platform CI jobs
   go green** — Windows and macOS exercise the WAL and lock paths differently, and this touches both.

**Local green is not sufficient here.** The mutation and conformance jobs are the real evidence, as
they were for the fixture-realism increment.

## 6. What to report

1. **Your §2 adjudication** — the shape of the name parameter, and why.
2. **The production/test split you actually found**, and whether it matches my table.
3. **Every test assertion you changed**, or that none were (§5.2).
4. All four controls (§5), quoted, including **per-job CI results**.
5. **Full gate set against the exact commit, after the last edit.**
6. **Every numbered requirement's disposition, including ones that went without incident.**
7. Anything here was wrong.

**Stop and escalate, do not guess**, if: an existing test's *assertion* must change to pass (§3) — that
means behaviour moved and this increment's premise is broken; or the on-disk layout shifts in any way
the G1 fixture notices — **that would be a format change, and this increment is not authorised to make
one.**
