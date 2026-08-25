# RFC 118 stage 5 — fix the Windows-only failure in the hostile-escaping control

**URGENT: `main` is red.** `da2b242` and `770931f` both fail CI. **Take this before anything else.**

**Base:** current `main` (`770931f`). **Under `003-landing-work-on-main.md`.**

---

## 1. What failed

`Windows mutation test suite` — every other job passed, including both macOS jobs and the Windows
build and conformance jobs.

```
test hostile_stage_failure_message_escapes_correctly_and_the_document_still_parses ... FAILED
create the stray non-file entry: Os { code: 123, kind: InvalidFilename,
message: "The filename, directory name, or volume label syntax is incorrect." }
```

**The test never reached the escaper.** It plants a directory named
`quote"back\slash\nnewline\ttab\u{1}control`, and Win32 forbids `"`, `\`, and every character below
`U+0020` in a filename. **The directory cannot be created, so the control could not run at all.**

**The escaper itself is not implicated.** `escape_json_string` handles `"`, `\`, `\n`, `\r`, `\t`, and
all C0 controls, and that is unchanged. **This is a defect in how the control delivers its input, not
in what it proves.**

**This was my miss in review, not yours.** I check platform behaviour when a diff contains
`#[cfg(target_os)]`. This diff contained none — because the platform dependence here is **unmarked**,
which is precisely the case that rule cannot see. **The absence of a `cfg` is not evidence of
portability.**

## 2. The fix: prove the escaper directly, not through the filesystem

**Move the hostile-input proof to a unit test of `escape_json_string` itself**, in a `#[cfg(test)]`
module inside `crates/prikk-cli/src/output/verification.rs` (the function is private and `prikk-cli`
has no `lib.rs`, so it must live beside it).

**This is the better control regardless of Windows.** Escaping is a pure function from `&str` to
`String`. Proving it through a planted directory name made the test depend on filesystem naming rules
that have nothing to do with JSON, and **capped the input at whatever the filesystem happens to
allow** — the unit test can pass strings no filesystem would ever accept, which is the point.

**Assert on the full hostile string**, including `"` `\` `\n` `\r` `\t` and at least one C0 control
such as `U+0001`, and assert the output parses. **Paste the expected value from the code, do not
retype it** — the last report's transcription silently dropped the escaped control byte.

## 3. Keep an end-to-end test, but make it portable

The integration test still earns its place: it proves a `Failed` message reaches the JSON escaped, and
that the document parses. **Keep it — with a planted name that is legal on every target.**

**Do not `#[cfg(unix)]` the existing test and call it done.** That would leave the end-to-end path
unproven on Windows, which is where the mutation suite exists to find exactly this class of problem.
**If you conclude a portable name cannot exercise anything meaningful, say so and gate it explicitly
with a comment naming Win32 as the reason** — but try portable first.

## 4. Out of scope

- **`escape_json_string`'s behaviour.** It is correct; do not change it.
- The `ALL`-order emitter, `VERDICT_CONDITIONS`, and everything else reviewed and accepted at
  `da2b242`.
- Any other test's platform assumptions. **If you notice one, report it, do not fix it here.**

## 5. Controls

1. **The unit test fails if the escaper regresses** — break one arm (drop the C0 branch), quote the
   failure, revert.
2. **The portable integration test still catches an unescaped message** — same technique, quote it.
3. **The full suite passes**, and say whether the count moved and why.

## 6. What to report

1. The two tests, and **the exact planted name** you chose for the portable one.
2. **Whether any other test in the workspace plants a path from arbitrary text** — a one-time sweep,
   reported, not fixed (§4).
3. All three controls (§5), quoted.
4. **Full gate set against the exact commit, after the last edit.**
5. Every numbered requirement's disposition.
6. Anything here that was wrong.

**I will push and watch Windows CI specifically before calling this closed.** Local green is not the
bar for this one.
