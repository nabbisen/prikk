# `sync`'s four output writes have the defect `bundle export` just had

**Authority:** surfaced by DC-44 increment 2's own report and confirmed in review; owner instructed
2026-08-31 to take it. **Base:** `fd2424d` or later `main`.
**Under `003-landing-work-on-main.md`** — commit locally on `main`, do not push, do not tag.

**This is a small, mechanical increment.** The primitive already exists and was built for reuse; the
adjudications are about policy consistency, not design.

---

## 1. The defect, and why it outranks new capability

**Four sites in `crates/prikk-cli/src/sync.rs` write to a user-supplied path with `std::fs::write`,
which creates-or-truncates:**

```
sync.rs:82    run_summary
sync.rs:117   run_have
sync.rs:145   run_build
sync.rs:232   run_accept      (--claims-out)
```

**Zero of them guard the destination** — confirmed by grep, no `--force`, no existence check.

**This is the same failure DC-44 increment 2 just fixed for `bundle export`**: an existing file is
destroyed silently, and an interrupted write leaves a partial file at the destination having already
destroyed what was there.

**`sync build` is the sharpest case.** It writes the `PEXCH002` exchange artifact — the thing you
hand to the other party. Building a second artifact over the first destroys it with no message; if
that write fails partway, you have neither.

**Taken before DC-44's remaining items on severity, not size:** this is a live data-loss shape in four
shipped commands, whereas the manifest is a new capability.

## 2. What to build

**Reuse `crates/prikk-cli/src/durable_output.rs`.** It was built small, dependency-free and reusable
for exactly this, and it has just been reviewed: `create_new` → `write_all` → `sync_all` → `rename` →
parent-directory sync, with best-effort temp cleanup and a documented TOCTOU limit on the existence
check.

**Do not reimplement, and do not extend it without saying why.** If a site needs something it does not
offer, that is a finding worth reporting before writing code.

## 3. What you must adjudicate

**3.1 — does every site get the same `--force` policy?** `bundle export` refuses by default and
overwrites with `--force`. **Consistency across the CLI is a strong default**, but argue it per
command rather than assuming: an exchange artifact and a `--claims-out` file may not carry the same
consequence, and `sync summary`'s output may be regenerated freely. **If you conclude a site should
differ, say which and why** — a CLI where `--force` means different things in different places is
worse than one where it means nothing in some.

**Note the existing precedent, and stay compatible with it:** `--force` already exists in
`bundle export` (overwrite the destination) and in `unlock` (alias for `--yes`, skip the
confirmation). Both mean *proceed past a safety stop*. **Keep that meaning.**

**3.2 — flag naming where the output flag is not `--output`.** `run_accept` writes via
`--claims-out`. **Decide whether one `--force` covers a command with several outputs**, and say so.

**3.3 — where the collision check goes in each command.** `bundle export` checks before
`open_repository`, so the common case fails before any read. **Whether that ordering is achievable at
each sync site is yours to determine** — some may need the repository to know the output path is even
reachable. Report what you found rather than forcing a uniform shape.

## 4. What must not change

- **What any `sync` command produces.** The bytes are unchanged; this is about how they reach disk.
- **`durable_output`'s own behaviour**, unless §2's finding says otherwise.
- **Negotiation or exchange semantics.** No protocol change, no artifact format change.
- **`bundle export`.** Already done; do not revisit it.

## 5. Controls

1. **A failed write leaves the previous file intact**, at each site that gets the guard. The
   assertion is on the destination's bytes, not the error message.
2. **No partial file anywhere** after a failure — neither at the destination nor as an abandoned temp.
3. **Collision behaves as §3.1 decided**, demonstrated both ways per site.
4. **Each command's output is unchanged** — a successful write produces what it produced before.
5. **Existing sync tests pass unmodified.** If one must change, behaviour moved where §4 forbids —
   **stop and report.**
6. **Full gate set against the exact final commit.**
7. **Per-job CI, re-derived for this diff.** Increment 2 needed `#[cfg(unix)]` for its `chmod`-based
   failure tests; say whether this diff does, rather than inheriting that answer.

## 6. The report

To `.git-exclude/review-request/`. Include §3's three adjudications with reasoning, all seven controls
quoted, the full gate set, and **anything in this handoff that was wrong** — including my claim that
all four sites are unguarded and that `durable_output` needs no extension, both of which I checked by
grep and reading rather than by attempting the change.
