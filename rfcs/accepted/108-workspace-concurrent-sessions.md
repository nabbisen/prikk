# RFC 108 — Workspace: concurrent sessions in one physical project

**Status.** **ACCEPTED by the project owner 2026-08-27**, on the design recorded below. Authored by the
project owner 2026-08-18 as a concept; the owner ruled §6 — **Workspace patches are unsealed** (§6.2) —
on 2026-08-27, and the architect's design (D1–D5) follows from that ruling.

**A design now exists.** Implementation is scoped by handoff, and D5's recommended first increment is
the mechanical one: generalise `active/<name>/` for the WAL and the active lock, with no CLI surface
and no Workspace concept exposed. **D5 also names what this design deliberately does not settle** —
naming and CLI surface, whether a Workspace may be shared, and the RFC 109 interaction.

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

### 6.2 Owner's ruling, 2026-08-27: **unsealed** — seal late

**Workspace patches are unsealed.** A Workspace is a private staging area; work crosses into permanent
history only at integration.

**This reverses the owner's own 2026-08-18 instinct (§6.1), and the three costs priced there are why
it should.** Sealing early makes every wrong turn permanent in a tool with **no amend, no rewrite, and
no force-push**; an abandoned Workspace's blocks would remain in the repository indefinitely,
unreachable and unreclaimed, since `prikk compact` reclaims records from three containers and **not**
blocks or objects. **The decision test in §6 answers itself once stated: a user must be able to throw
away "let me try something" without trace, and sealing early makes discarding impossible.**

**It also avoids a cost that would have landed on a badge criterion.** `verify` is O(N³); per-Workspace
sealed history multiplies N. **Three times the history is roughly twenty-seven times the verification
cost**, which would have made criterion 3 pressing rather than theoretical. Seal-late does not incur it.

### 6.3 What the ruling forces — and it is cheaper than §6 feared

§6 states the consequence: unsealed stacks are WAL records, **and the WAL is repository-scoped**, so
concurrent Workspaces need either several WALs or a WAL that is no longer repository-scoped.

**Verified against the code, the layout already anticipates this:**

```rust
pub fn active_dir(&self) -> PathBuf { self.prikk_dir.join("active") }
pub fn default_active_dir(&self) -> PathBuf { self.active_dir().join("default") }
```

**`.prikk/active/<name>/queue.wal` is already the shape** — with exactly one name, `default`, in use,
and `Wal::for_layout` hardcoding both it and the relative literal `"active/default/queue.wal"`.

**So this is not a repository-topology problem after all.** The directory structure accommodates
several actives today; what is hardcoded is the *choice* of one. **That is a materially smaller design
than §6's framing implied, and it is the strongest argument that the ruling is affordable.**

### 6.4 What a design must now answer

**These are the next questions, and none is settled by the ruling:**

1. **Does `Wal` become workspace-scoped, or does a Workspace own a `Wal`?** The type already carries a
   `layout` and a mutation root; which of those generalise is the first mechanical question.
2. **What serialises, and when.** §6.1's finding stands: sealing takes a repository-wide `ActiveLock`
   (`seal.rs:81`). Under seal-late that lock is hit **only at integration**, which is the point of the
   ruling — but the design must say what concurrent Workspaces contend for in between.
3. **Crash safety is now load-bearing for Workspace data.** Unsealed work is protected *only* by the
   WAL. Invariant 7 of §5 — *"crashes must not silently destroy recoverable Workspace changes"* — is no
   longer a general aspiration; it is the guarantee this ruling depends on.
4. **What `verify` says about a Workspace.** Unsealed work is outside sealed history by construction.
   Whether `verify` reports it, ignores it, or refuses to comment is a user-facing decision.

**Unchanged by this ruling:** §5's ten safety invariants, and §4's three candidate physical
architectures — the ruling constrains *when work becomes history*, not *how trees are materialised*.

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


---

# Design, 2026-08-27

**Authored by the architect on the owner's ruling that Workspace patches are unsealed (§6.2).**
**This is the design §5–§7 called for. It does not grant implementation authority; a handoff does.**

## D1. Architecture: A, with C's identity model

**§4's B — shared-base copy-on-write / reflink trees — is eliminated on evidence.** Reflink needs
btrfs, XFS, or APFS. **Windows NTFS has none.** Criterion 6 is met precisely because all three
platforms mutate equally, and `prikk-store` is confined by DC-51 to `getrandom` and `rustix`. **B
would make Workspace a Linux-and-macOS feature and un-meet a banked criterion.**

**Between A and C, §2 has already ruled the user-facing half**: *"a simple materialized environment is
preferred wherever it provides the required guarantees"*, and *"the physical tree is an implementation
mechanism required to give ordinary build tools ordinary semantics."*

**Chosen: A — independent materialized trees — with C's definition of identity.** A Workspace *is* its
patch composition; the tree is the environment that composition renders into. **prikk must be able to
name, list, and reason about a Workspace without materialising it**, which `verify`, any listing
surface, and any agent-facing interface will need.

**Why not C wholesale:** the concept requires building and testing concurrently, which needs real files
simultaneously. C must therefore materialise every *active* Workspace anyway, converging on A wherever
work is happening, and differing only for dormant ones — at the price of a render step or first-access
latency in exactly the workflows §2 says the tree exists to keep ordinary.

## D2. The mechanism: `active/<workspace>/`, and it already exists

**The WAL and the lock share one hardcoding, so they generalise together:**

```rust
layout.default_active_dir()       -> active_dir().join("default")
layout.default_queue_wal_path()   -> default_active_dir().join("queue.wal")
layout.default_active_lock_path() -> (same "default" parent)
Wal::for_layout(layout)           -> hardcodes "active/default/queue.wal"
ActiveLock::acquire(layout)       -> hardcodes the default lock path
```

**`.prikk/active/<name>/` is already the shape.** Exactly one name is in use. **This is the single
mechanical change the ruling forces** — not a repository-topology redesign, and not two changes.

**`ActiveLock` already has a second, ref-scoped constructor** (`lock.rs:67`), so more than one lock
granularity is already an accepted idea here.

## D3. Answers to §6.4

1. **A Workspace owns a `Wal`; `Wal` becomes workspace-scoped.** `Wal::for_layout` gains a workspace
   name rather than `Wal` learning about Workspaces. **The layout stays the authority on paths**, which
   is the existing division of responsibility.
2. **What serialises:** editing, building and testing serialise on **nothing** — separate trees,
   separate WALs, separate active locks. **Integration serialises**, because sealing takes the
   repository-wide lock, and that is correct: seal-late means the bottleneck is hit once per
   integration rather than once per commit.
3. **Crash safety is load-bearing and must be stated as such.** Under seal-late, unsealed Workspace
   work is protected *only* by its WAL. **Invariant 7 stops being an aspiration.** A design increment
   must show a Workspace's WAL recovering independently of every other.
4. **`verify` reports Workspaces as out of scope, explicitly.** Unsealed work is outside sealed history
   by construction; `verify`'s claim is about sealed history. **Silence would be the wrong answer** —
   the project's own rule is that absence must be explicit. A named line saying "N workspaces, not
   verified here, by construction" is the shape.

## D4. §5's ten invariants — what already holds

**Six are satisfied by D2's mechanism plus existing machinery, and a design that re-solves them wastes
effort:**

| # | Invariant | Status under this design |
|---|---|---|
| 1 | Multiple Workspaces active concurrently | **D2** — separate actives |
| 2 | One cannot overwrite another's state | **D2** — separate trees, separate locks |
| 3 | Ordinary ops cannot modify the canonical base | **Already true** — the base is sealed history; unsealed work cannot alter it |
| 4 | Each can build and test independently | **D1** — real trees, ordinary tool semantics |
| 5 | Same-file edits stay independent until integration | **Already true** — patches are content-anchored, not tree-diffed |
| 6 | Build state cannot corrupt another | **D1** — build output lives in the tree, not `.prikk` |
| 9 | Publication and merge are explicit | **Already true** — `seal` and `merge` are commands |

**Four need real design work:**

- **7 — crashes must not silently destroy recoverable changes.** D3.3.
- **8 — patch creation and state transitions are atomic or recoverable.** The WAL provides this per
  active today; the transitions *between* Workspace states are new.
- **10 — Workspace state is inspectable and auditable.** This is where C's identity model pays: a
  Workspace's composition is inspectable without materialising it.
- **Workspace lifecycle itself** — creation, abandonment, and integration are not in §5 but are the
  operations that make the invariants meaningful. **Abandonment is the one the ruling makes cheap**:
  unsealed work discards without trace, which was §6's decision test.

## D5. What this design does not settle

- **Naming and CLI surface.** Not designed here.
- **Whether a Workspace may be shared or only exists locally.** §2 implies local; it is not ruled.
- **Interaction with RFC 109's agent-native interface**, which may want Workspace as a primitive.
- **The first increment's scope.** Generalising `active/<name>/` is the smallest useful thing and is
  separable from every user-facing surface.

**Recommended first increment:** generalise `active/<name>/` for the WAL and the active lock, with no
CLI surface and no Workspace concept exposed — **a mechanical change with existing tests as its
control**, which de-risks everything above it before any user-facing decision is made.
