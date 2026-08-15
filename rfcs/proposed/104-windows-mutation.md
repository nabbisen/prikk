# RFC (proposed) - 104 Windows Mutation

**Status.** Proposed 2026-08-16. **No design, no implementation, no production code authorized.**
**Independence.** Author-reviewed — the standing ceiling.
**Arises from.** RFC 102, complete and merged 2026-08-15, which removed the obstacle rather than the
capability; and the owner's direction of 2026-08-16 that the next release should be *"RFC 102 delivered,
Windows mutation enabled."*
**Supersedes.** **DC-37**'s Linux-only mutation ruling, already partly superseded by DC-81 (macOS).
**Target.** Owner's call. RFC 102 §9 already says that arc is **1.0-scale, not 0.20.0** — this is its
other half, and the same judgement applies.

---

## 1. What RFC 102 settled, so this RFC does not re-derive it

**The primitive Windows cannot provide is no longer needed for repository state.** Verified against the
merged tree, not assumed:

- **`durable_directory_entry`** — the durable-new-name confirmation RFC 101 §5.5 established has no
  Windows primitive, documented or otherwise — has **exactly two production callers**,
  `worktree.rs:151` and `:199`, **both on `worktree_mutation_root()`**. Nothing under `.prikk/` calls it.
- **`set_permission_bits`** — same, `worktree.rs:154` and `:158`, worktree only.
- **`promote`** — **zero** production callers.
- **`publish_immutable`** — **zero**; the standing G5 orphan finding.

**So every durability-bearing write to `.prikk/` is now an append to a name allocated at `init`, a
truncate of one, or an exclusive create during `init` itself.** That is the whole point of RFC 102's six
stages, and it is what makes this RFC tractable at all.

**DC-82 already made the platform layer pluggable.** `ACTIVE_DURABILITY` selects one implementor;
`linux.rs`, `macos.rs` and `none.rs` sit behind it. Windows is a fourth file and a `cfg` arm, not a
change to any call site.

## 2. Why this is **not** a port, unlike DC-81

DC-81's shaping claim was *"this is a port, not a redesign"* — because every `rustix` primitive in use was
gated against `redox`/`espidf`/`horizon`/`wasi` and **never against `apple`**. **That argument does not
transfer**, and pretending otherwise is the main way this RFC could go wrong.

Three specific reasons, each checked:

**2.1 `MutationRoot` has no Windows representation, and it is the abstraction everything rests on.**
`fsutil/anchored/directory.rs:23-27` holds an `AnchoredDirectory` behind
`#[cfg(any(target_os = "linux", target_os = "macos"))]` — a retained directory handle giving
`openat`-style anchored access, so every path operation resolves relative to a held authority rather than
by re-walking a string. **Windows has no `openat`.** Whatever replaces it must preserve the property the
anchor exists for — that a mutation cannot be redirected by a path component changing under it — and that
is a design question with real TOCTOU content, not an implementation detail.

**2.2 `rustix` is a POSIX abstraction; a new third-party dependency is near-certain.** `prikk-store`'s
`ALLOWED_THIRD_PARTY` entry is `getrandom` + `rustix`. DC-81 explicitly needed no change to it. Windows
almost certainly needs `windows-sys` or equivalent, which is **policy-gated** — `boundary-check` enforces
the list, and DC-51 governs dependency placement. **Establish this before designing around it**; if some
subset is reachable through `std` alone, that materially changes the shape.

**2.3 Two contract methods are impossible on Windows and need argued semantics, not a silent no-op.**
`durable_directory_entry` cannot be implemented; `set_permission_bits` has no meaningful Windows
behaviour. Both are worktree-only (§1), and the argument that this is acceptable already exists —
worktree content is rebuildable from sealed history, and Stage 1's dirty marker closes T12's
infer-deletion-from-absence hazard. **But that argument has to be written down and tested, not assumed**,
because it is the one place where Windows behaviour genuinely differs from Linux rather than merely being
implemented differently.

## 3. What must not change

- **Criterion 2 stays closed.** No new `atomic_replace` on a durability-bearing path, no name created
  outside `init`. A Windows implementation that reintroduces either has reversed RFC 102.
- **The nine `DurabilityContract` guarantees are guarantee-named, not syscall-named** (DC-76). Windows
  implements the guarantees; it does not get its own weaker contract.
- **DC-41's crash matrix is portable** — DC-81 established this. Its assertions are guarantee-level and
  its failure-injection seams are thread-local, not OS-specific.
- **`none.rs` stays.** Platforms that are neither Linux, macOS nor Windows must keep failing at runtime
  with a clear error, not become a compile error.

## 4. Prerequisites — report before any design

**§4.1 is blocking. The rest can proceed in parallel but not past it.**

1. **`MutationRoot` on Windows.** What replaces the retained directory handle, and what anchoring
   guarantee does it actually provide? If it is weaker than the POSIX one, say so and say what follows —
   a weaker anchor may still be acceptable, but only stated, never discovered.
2. **The dependency question.** Exactly which Windows APIs are needed, and whether any are reachable
   through `std`. If a new crate is required, name it and its transitive footprint — this is a
   `boundary-check`/DC-51 matter before it is an engineering one.
3. **The two impossible methods.** Proposed Windows semantics for `durable_directory_entry` and
   `set_permission_bits`, with the argument for why worktree-only makes each acceptable, and what test
   demonstrates it.
4. **The verification problem, which DC-81 says shapes everything.** Windows mutation is untestable on
   the developer's own machine. What does a Windows mutation CI job run, and what does its absence
   during development mean for how the work is sequenced? Yesterday's macOS symlink defect — invisible to
   eleven local gates, caught only by CI — is the argument for taking this seriously.
5. **What DC-37's supersession actually says.** It is the standing Linux-only ruling and cannot be
   contradicted silently. DC-49's own text still cites it as making mutation "definitionally unsupported
   off Linux," which is already stale for macOS.

## 5. Acceptance criteria

1. **Repository mutation works on Windows** — `init`, `commit`, `seal`, `branch`, `tag`, `merge`,
   `trust`, `compact`, `unlock`.
2. **A Windows mutation CI job**, modelled on the macOS one, green.
3. **Criterion 2 still closed for the repository**, checked on the Windows path specifically.
4. **DC-41-grade recoverability re-earned on Windows** at the current state count.
5. **The two impossible methods behave as argued**, with a test for each.
6. **DC-37 superseded explicitly**, and DC-49's stale premise corrected.
7. **`docs/src/reference/` no longer says mutation is Linux/macOS only** — three pages currently do.
8. Full gate set per `EXECUTION-ORDER.md` §6 rule 9, plus green CI on **all three platforms including
   Windows mutation**.

## 6. The cost, honestly

**This is the larger half of the Windows story, not the smaller one.** RFC 102 was six stages of removing
the obstacle; this is building on the cleared ground, and §2 is why it is not merely mechanical.

**What it changes immediately, before any of it is built:** the three reference pages that say Windows
mutation "remains unimplemented" stop being a permanent posture and become a schedule. That is the same
transition RFC 102 §9 recorded for *never* → *not yet*, one step further along.
