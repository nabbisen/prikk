# RFC 102 §6.1–§6.2 — Prerequisite Ruling v1

**Reviewing:** `prikk-rfc102-prerequisite-6.1-v1.md` and `prikk-rfc102-prerequisite-6.2-v1.md`.

**Both accepted. No stop-and-report: RFC 102 survives where RFC 101 died.** Three rulings follow — one
resolving an ambiguity in my own §3, one amending a constraint their work showed incomplete, and one
categorising a disclosure buried in §6.1 that deserves to be visible.

## 1. What §6.1 actually established, and it is not what the RFC assumed

The survey's finding is sharper than "the direction is sound":

> **Every comparable system converged on a bounded-container shape — revlogs, journal/WAL files,
> packfiles — and none of them claims a Windows primitive that makes the *remaining, rarer* new-name
> creation durable.**

So the direction is not naive; it is what SQLite/Fossil, Git and Mercurial each arrived at
independently. **But "shrinks the surface" is not "closes the gap,"** and they are right that RFC 102
implied the latter by analogy to systems carrying the identical open question.

**Ruling: RFC 102's claim is restated, and it is narrower than §3 implies.**

> The container model does **not** find a Windows primitive for new-name durability. It reduces
> new-name events to a **fixed, enumerable set created once at `init`** — and `init` is idempotent and
> retry-safe, so a crash there loses no history and the remedy is to run it again.

That is RFC 101 §5.3's T1 finding, which I ruled, applied to a small set of names instead of one. **It
closes the gap only if the set is genuinely fixed.** Stated this way it is honest and still sufficient;
stated as "containers solve Windows durability" it is false.

## 2. Ruling on the ambiguity they found in §3

They flagged that *"bounded set"* is unresolved between two shapes with different answers: a fixed small
set of names each unbounded in size (Fossil/SQLite), or periodically-rotated size-capped segments (Git
packfiles), where **each rotation is a new-name event** — rarer, not absent.

**Ruled: fixed set of names, each unbounded in size. Rotation is forbidden.**

Rarer is not never, and prikk's standard is invariants rather than probabilities. A design whose
durability degrades every N megabytes is a design that fails eventually and unpredictably — the shape of
failure this project consistently refuses.

**And the consequence they did not reach, ruled now because it constrains the design:** unbounded growth
needs compaction, and compaction naively writes a new container — a new name, reintroducing the problem
at the worst moment. **Compaction must target a pre-created alternate slot** — fixed A/B names,
allocated at `init` like every other container. Any design that creates a name after `init` fails
this RFC's own premise.

**All container names are created at `init` or the design is wrong.** That is now the acceptance test
for §6.3.

## 3. Their blast-radius finding shows my constraint 5 was incomplete

§6.2 §"Second" observes that today's one-file-per-object model has an isolation property: every object
is independently content-hash validated, so corruption is confined to one object and `verify` names
exactly which. §6.1's Git research supplies the contrasting evidence — *"if a packfile gets corrupted,
you might lose access to hundreds of objects at once."*

**They are right, and my constraint 5 does not catch it.** It says recoverability must not regress below
DC-41 Stage 1's audited **24/24 reachable states** — a *coverage* measure. Blast radius is a *severity*
measure. A container could hold state coverage at 24/24 while converting single-object damage into
multi-object loss, and constraint 5 as written would be satisfied.

**Constraint 5 is amended**, and this ruling is its authority until the RFC is edited:

> Recoverability does not regress below DC-41 Stage 1's audited 24/24 reachable states, **and corruption
> isolation does not regress**: a single corruption event must remain attributable to, and confined to,
> a single object. Per-entry checksums or an equivalent isolation mechanism is a **requirement of any
> proposed container format**, demonstrated rather than asserted.

**This is the most valuable thing in either report.** It is a real regression risk that the RFC's own
constraints would have permitted, found by doing §6.2 carefully rather than by answering it.

## 4. The SQLite disclosure deserves to be visible, and categorised

§6.1 quotes SQLite's own documentation: *"FlushFileBuffers() can be completely disabled using registry
settings on some Windows versions."*

That is buried in a subsection and it undercuts the foundation the container plan rests on — **content**
durability, which DC-87 Stage 2 established Windows does provide.

**Ruled: this is a configuration hazard, not an architectural one, and the two must not be conflated.**
A platform that *lacks* a primitive and an administrator who *disables* one are different categories —
the latter has a direct POSIX analogue in `eatmydata` and equivalent mount options, and prikk does not
treat those as invalidating Linux durability. It does not change RFC 102's viability.

**But it is not nothing.** §6.5 or §6.6 should state whether prikk can detect the condition, and if it
cannot, that belongs in the platform documentation rather than in silence. Carry it as an open item.

## 5. On the difference between this result and RFC 101's

Their closing explanation is right and worth endorsing: RFC 101 targeted one transition while leaving
the decisive one (object publication) outside its stated scope, whereas RFC 102 targets the storage
model, **which is what T2 is made of**. The different outcome is structural, not a difference in care.

Two smaller things done well: confirming no production code changed since the §5.2 table was built,
rather than assuming the facts still held; and reporting T4 and T8 as *possibly eliminable* rather than
merely routable, while flagging both as design claims this investigation could not settle. **Naming the
stronger result and declining to bank it is the right shape.**

## 6. Standing

- **§6.1, §6.2: accepted.** No stop-and-report.
- **§6.3 next**, and it now has a hard acceptance test from §2: enumerate the container set and show
  every name is created at `init`. Any name created later fails the RFC's premise.
- **§6.4, §6.5, §6.6** follow. §6.5 carries T12 and is the one I flagged as most likely to be waved
  through; §6.6 must cost the per-entry-checksum requirement from §3.
- **RFC 102's §3 and constraint 5 are amended by this ruling** and should be edited into the RFC before
  design begins.
- T11's classification remains open and is prior to storage architecture, as they say.
