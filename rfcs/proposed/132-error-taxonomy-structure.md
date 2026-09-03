# RFC 132 — `PrikkError` carries less than it knows, and cannot grow

**Status.** **ACCEPTED by the project owner 2026-09-03**, at the scope written here — including
§5's two-increment split, §5's refusal of `source()` in increment 1, and §6's requirement that
`Display` output not change. Increment 1 was ruled by the architect the same day and its handoff
is issued. Raised by the external
architecture audit of 2026-08-31 as **`ROADMAP.md`'s `AUD-04`**, the last of that program's four
remaining rows and the only one carrying a sequencing constraint: **before any stability promise.**

**Tracks.** The shape of one public type. **No user-visible message changes in increment 1** — see §6.

---

## 1. What the audit found, and the part it did not

`AUD-04`'s row reads: *"`PrikkError` discards `io::ErrorKind`, implements no `source()`, and is not
`#[non_exhaustive]`."* All three are true at `663ccf6`.

**The row understates the first one.** `From<std::io::Error>` does discard the kind:

```rust
impl From<std::io::Error> for PrikkError {
    fn from(value: std::io::Error) -> Self { Self::Io(value.to_string()) }
}
```

**But that is not the only thing wrong with `Io`.** There are **45 explicit `PrikkError::Io(...)`
construction sites across 20 files, and none of them is an operating-system I/O failure.** A sample:

| Site | Message |
|---|---|
| `fsutil.rs:108` | `"temporary path destination has no file name"` |
| `fsutil/anchored.rs:201` | `"repository mutation requires Linux, macOS, or Windows root-scoped filesystem capabilities"` |
| `fsutil/anchored/directory.rs:372` | `"path must be relative to its authority root"` |

These are a caller-precondition violation, a platform-capability refusal, and a validation failure.
**`Io` is this codebase's catch-all**, and the only place a real `ErrorKind` ever exists is the `From`
impl above. A mandatory `kind` field would therefore be `None`, or a lie, at 45 of 46 sites.

**That is the actual design defect, and it is bigger than the row.** Fixing it means re-classifying 45
sites into variants that describe what they are — which changes user-facing messages, and so cannot
ride along with the cheap structural work.

## 2. `source()` is in tension with the type's existing derives

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrikkError { ... }
```

**`std::io::Error` is none of `Clone`, `PartialEq`, or `Eq`.** A `source()` that returns anything real
must store the underlying error, which costs at least `Eq` and probably `PartialEq`, or forces an
`Arc` and a hand-written `PartialEq` that ignores the stored error — an `Eq` that says two values are
equal while their sources differ.

**And a `source()` that returns `None` for every variant is worse than none**: it satisfies the
audit's checklist while adding nothing, which is the shape this project refuses elsewhere (RFC 127's
vacuous gate, RFC 126 §2's tautological property).

**So `source()` is not free, and it is not ruled here.** §5 makes it conditional on evidence that does
not exist yet.

## 3. What was verified before ruling

- **No exhaustive `match` on a `PrikkError` value exists anywhere in the workspace.** The one bare
  `match err {` (`lifecycle_cache/replay/tests.rs:304`) is on `LifecycleReplayError` and carries a
  wildcard. **`#[non_exhaustive]` is free.**
- **`Clone`/`PartialEq`/`Eq` have no found in-tree consumer** — no `assert_eq!` on a `PrikkError`
  value, no `err.clone()`. **Not proof they are unused**: five modules hold a `PrikkError` inside
  another type, and any of those deriving `PartialEq` would require it transitively. §5 requires that
  check before anything depends on the answer.
- **The CLI flattens every error to a string**: 240 `map_err(|err| err.to_string())` sites, and
  `CliError` is `Usage(String) | Failure(String)`. **Nothing in the CLI matches on a `PrikkError`
  variant**, so no exit-code mapping is affected — and equally, **none of this work improves what a
  CLI user sees.** It improves the library for an embedder. Saying otherwise would be a false promise.
- **`Display` output is documented, user-facing surface.** `docs/src/guide/troubleshooting.md:51` uses
  the literal rendered string `error: i/o error: repository mutation requires Linux, macOS, or
  Windows root-scoped filesystem capabilities` as a section heading.
- **`prikk-error` has zero dependencies** and `std::io::ErrorKind` is std, so the row's "no new
  dependency" constraint is satisfiable.

## 4. Why this is sequenced before a stability promise

`#[non_exhaustive]` is the item that matters for that constraint, and it is the cheapest of the three.
**Without it, every future variant added to `PrikkError` is a breaking change for every embedder.**
The crate is published — `prikk-error 0.29.0` is live on crates.io — so the window in which this
costs nothing is open only while the project is pre-1.0 and says so.

## 5. Ruling — two increments, and what separates them

**Increment 1 (ruled, and the subject of this RFC's first handoff):**

1. **`#[non_exhaustive]` on `PrikkError`.** Free, per §3.
2. **`Io { kind: Option<std::io::ErrorKind>, context: String }`.** `From<std::io::Error>` supplies
   `Some(kind)`; the 45 hand-built sites become `None`. **`Option` is transitional and deliberate** —
   it records honestly that most current uses have no kind, rather than inventing
   `ErrorKind::Other` for them. Increment 2 is expected to make it non-optional by moving those sites
   elsewhere.
3. **A classification of all 45 sites**, produced as evidence, not as a change.
4. **Whether `Clone`/`PartialEq`/`Eq` are load-bearing**, answered by checking the five modules that
   hold a `PrikkError` inside another type.

**`source()` is refused in increment 1.** It cannot be implemented meaningfully without storing a
source, storing a source cannot be costed without item 4's answer, and implementing it vacuously is
worse than leaving it absent.

**Increment 2 (not ruled; needs increment 1's evidence):** re-classify the 45 sites, make `kind`
non-optional, and decide `source()` against the derive question. **This one changes user-facing
messages** and therefore needs release notes and a `troubleshooting.md` pass.

### Increment 1's evidence, delivered 2026-09-03 (`264ba73`) and verified by the architect

**The 29 production sites classify as:** 12 caller-precondition violations, 6 validation failures,
5 "the entry disappeared" races, 4 real OS I/O failures, 2 platform-capability refusals. The
remaining 16 of the 45 are test-only — including `fsutil/anchored/failpoints.rs:256`, which is under
no `tests/` path but whose only call site is `#[cfg(test)]`-gated.

**The "entry disappeared" race is a sixth category increment 2 must carry**: five sites report a
directory entry or worktree file vanishing between one syscall and the next. They synthesize rather
than wrap an `io::Error`, but they describe a genuine external race, not a caller mistake.
`fsutil.rs:114` (a `getrandom` failure) is filed as a platform-capability refusal rather than I/O and
is explicitly flagged as arguable — **increment 2 rules it, rather than inheriting the placement.**

**The derive question is answered, by perturbation rather than by grep, and reproduced independently:**

- **`Clone` is dead.** Removing it produces **zero** compile errors across `--workspace
  --all-targets`. It can go whenever increment 2 wants it gone.
- **`PartialEq`/`Eq` are load-bearing — 56 compile errors — but not for the reason the question
  assumed.** Nothing compares two `PrikkError` values. The dependency is entirely transitive:
  `Result<T, E>: PartialEq` requires `E: PartialEq`, and **54 sites across 11 files in two crates**
  write `assert_eq!(result, Ok(x))`. Heaviest are `prikk-object/src/payload/tests.rs` (19),
  `prikk-store/src/wal/tests.rs` (10) and `refs/tests.rs` (10); all 11 are test-only, `vectors/hard.rs`
  included (`vectors.rs` is `#![cfg(test)]`).

**So `source()`'s real price is rewriting 54 test assertions**, not redesigning the type. That is the
number increment 2 is scoped against.

## 6. What must not change in increment 1

**`Display` output, exactly.** `Io` must still render `i/o error: {context}` — the structured field is
for programmatic access, not for changing what anyone reads. `troubleshooting.md:51` and every test
asserting on message text must pass untouched. **A diff that changes message text has left increment
1's scope**, whatever else it got right.

## 7. Scope

**In:** the four items in §5's increment 1.

**Out:** `source()`; re-classifying the 45 sites; any `Display` change; any CLI change. The CLI's
240-site string flattening is real and worth its own decision, but it is not this RFC's — recorded in
§3 so nobody mistakes this work for a user-visible improvement.
