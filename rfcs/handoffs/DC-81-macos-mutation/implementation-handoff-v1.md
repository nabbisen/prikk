# DC-81 macOS Mutation — Handoff v1

**Cleared to start on §1 only.** Accepted 2026-08-09, `rfcs/accepted/DC-81-MACOS-MUTATION.md`.
**Authored by** the architect. **This is the increment the owner's cross-platform priority points at**,
and DC-76 exists to make it a port rather than a redesign.

## 1. What you already established, so you do not redo it

Your own `prerequisite-questions-v1.md` settled the hard part: **the gates are incidental, not a
primitive boundary.** Every `rustix` primitive in use excludes only `redox`/`espidf`/`horizon`/`wasi`,
never `apple`. **G3 is the one real difference** — `fcntl_fullfsync`, which `rustix` wraps, so the
dependency envelope is unchanged and `ALLOWED_THIRD_PARTY` needs no edit.

## 2. Four questions before design — and the first is not about durability at all

1. **APFS is case-insensitive by default.** DC-72 built case-collision rejection precisely because prikk
   must refuse what a filesystem would silently fold — and on macOS the filesystem folds case
   *underneath* prikk. **What happens to NFR-SEC-03 on a case-insensitive volume?** I think this may be
   the largest finding in the increment, and it is not a durability question. Answer it first.
2. **Can CI run the mutation suite on macOS at all?** Runner availability, and whether the harness
   carries Linux assumptions — paths, `/tmp`, `TMPDIR`, permissions.
3. **What does `fcntl_fullfsync` cost?** Materially slower than `fsync` by design, and commit/seal are
   already durability-bound. Measure it — it may change NFR-PERF-01's picture on macOS.
4. **Does DC-76's conformance suite make Linux-specific assumptions?** It was asserted portable. **Test
   the claim; do not inherit it** — including from yourself.

## 3. The verification problem, which is genuinely new

**Every increment so far was verifiable locally with CI as confirmation. This one inverts that.** Neither
you nor I can run macOS locally. **CI on a macOS runner is the only verification available**, and today
CI exercises macOS for read-only conformance and clippy only — no job runs the mutation suite there.

**Building that job is part of this increment, and it must be green before any gate is relaxed in a
merged commit.** An implementation nobody can observe is not evidence.

**One limit to state rather than discover:** a CI runner cannot be power-cycled. The crash matrix tests
*our* behaviour at injected failure points; it does not prove the OS persisted anything. That is equally
true on Linux, so it is not a new weakness — but it matters more where `fsync` semantics differ. **Do not
report a green crash matrix as proof that macOS durability holds**, and I will not accept it as such.

## 4. The bar

- **DC-76's conformance suite passes on macOS unmodified.** A suite that must change to pass is a
  **finding** — it would mean the contract was written to Linux rather than to the guarantee, which is
  the one thing DC-76 existed to avoid.
- **Linux behaviour unchanged**, and DC-76's nine negative controls still fail when their guarantee is
  removed. I will re-run some.
- **Docs must not claim macOS mutation before the CI job is green.** The project published a false
  portability claim once; it will not do so twice.

## 5. Stop-and-report conditions

**If macOS cannot satisfy one of the nine guarantees, stop and report.** That is a finding about the
contract — possibly about DC-37 itself — and not scope to absorb. The same applies if question 1 shows
case-insensitivity breaks something DC-72 guaranteed.

Gates: rule 9 **as amended** — the canonical nine plus macOS and Windows clippy.
