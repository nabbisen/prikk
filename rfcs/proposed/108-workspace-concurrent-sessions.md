# RFC 108 — Workspace: concurrent sessions in one physical project

**Status.** **Proposed** — concept under discussion. Authored by the project owner 2026-08-18; recorded
here by the architect with the open questions a design must answer first. **No design exists and
implementation must not start from this record.**

**Tracks.** A new product capability, not a correction.

## 1. The concept

Multiple independent sessions should be able to develop, build, test, and inspect the same project
concurrently, without the user maintaining several conventional checkouts.

> **Multiple Workspaces must be able to remain concurrently active within the same physical project, with
> each Workspace providing an independent project environment to its attached session.**

Sessions are **not** expected to take turns. Three Workspaces may simultaneously edit, build, test, run
static analysis, drive an IDE, or host an AI agent, for hours. **Concurrent activation is the primary use
case, not an edge case.**

The desired qualities: safe (no session overwrites another), robust (crashes and abandoned sessions
recover), convenient (a real environment ordinary tools accept), concurrent (no switching), practical
(complexity justified by benefit), and patch-oriented (changes become prikk patches and flow through
existing validation and publication).

## 2. What this deliberately is not

**Not filesystem virtualization.** prikk does not need several processes seeing different contents of the
same inode. Overlay filesystems and similar mechanisms may impose large implementation and maintenance
costs without proportionate value.

The target is instead **multiple complete project environments, logically belonging to one prikk project,
with isolated mutable state and shared project identity** — and a simple materialized environment is
preferred wherever it provides the required guarantees.

**A Workspace is not another checkout.** The user and prikk reason about *one project, many Workspaces*,
not *many unrelated copies*. The physical tree is an implementation mechanism required to give ordinary
build tools ordinary semantics.

## 3. Logical model

```
Workspace = Base Revision + Workspace Changes + Workspace-local Project State
```

State decomposes into five kinds, and the distinctions matter more than the names:

| Kind | Description |
|---|---|
| **Project base** | The canonical state Workspaces derive from. Not modified by ordinary Workspace activity |
| **Workspace project tree** | The complete tree tools operate on — sufficient to build and test |
| **Workspace changes** | The diff from base: modified, added, deleted, renamed. Representable as prikk patches |
| **Workspace-local state** | Build outputs, temp files, caches, test artifacts, editor state. Not published |
| **Project-wide shared state** | Immutable dependency/compiler/download caches — shared only where provably safe |

**Workspace and Session are separate concepts.** A Workspace is an isolated development environment; a
Session is a process, agent, editor or terminal operating within one. One Workspace may host several
Sessions, and **a Workspace should survive its Session** — a crashed agent leaves a recoverable Workspace,
which is why their lifecycles must be separable.

## 4. Candidate physical architectures — none mandated

- **A: independent materialized trees.** Strongest isolation, ordinary tools work, easy to reason about;
  costs storage and setup time.
- **B: shared-base copy-on-write / reflink trees.** Same user model, potentially far cheaper; filesystem
  capability varies by platform and recovery is harder.
- **C: patch-native workspace with materialization.** Workspace *is* a patch composition, materialized for
  active sessions. Best conceptual fit; materialization and uncommitted-change handling become load-bearing.

**The invariant to hold, whichever is chosen:**

> **Every active Workspace provides its attached session with a complete, independently mutable project
> environment.**

The design should compare implementations against the requirements rather than assume one mechanism is
correct — evaluating concurrent use, isolation, completeness, recovery, performance, storage, portability,
tool compatibility, patch integration, operational complexity, UX, and extensibility. **A pragmatic
compromise is explicitly permitted**: physical duplication with excellent isolation, portability and
recovery may beat an elegant abstraction with far greater complexity.

## 5. Safety invariants the architecture must eventually guarantee

1. Multiple Workspaces may remain active concurrently.
2. One Workspace cannot accidentally overwrite another's project state.
3. Ordinary Workspace operations cannot directly modify the canonical project base.
4. Every active Workspace can build and test independently.
5. Same-file modifications across Workspaces stay independent until explicit integration.
6. Workspace-local build state cannot corrupt another Workspace.
7. Crashes must not silently destroy recoverable Workspace changes.
8. Patch creation and Workspace state transitions are atomic or recoverable.
9. Publication and merge are explicit.
10. Workspace state is inspectable and auditable.

**UX principle: simple by default, transparent when inspected, explicit when shared.**

---

# Architect's analysis — what must be answered before a design

The concept above is coherent and its non-goals are well drawn. What follows is not disagreement; it is
the set of questions whose answers determine what this increment *is*.

## 6. The question that decides the shape: are Workspace patches sealed or unsealed?

The concept says a Workspace holds `Base Revision + Patch Stack + Working Changes`, and never says which
side of prikk's integrity boundary those patches sit on. **Everything else follows from this.**

- **If sealed** — they are real history: signed, immutable, in blocks, reachable from a ref. Then a
  Workspace is close to a *branch*, integration is `merge` (which exists, DC-74/DC-75), and the hard part
  is concurrent writers to shared containers.
- **If unsealed** — they are WAL records. And **the WAL is scoped to one `.prikk`**
  (`Wal::for_layout(&RepositoryLayout)`), so three concurrent Workspaces with unsealed stacks need either
  three WALs or a WAL that is no longer repository-scoped.

**This is the first thing a design must settle**, and it is not a detail — it decides whether this is a
concurrency problem in the container/lock layer or a repository-topology problem.

### 6.1 Owner's direction, 2026-08-18: sealed — and what that costs

The project owner's instinct is **sealed**, on the grounds that it follows prikk's philosophy. It does.
Three consequences follow that a design must price rather than discover, each verified against the code
rather than reasoned from the model:

1. **Sealing takes a repository-wide lock.** `crates/prikk-cli/src/seal.rs:81` acquires `ActiveLock`.
   Concurrent Workspaces do **not** serialise while editing, building or testing — but they do **at the
   moment of sealing**. Probably acceptable, since sealing is brief; but it is where the concurrency
   requirement meets a real bottleneck, once per commit, and the design must say so.
2. **prikk has no amend, no rewrite, and no force-push.** Searched; none exists. **Sealed is permanent.**
   Under this choice every exploratory commit in a Workspace becomes permanent signed history, wrong turns
   included.
3. **Abandoning a Workspace does not undo that.** `branch close` is a state on the ref, not a deletion —
   and `data-model-lifecycle.md:156` notes even that state is silently reopened by a later seal or merge.
   `prikk compact` reclaims dead records from three containers, **not blocks or objects**. So an abandoned
   Workspace's sealed blocks remain in the repository indefinitely, unreachable and unreclaimed.

**The question is narrower than §6 first framed it.** The concept already contains both kinds of state —
`Base + Patch 1 + Patch 2 + Working Changes`, where working changes are unsealed by definition. So the real
question is:

> **At what moment does a Workspace's work cross into permanent history?**

| | Buys | Costs |
|---|---|---|
| **Seal early** (per Workspace commit) | Durable, attributable, verifiable and crash-safe from the moment of commit | Every wrong turn permanent; history grows per-Workspace, and `verify` is O(N³) — **three times the history is roughly twenty-seven times the verification cost**, making badge criterion 3 pressing rather than theoretical |
| **Seal late** (only at integration) | Workspace is a private staging area; only integrated work becomes history | Uncommitted work is protected only by the WAL, and the WAL is `.prikk`-scoped — forcing §7's topology question immediately |

**A decision test that does not require settling philosophy first:** *what should a user be able to throw
away without trace?* A scratch space — "let me try something" — argues for seal-late, because sealing early
makes discarding impossible. A long-lived line of development someone returns to for weeks argues for
seal-early, because permanence is then a feature.

**The concept itself reads long-lived** — §18's "a Workspace survives its session, recover and resume", and
§14's fork/archive/transfer operations. That supports the owner's instinct.

**If sealed is the answer, one question must be answered with it:** can an abandoned Workspace's blocks
ever be reclaimed? Today nothing reclaims them, and a feature that creates unreachable permanent history by
design should say what happens to it after a year of abandoned experiments.

## 7. One `.prikk` or many? — and the answer routes to two different projects

Follows directly from §6, and the concept's diagrams are compatible with both.

- **One `.prikk`, shared.** Then concurrent sealing contends on the four `LockableContainer`s and the
  `ActiveLock`. That is prikk's existing durability and locking surface, and this increment becomes a
  concurrency increment on top of it. Note `import_bundle`'s object writes already have a registered
  concurrency gap (`concurrency-locking.md`) — concurrent Workspaces would make it reachable rather than
  theoretical.
- **One `.prikk` per Workspace.** Then Workspaces are separate repositories, and integration is history
  exchange — **which is badge criterion 1, and does not exist.**

## 8. The strategic observation: this may be a path to sync rather than a detour from it

§14's Workspace-to-Workspace patch transfer is **the sync problem minus the network**. Criterion 1 —
*"two machines can exchange sealed history, and both verify it afterward"* — is recorded as the largest
single gap in the project, unowned.

If Workspaces exchange patches locally, that is the same machinery: export a patch stack from one
repository state, admit it into another, verify what arrives, detect conflict, integrate explicitly.
**Local exchange is the same problem with the transport removed and the trust question simplified** — both
sides are the same user on one machine.

That is worth weighing deliberately. A Workspace increment could either be a large feature that defers the
largest gap, or the increment that builds sync's core and proves it locally first. **Which one depends on
§6 and §7**, and the difference is worth choosing rather than discovering.

## 9. Concrete corrections to the sharing table

The concept lists compiler cache as "potentially shared." For this project specifically:

- **`target/` must be Workspace-scoped.** Cargo takes an exclusive lock on the target directory, so
  sharing it makes concurrent builds *serialise* — defeating §11's parallel-build goal while adding a
  contention point. This is also the largest storage cost per Workspace, and naming it early keeps
  Option A's cost honest.
- **`~/.cargo/registry` is safely shared** — cargo manages its own concurrency there.

**The general rule the table should adopt:** shared state is safe only where the *owning tool* already
guarantees concurrent safety, never merely where the content looks immutable.

## 10. What already exists, so the design does not rebuild it

- **Materialization**: `prikk checkout` already builds a worktree from sealed history. Option C's
  "materialize a complete tree" is closer to current architecture than the document suggests.
- **Change capture**: `worktree_status` and worktree-patch authoring already turn a modified tree into
  patches.
- **Patch transport**: `bundle export` / `bundle import` already move objects between repositories, with
  the trust check DC-85 added.
- **Merge and conflict**: `merge` executes, refuses cleanly on conflict, and seals a structural merge
  record (DC-74/DC-75).

**So a substantial part of §14's operation list has an implementation today.** What is genuinely new is
Workspace lifecycle, concurrent activation, and the isolation guarantees.

## 11. What a design stage must produce before implementation

1. **The §6 answer**, with its consequences traced.
2. **The §7 topology**, and if one `.prikk` is shared, the concurrency analysis against the existing lock
   surface — including whether the `import_bundle` gap must close first.
3. **A per-platform feasibility statement** for whichever physical option is chosen. Reflinks and
   copy-on-write differ sharply across Linux, macOS and Windows, and this project has just spent four
   increments learning that platform claims must be verified rather than assumed.
4. **Storage cost, measured** on this repository, for the chosen option — not estimated.
5. **The failure matrix**: crash mid-materialization, abandoned Workspace, interrupted integration, stale
   lock held by a dead session. Invariant 7 is the one most likely to be violated silently.

## 12. Non-goals to state explicitly

- **Not filesystem virtualization** (§2).
- **Not a networked feature.** Even if §8's connection holds, the transport half stays out.
- **Not a replacement for branches.** How Workspaces relate to refs is a design question, not an
  assumption.

## 13. Is this one RFC? — asked by the owner 2026-08-18

**By this project's own test, it is already more than one; but it cannot be split yet.**

**The test.** RFC 102 was one RFC across six stages and that was correct, because every stage served one
guarantee. **DC-42 was superseded into DC-56, DC-57 and DC-58** after design review found it *"bundled
three unrelated increments."* What separates the two cases is whether **one increment can satisfy the
acceptance criteria**. §5's ten safety invariants span materialization, concurrency, lifecycle, recovery
and integration; no single increment could meet all ten, so none could be accepted against them.

**Why it cannot be split now.** The fault lines depend on §6 and §7, which are unanswered:

- **One `.prikk` shared** → the concurrency piece is an increment on the existing lock surface, and
  Workspace-to-Workspace transfer is a local operation.
- **Many `.prikk`** → transfer *becomes* history exchange, and that piece is no longer a Workspace RFC at
  all. It is the sync RFC, badge criterion 1.

**So the topology answer does not merely inform the split — it determines which pieces exist.** Splitting
first would produce RFCs that each look complete and collectively miss the guarantee: DC-42's failure mode
reached from the opposite direction.

**Expected shape once topology lands** — dependency-ordered, not a menu:

| RFC | Scope |
|---|---|
| **108** (this one) | Umbrella: vocabulary, requirements, and **the ten invariants** |
| **Foundation** | Topology, sealing, relationship to refs. **Blocks every other piece** |
| **Materialization & isolation** | Physical mechanism, per-platform feasibility, measured storage cost, `target/` scoping |
| **Lifecycle** | Create, fork, abandon, recover — and §6.1's block-reclamation question |
| **Transfer** | Or this becomes the sync RFC instead, per §8 |
| **Session model** | §3 already states this should be designed separately |

**The condition that must hold however it splits:** the ten invariants stay here, and **each child RFC
names which of them it discharges.** Otherwise ten guarantees become ten orphans — which is exactly how
DC-70's criterion 3 sat mis-scoped across four releases, satisfied by a mechanism other than the one it
named, with nobody able to tell.
