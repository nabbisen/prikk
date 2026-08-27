# Merge-evidence privacy assertions: establish which can fail, then ground or remove them

**Base:** `5c6edc9` or later `main`. **Under `003-landing-work-on-main.md`** — commit locally on
`main`, do not push, do not tag.

**Filed under RFC 118 deliberately.** This is not RFC 118 work, but the defect is exactly its
subject: **a transcribed magic string standing in for a property a type already guarantees.** The
handoff naming gate needs a numbered directory and this is the closest true one.

---

## 1. What I told the owner, and why it was wrong

I reported this as *"POSIX prefixes that cannot match on Windows, so the privacy control is silently
vacuous there."* **That is true and shallow, and acting on it would have produced the wrong fix** —
adding `C:\Users\` and `C:\Windows\Temp\` would have made a vacuous assertion vacuous in four shapes
instead of two.

`crates/prikk-store/src/patch_algebra/tests/merge_evidence_report_privacy.rs:52-53`:

```rust
assert!(!formatted.contains("/home/"));
assert!(!formatted.contains("/tmp/"));
```

**The report cannot contain any absolute path, on any platform.** I established three things:

1. **`MergeEvidenceItem.path` is `Option<RepoPath>`**, not a `PathBuf`. There is **no `PathBuf`
   anywhere in `patch_algebra/report/`.**
2. **`RepoPath::parse` validates through `validate_repo_path`**, which rejects a leading `/`,
   **backslashes**, and **colons** — so `/home/x`, `C:\Users\x` and `C:/Users/x` are all rejected.
3. **This test builds no repository and touches no filesystem.** Its inputs are `node()`/`blob()`
   fixtures and the relative names `"doc.txt"`/`"fresh.bin"`. **Nothing in scope could supply a host
   path even if the type allowed one.**

**So the two assertions cannot fail, anywhere, for a stronger reason than the one I gave.**

## 2. The question this raises about the other three, which you must answer

The same test asserts:

```rust
assert!(!formatted.contains("secret old text"));
assert!(!formatted.contains("secret replacement text"));
assert!(!formatted.contains("secret blob bytes"));
```

**I do not know whether these can fail either.** `MergeEvidenceItem` carries no text or blob payload
field, which suggests the same structural guarantee — **but I have not checked every type reachable
from the report's `Debug`, and I am not going to assert it from a partial read.**

**Establish, for each of the five assertions, whether any input this test can construct could make it
fail.** Report per assertion, with the evidence. **The method that settles it is a probe, not a
read:** make the property false — feed the payload through a path that would reach the report — and
see whether the assertion fires. **An assertion that survives a deliberate violation is vacuous, and
that is the finding.**

## 3. What to do with what you find

**Adjudicate and justify. I am not ruling this.**

- **Remove a vacuous assertion, recording why** — the type guarantees it, so the test was asserting
  the compiler's work.
- **Ground it** so it can fail — if a real path exists by which a payload or host path could reach
  the report, assert against *that*.
- **Replace it with a guard that watches the invariant** rather than a string — the report module
  gaining a `PathBuf`, or a payload-bearing field, is the change that would make the privacy property
  false, and **that is a structural fact a gate can check**, in the shape `rfc_naming.rs` and
  `open_work_index.rs` already use.

**The criterion:** an assertion that cannot fail is worse than no assertion, because it reads as
coverage in a file whose name promises privacy. **But deleting all five and leaving nothing watching
the property is also a loss** — the property is real even if this test does not test it. **Say what
still guards it after your change.**

**If you conclude the honest answer is "delete these and the invariant is guarded by the type
system", that is an acceptable outcome** — say so plainly and name the types doing the guarding.

## 4. What must not change

- **No production code.** If closing this properly needs a production change, **stop and report** —
  that is a different increment.
- **The first test in this file** (`report_items_have_deterministic_secondary_ordering`) is not in
  scope.
- **No `#[cfg]` gating as a fix.** The defect is not platform-specific; a platform gate would encode
  my original wrong diagnosis into the code.

## 5. Controls

1. **Per-assertion, whether it can fail** (§2), each with its probe result quoted.
2. **Whatever you keep, prove it fires** — the standard this arc has held throughout: a control that
   passes with the mechanism removed is not a control.
3. **The full suite, count moved or unmoved as your change implies** — and say which you expected
   before running it.
4. **Full gate set against the exact final commit.**
5. **Per-job CI** — named as unavailable locally. **No platform gate should appear in this diff**
   (§4); if you think one is needed, that contradicts §1 and is a stop-and-report.

## 6. The report

To `.git-exclude/review-request/`. Include §2's per-assertion findings, §3's adjudication with
reasoning, what still guards the property afterwards, all five controls, the full gate set, and
**anything in this handoff that was wrong** — including §1's analysis, which I derived myself and
which you should re-derive rather than trust.
