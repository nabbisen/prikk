# RFC 112 — Three core write operations live in the binary crate

**Status.** **Proposed.** Raised by the architect 2026-08-18, at the project owner's request, from a
finding surfaced by RFC 111 Stage 2. **Not a design.** Independence: author-reviewed, the standing
ceiling — every claim below is a file-level fact that can be re-derived in one command.

**Arises from.** RFC 111 Stage 2 needed a cost gate on `seal` and could not write one, because `seal` is
not reachable in-process. The workaround — a 167-line replica plus a drift guard to police it — is the
symptom this RFC is about, not the disease.

## 1. The finding

**`seal`, `branch create`/`close`, and `tag create` are implemented in `crates/prikk-cli`, the
binary-only crate. Every other core operation lives in `prikk-store`.**

Which CLI modules carry write logic, by count of object writes, ref publications, and state-root
derivations in each file:

| CLI module | `write_object` | `publish` | `derive_next_state_root` |
|---|---:|---:|---:|
| `seal.rs` (+ `seal/support.rs`, 440 lines) | 3 | 1 | 2 |
| `branch.rs` | 3 | 2 | 0 |
| `tag.rs` | 3 | 1 | 0 |
| `bundle.rs` | 0 | 0 | 0 |
| `compact.rs` | 0 | 0 | 0 |
| `unlock.rs` | 0 | 0 | 0 |

The bottom three delegate to `prikk-store` and hold only argument parsing and output. **The top three do
the work themselves.**

`prikk-store`'s public surface confirms the asymmetry. It exports `verify_repository`,
`commit_worktree_changes_signed`, `execute_merge`, `export_bundle`, `import_bundle`, `compact`,
`doctor`, `append_rollback_draft` — **and nothing for seal, branch, or tag.**

## 2. Three consequences, in increasing order of seriousness

### 2.1 The library cannot complete a basic workflow

`prikk-store` is a published crate. A library consumer can initialize a repository, author and commit
patches, merge, verify, bundle, compact and diagnose — **and cannot seal, branch, or tag.** Sealing is
what turns WAL patches into immutable signed history; it is the operation the product's central claim
rests on, and it is absent from the library that claims to implement the product.

**The nearest available symbol is `simulate_one_seal_for_test_support`** — a feature-gated replica
written for a benchmark gate. That is currently the only seal-shaped thing `prikk-store` exports.

### 2.2 These operations cannot be tested in-process, only through a subprocess

A binary crate exposes no library target, so a test can only drive these three by spawning the compiled
binary. That is adequate for behaviour, and **insufficient for anything that must observe what the
process did** — allocation, call counts, internal state.

RFC 111 Stage 2 hit exactly this: its cost gate counts index decodes via an in-process thread-local, and
a subprocess cannot share one. **A cost gate on `seal` was therefore impossible to write against `seal`.**

### 2.3 It has already forced a second implementation into existence

`crates/prikk-store/src/rfc111_seal_simulation.rs` reproduces `seal`'s sequence so the gate has something
to measure, and `crates/prikk-cli/tests/rfc111_seal_drift_guard.rs` exists solely to detect the replica
drifting from the real thing.

**That is two artifacts and an ongoing obligation, all bought to work around a module living on the wrong
side of a crate boundary.** The drift guard is correct and I required it — but a guard against
self-inflicted duplication is a cost, not a feature.

## 3. Why this went unnoticed

Nothing enforces the boundary. `prikk-cli` is free to depend on `prikk-store` and write whatever it
likes, and the three operations that ended up there are ones whose CLI surface was written first. The
project has a placement gate for `unsafe` (DC-90) and one for RFC naming (RFC 105); **it has none for
"core operations belong in the library."**

## 4. What a design must decide

1. **Where the boundary actually is.** The honest counter-argument deserves an answer rather than a
   dismissal: `branch` and `tag` involve naming policy and user-facing defaults, which are plausibly
   command concerns. **`seal` has no such defence** — it derives a state root, writes a Block, and
   publishes a RefState. Decide the rule, then apply it; do not decide per-module by taste.
2. **What the library API should be**, if these move. `execute_merge`'s existing shape is the obvious
   precedent, and following it costs nothing extra.
3. **Whether `prikk-cli` should gain a `lib.rs` instead.** Cheaper than moving code, and it would make
   the operations testable in-process — but it leaves the library gap in §2.1 wide open and adds a
   second published API surface. **My assessment: it addresses the least serious consequence and none of
   the others.**
4. **What happens to RFC 111's simulation and drift guard.** If `seal` becomes callable in-process, the
   gate should call the real function and **both artifacts should be deleted.** Retiring them is part of
   the work, not a later tidy-up — a replica left behind after the reason for it is gone is exactly the
   staleness this project keeps finding.
5. **Staging.** `seal` first — it is the one with no counter-argument, the one the gate needs, and the
   one whose migration retires the two artifacts. `branch`/`tag` follow, or are ruled out of scope by
   §4.1's answer.

## 5. What this is not

- **Not a behaviour change.** Nothing here proposes altering what `seal`, `branch` or `tag` do. A move
  that changes behaviour has failed.
- **Not an argument that `prikk-cli` should be thin on principle.** The claim is narrower and evidenced:
  three operations sit on a different side of the boundary from every comparable one, and that
  inconsistency has measurable costs.
- **Not urgent.** Nothing is broken for users today. This is a structural debt with a known price, now
  written down instead of rediscovered the next time something needs to observe a seal.

## 6. Acceptance shape

A design is acceptable when it answers §4.1 with a stated rule, moves what that rule requires **with no
behaviour change**, replaces RFC 111's simulated gate with one that calls the real operation, and deletes
the simulation and its drift guard in the same increment.
