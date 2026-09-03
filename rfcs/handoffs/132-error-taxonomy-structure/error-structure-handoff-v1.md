# RFC 132 increment 1 — make `PrikkError` able to grow, and stop it discarding the kind

**Authority:** `rfcs/proposed/132-error-taxonomy-structure.md` §5, increment 1.
**Base:** current `main` (`663ccf6`). **Under `003-landing-work-on-main.md`.**

**Scope: the four items in §5's increment 1 and nothing else.** `source()` is **refused** for this
increment — read §4 below before assuming that is an oversight. Re-classifying the 45 catch-all sites
is increment 2 and needs this increment's evidence first.

---

## 1. `#[non_exhaustive]` on `PrikkError`

**Verified free before asking for it:** no exhaustive `match` on a `PrikkError` value exists in the
workspace. The one bare `match err {` (`lifecycle_cache/replay/tests.rs:304`) is on
`LifecycleReplayError` and already carries a wildcard arm.

**Confirm that yourself before relying on it.** `#[non_exhaustive]` binds other crates, not the
defining one, so the compiler will tell you — but a clean build is the *result* to report, not the
check.

**This is the item that earns the whole increment.** `prikk-error 0.29.0` is published; until this
lands, every future variant is a breaking change for anyone who depends on it.

## 2. `Io { kind: Option<std::io::ErrorKind>, context: String }`

```rust
impl From<std::io::Error> for PrikkError {
    fn from(value: std::io::Error) -> Self {
        Self::Io { kind: Some(value.kind()), context: value.to_string() }
    }
}
```

The 45 explicit construction sites become `kind: None`.

**Why `Option` and not `ErrorKind::Other`.** None of those 45 sites is an operating-system failure —
they are caller-precondition violations, platform-capability refusals, and validation failures wearing
the wrong variant. `Other` would assert something false about each of them. **`None` records that
there is no kind, which is the truth, and increment 2 is expected to remove the `Option` by moving
those sites to variants that describe them.** Do not "tidy" this into a non-optional field.

**`prikk-error` has zero dependencies and `ErrorKind` is std** — the row's "no new dependency"
constraint holds. Note the crate currently imports `core::fmt`, not `std`; adding a `std::io` type is
a deliberate widening, so say in your report whether anything in the crate assumed `core`-only.

## 3. `Display` must not change — this is the constraint most likely to be broken

`Io` must still render exactly `i/o error: {context}`.

**`docs/src/guide/troubleshooting.md:51` is a section heading made of that rendered string**:
`` ## `error: i/o error: repository mutation requires Linux, macOS, or Windows root-scoped filesystem capabilities` ``.
Several test files assert on `"i/o error"` text as well.

**Do not add the kind to the message**, however tempting — the structured field is for programmatic
access. **If any test's expected message changes, you have left scope**, and the right response is to
stop and report, not to update the expectation.

## 4. `source()` is refused, and the reason is the point

`PrikkError` derives `Clone, PartialEq, Eq`. **`std::io::Error` is none of those.** A `source()`
returning anything real needs a stored error, which costs `Eq` outright and forces either dropped
derives or an `Arc` plus a hand-written `PartialEq` that ignores the stored source — an `Eq` claiming
two values equal while their sources differ.

**And `fn source(&self) -> Option<&(dyn Error)> { None }` is worse than not implementing it**: it
satisfies the audit's checklist and adds nothing. This project has refused that shape twice already
(RFC 127's gate that could not fail, RFC 126 §2's tautological property).

**So do not implement it. Produce §5's evidence instead**, and increment 2 will rule it.

## 5. The two pieces of evidence this increment exists to produce

**These are deliverables, not homework.** Increment 2 cannot be ruled without them.

**(a) Classify all 45 `PrikkError::Io` construction sites.** For each: file:line, the message, and
what it actually is — OS I/O failure, caller-precondition violation, platform-capability refusal,
validation failure, test-only failpoint, or something my six categories miss. **My count of 45 across
20 files is from `grep -c` summed across files; derive your own and correct me.** Note that some of
those files are test modules, which may or may not belong in the same treatment — say which you think
and why.

**(b) Are `Clone`, `PartialEq`, and `Eq` load-bearing?** I found no `assert_eq!` on a `PrikkError`
value and no `err.clone()`, **but that is not proof.** Five modules hold a `PrikkError` inside another
type — `block_state.rs`, `doctor.rs`, `fsutil/anchored.rs`, `lifecycle_cache/cache_ladder.rs`,
`worktree_patch/node_authoring.rs` — and any of those deriving `PartialEq` or `Clone` needs it
transitively. **Check each. Report "all five are fine" explicitly if that is the answer** — a negative
result stated plainly is what makes increment 2 decidable.

## 6. What this work does not do, and must not be reported as doing

**No CLI user sees any difference.** `CliError` is `Usage(String) | Failure(String)`, and errors reach
it through **240** `map_err(|err| err.to_string())` sites. Nothing in the CLI matches a `PrikkError`
variant, so no exit code moves and no message changes.

**This increment improves the library for an embedder and produces evidence for increment 2.** If your
report implies a user-facing improvement, it is wrong.

## 7. Gates and reporting

Full set from `rfcs/EXECUTION-ORDER.md` §6 rule 9, run from there against your final commit — **not
reproduced here**: `reference-check` rejects a policy-command line outside its registered sites.

**`mdbook build` does not apply** unless you change a `docs/src/` page — and changing one means §3 was
violated.

**This is a breaking change to a published crate.** Pre-1.0 and permitted, but say so in your report
so it reaches the next release's notes; `release-compatibility.md`'s policy requires named changes.

Local commits on `main`; **no push, no tag, no publish.** Report to `.git-exclude/review-request/`,
and state:

1. Your own derived count and classification of the `Io` sites (§5a).
2. The `Clone`/`PartialEq`/`Eq` answer for all five modules (§5b).
3. That no test's expected message text changed — and if one did, stop rather than update it.
4. Whether the crate had any `core`-only assumption that `std::io::ErrorKind` breaks.
5. Every place this handoff's claims proved wrong. **The 45, the 20 files, the five modules and the
   240 sites are all my counts, and this project's handoffs have a consistent record of getting them
   wrong in both directions.**
