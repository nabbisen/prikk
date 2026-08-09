# RFC (accepted) - DC-81 macOS Mutation

**Status.** **ACCEPTED by the project owner 2026-08-09**, who directed cross-platform mutation as a
priority and approved the sequence this sits in. **Independence.** Author-reviewed — the standing ceiling.
**Arises from.** The owner's direction of 2026-08-08, and **DC-76**, which exists to make this tractable.
**Target.** 0.20.0, item 2. **Status-claim criterion 6.**

## 1. What DC-76 already settled, so this increment does not re-derive it

- **Nine guarantees are stated**, as `DurabilityContract` — guarantee-named, not syscall-named.
- **The `target_os = "linux"` gates are incidental, not a primitive boundary.** Every `rustix` primitive
  in use is gated against `redox`/`espidf`/`horizon`/`wasi` only, **never against `apple`**; `renameat`
  is ungated entirely and `renameat_with` explicitly includes `apple`. Architect-verified against
  `rustix` 1.1.4 on disk. **This is a port, not a redesign.**
- **G3 needs a different primitive on macOS.** `rustix`'s own documentation (`src/fs/fd.rs:253`) states
  `fsync` does **not** ensure persistent storage on Apple and directs callers to `fcntl_fullfsync`,
  which `rustix` wraps (`src/fs/fcntl_apple.rs:24`) — so this stays inside the existing
  `getrandom` + `rustix` dependency envelope, and **`ALLOWED_THIRD_PARTY` needs no change.**
- **DC-41's crash matrix is portable** — its assertions are guarantee-level observations of durable
  state, and its 24 failure-injection seams are plain thread-local checks, not OS-specific.

## 2. The verification problem, which is new and shapes everything

**Every increment to date was verifiable locally, with CI as confirmation. This one inverts that.**
Neither the developer nor the architect can run macOS tests locally. **CI on a macOS runner is the only
verification available**, and CI currently exercises macOS for *read-only conformance* and clippy only —
there is no job that runs the mutation suite on macOS.

**So building that job is part of this increment, and it must exist and pass before any gate is
relaxed.** An implementation that cannot be observed is not evidence.

**And a limit to state rather than discover:** a CI runner cannot be power-cycled, so the crash matrix
tests *our* behaviour at injected failure points — it does not prove the OS persisted anything. That is
equally true on Linux and is not a new weakness, but it matters more where `fsync` semantics differ.
**Do not let a green crash matrix be read as proof that macOS durability holds.**

## 3. Blocking prerequisites

1. **APFS is case-insensitive by default.** DC-72 built case-collision rejection because prikk must
   refuse what the filesystem would silently fold. On macOS the filesystem folds case *underneath*
   prikk. **What happens to NFR-SEC-03's guarantees, and to a repository created on a case-insensitive
   volume?** Answer before designing — this may be the largest finding in the increment and it is not a
   durability question at all.
2. **Can CI run the mutation suite on macOS at all?** Runner availability, and whether the existing test
   harness carries Linux assumptions — paths, `/tmp` behaviour, permissions, `TMPDIR`.
3. **What does `fcntl_fullfsync` cost?** It is materially slower than `fsync` by design. Commit and seal
   are already durability-bound; measure it, because it may change NFR-PERF-01's picture on macOS.
4. **Does the conformance suite make Linux-specific assumptions?** DC-76 asserted it is portable. **Test
   that claim; do not inherit it.**

## 4. Acceptance criteria

1. §3 answered and reported before any design.
2. `MacosDurability` implements `DurabilityContract`, with **G3 using `fcntl_fullfsync`** and the reason
   recorded at the call site.
3. **The conformance suite's assertion bodies pass on macOS unchanged.** **Amended 2026-08-09** — as
   first written this said "passes on macOS unmodified", which §1's report showed is unsatisfiable: the
   suite is `#[cfg(all(test, target_os = "linux"))]` at the module level and does not compile on macOS
   at all, so relaxing a gate is itself a modification. Module gates may be relaxed and a
   per-implementor `#[test]` wrapper added per assertion — both mechanical. **Any change to an assertion
   body, or to what it asserts, is a finding to report**, because it would mean the contract was written
   to Linux rather than to the guarantee.
4. **DC-41's crash matrix passes on macOS**, through the same seams, module gate relaxed only.
5. **A CI job runs the macOS mutation suite and is green** before any `target_os` gate is relaxed in a
   merged commit.
6. **Linux behaviour is unchanged** — every existing test passes unchanged, and the negative controls
   DC-76 established still fail when their guarantee is removed.
7. Gate set per `EXECUTION-ORDER.md` §6 rule 9 **as amended** — the canonical nine plus macOS and Windows
   clippy, since this touches `#[cfg(target_os)]` code.
8. Documentation updated: `README.md` and `docs/src/reference/platform-support.md` must not claim macOS
   mutation before criterion 5 holds. **The project has published a false portability claim once; it
   will not do so twice.**

## 5. Non-goals

- **Windows mutation.** Separate, and still carrying an unresolved dependency question.
- Changing any guarantee. If macOS cannot satisfy one of the nine, **stop and report** — that is a
  finding about the contract, and possibly about DC-37, not scope to absorb.
- Relaxing gates for any platform other than macOS.
