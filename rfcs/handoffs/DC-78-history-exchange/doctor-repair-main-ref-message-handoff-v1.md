# `doctor --repair-main-ref`'s refusal message: implementation handoff

**Base:** current `main` (`b6cd309`, tagged `0.23.0`). **Under `003-landing-work-on-main.md`.**
**Origin:** `.git-exclude/reviewed/pre-release-cli-currency-review-v1.md` §3, recorded as unowned.

**Small, but it is not the version-string swap the review recorded.** Investigating it to write this
handoff turned up **three defects in one string**, and the version number is the least of them.

---

## 1. The string

`crates/prikk-store/src/doctor.rs:396-402`:

```rust
layout.require_current_format()?;                                        // 396
if options.reconstruct_main_ref {                                        // 397
    return Err(PrikkError::Integrity(
        "format-1 missing-pointer doctor repair is unsupported in 0.18.0; preserve the \
         repository for signer-backed retry or later recovery tooling"
            .to_string(),
    ));
}
```

### Defect 1 — it names a version five releases back

`0.18.0`. Current is `0.23.0`. Worse than merely old: **"unsupported in 0.18.0" implies a later version
may differ.** Nothing changed, and nothing is scheduled to.

### Defect 2 — it describes a scenario this code path cannot reach

**`require_current_format()` runs first, on line 396.** It returns `Err(UnsupportedFormatVersion(1))` for
any layout that is not `CurrentV6` (`layout.rs:330-336`).

**So a format-1 repository never reaches line 397** — it is refused one line earlier, with a different
error. The message describes a **format-1** repair, but is **only ever emitted for a format-6
repository.** A user who sees it does not have the problem it names.

### Defect 3 — it offers a remedy that does not exist

*"preserve the repository for signer-backed retry or later recovery tooling"* — there is no
signer-backed retry and no recovery tooling. **This is exactly what RFC 114 ruled against** when it made
formats 1-5 refuse outright *"rather than offering a migration the product cannot honour."* That ruling
reached the format refusals; it did not reach this one.

**This is the same class as the `0.23.0` changelog's original tag advice** — text telling a user to do
something that does not exist. Verify the replacement against the code, do not write what sounds
reasonable.

## 2. What the message should say

**The truth: this repair has never been implemented.** No version, no format-1 framing, no future
tooling promised. State that `doctor` cannot reconstruct a missing main ref and that the flag has no
working behaviour.

**Derive the exact wording yourself from the code**, and say in your report what you based it on. **Do
not copy a sentence out of this handoff** — my last four handoffs have each contained at least one
wrong claim, and §1's three defects are my reading, not a verified fact you may inherit.

## 3. A test asserts the current string — it must be updated, not deleted

```
crates/prikk-store/src/refs/tests/publication_recovery.rs:360
    assert!(error.to_string().contains("unsupported in 0.18.0"));
```

**This will fail the moment you change the message.** Update the assertion to match the new wording.
`crates/prikk-store/src/doctor/tests.rs:298` also exercises `repair_repository` with
`DoctorRepairOptions::reconstruct_main_ref()` — **read it and check whether it asserts on text too.**

**Do not weaken either assertion to something that would pass regardless** (e.g. asserting only that an
error occurred). **A test that no longer distinguishes the right refusal from the wrong one is worse
than the stale string**, because it would let the next drift through silently. If the new message makes
a precise assertion awkward, say so rather than loosening it.

## 4. Out of scope

- **Do not add `--repair-main-ref` to `--help`.** Ruled in the CLI currency review: an always-refusing
  flag does not belong in a command inventory.
- **Do not remove the flag.** Whether a permanently-refusing CLI flag should exist at all is a real
  question, but it is a CLI surface change and needs its own adjudication. **If you form a view while in
  there, report it** — that is the kind of finding this increment is well placed to surface.
- **Do not touch `require_current_format` or the format-1 refusal path.** Defect 2 is a defect in the
  *message*, not in the ordering. The ordering is correct: refusing an unsupported format before
  considering a repair is right.

## 5. What to report

1. The new message, and **what in the code you derived it from.**
2. **Whether my §1 reading holds** — especially defect 2. **Re-derive the ordering yourself**: does
   `require_current_format` really refuse format-1 before line 397 is reached? If I am wrong, say so.
3. Both tests: what you changed, and **why the new assertion still distinguishes this refusal from any
   other.**
4. Any view on the flag's existence (§4), report only.
5. The **full gate set against the exact commit, after the last edit.**
6. Test counts — **expected unchanged at 1302**; the assertion changes, the count does not.
7. Anything here that was wrong.

**Stop and escalate, do not guess**, if: the correct wording depends on whether a future repair is
planned (it is not mine to promise); or you find the flag reachable in some path I have not seen.
