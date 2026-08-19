# RFC 113 — History import foundations (Git, Subversion, CVS)

**Status.** **Proposed.** Originated by the project owner 2026-08-19: *"I would like to offer migration
tools to migrate from other VCS projects with the history safely preserved… the primary target is Git
projects. In addition, I would like to support Subversion projects and CVS projects in order to help
growth and maintenance of such great ancestors."*

**Recorded with the architect's assessment so the direction is reviewable before anyone designs against
it. No design exists and implementation must not start from this record.**

**Scope.** The **shared** problem, not any one source. Git, Subversion and CVS differ enormously in
difficulty (§5), but they fail against prikk's model in the *same three places*, and answering those
once is what makes three importers possible instead of three separate research projects.

## 1. The central difficulty: prikk records what these systems do not

prikk's model is **operations against node identity**. A Patch says *this node moved, this node's
content changed*. `NodeId` is first-class and carried in sealed history.

**Git has no node identity.** A commit is a tree snapshot; renames are **inferred at read time** by
similarity heuristics and are not recorded. Subversion records copies path-wise but not node identity.
CVS versions files independently with no atomic commit at all.

**So an importer must either infer what prikk requires, or admit it did not have it.** Inference is a
guess, and prikk's entire proposition is that it does not guess about history. **The import cannot make
the source's history more precise than the source recorded it**, and any design that appears to must be
wrong.

**The honest form of this is a recorded confidence, not a silent heuristic**: an imported operation that
was *derived* must be distinguishable from one that was *stated*. Which mechanism carries that is §4's
question; that it must exist is not negotiable.

## 2. The signature problem, and why DC-53 makes it sharper — and easier

`verify` now cryptographically checks every reachable Patch's AUTHOR signature (DC-53, 2026-08-18).
**Imported commits cannot carry valid prikk AUTHOR signatures.** A Git author did not sign a prikk
preimage; even a GPG-signed Git commit signed *Git's* object, not prikk's.

Three options, and two are wrong:

- **Sign imported patches with the importer's key.** This asserts that the importer authored ten years
  of someone else's work. **Rejected** — it is a false statement, and DC-53 exists to make authorship
  claims true.
- **Forge or synthesise author signatures.** Not possible and not desirable; excluded for completeness.
- **Import them unsigned, and let `verify` report them.** DC-53 already produces exactly the right
  outcome for this: **`Unverifiable`** — *"no key material recorded for this `key_id`"*, which passes
  `verify` while being reported, never silently treated as sound.

**The vocabulary imported history needs already exists, and it was built for a different reason.**
Imported history is precisely "present, readable, and not verifiable as authored" — the state DC-53
Stage 1 defined and made visible. **A design should use it rather than invent an import-specific
status.**

**What still needs deciding** is whether the *importer* additionally makes a signed claim — "I imported
this from that source" — which is a true statement it can sign. That is §4.

## 3. What "safely preserved" has to mean

The owner's phrase is the requirement, so it must be given a checkable meaning. **It cannot mean
"verified", because prikk cannot verify what the source never signed.**

**It should mean reproducible and comparable:**

- The import records **what it was made from** — source commit/revision identifiers, source repository
  identity, importer version, and the parameters that governed inference (§1).
- **Re-running the same importer over the same source produces the same prikk objects**, or the
  difference is explainable. Determinism is what lets a third party check the claim at all.
- A reader can always tell **"prikk verified this"** from **"this was faithfully imported and prikk
  verified nothing about its authorship."**

**The third point is the one that will be lost if it is not held deliberately.** Ten years of imported
history that looks like native history, in a tool whose selling point is verifiability, is worse than no
import — it is the manufactured-verification failure RFC 110 §4 already names, arriving through a
different door.

## 4. What a design must decide

1. **Where import provenance lives: attestation or payload.** `payload/attestation.rs` already carries a
   claim *about* a block, as its own signable object, and refs already carry
   `required_attestation_ids`. **An import claim is exactly that shape** — an assertion by a named party
   about history, separable from the history itself. Sealing source metadata *into* patch payloads would
   make hearsay look prikk-verified, which RFC 110 §4 rules against. **This is my recommendation, not a
   ruling** — the design must weigh it, because it also determines what a receiver of a bundle sees.
2. **How derived operations are marked** (§1). Rename inference is the concrete case: Git gives
   similarity, prikk wants identity. What does an importer record when it is 80% confident?
3. **What the importer signs, if anything.** "I imported this" is true and signable; "this person
   authored this" is not. Decide whether an import attestation is required, optional, or absent — and
   what `verify` says about an import with none.
4. **Whether imported history may be sealed at all**, and by whom. Sealing is a maintainer act with a
   verified signature; a maintainer sealing imported blocks is making a real claim about inclusion, which
   is defensible — but it must be a deliberate answer, not a consequence of reusing the seal path.
5. **What the source-side floor is.** Which Git features are refused rather than approximated —
   submodules, octopus merges beyond two parents, replace refs, grafts, shallow clones. **A refusal is a
   better outcome than a silent approximation**, and the list belongs in the design, not in bug reports.

## 5. The three sources are not one problem, and should not be one increment

**Git — hard but tractable.** Content-addressed, atomic commits, a real DAG. The work is identity
inference (§1) and the feature floor (§4.5).

**Subversion — different, not easier.** Atomic revisions help, but branches and tags are *path copies*,
not first-class refs, so branch identity must be reconstructed by convention (`/trunk`, `/branches/x`)
that many real repositories violate. Mergeinfo is advisory and frequently wrong.

**CVS — a research problem with known-imperfect prior art.** There are **no atomic commits**: a
changeset must be *reconstructed* from per-file revisions by clustering on author, message and time
window. This is what `cvs2svn` and successors do, and they do it imperfectly by nature. **A CVS importer
cannot be more faithful than the reconstruction**, and §3's honesty requirement bites hardest here.

**Recommended shape: this RFC's foundations first, then one RFC per source**, in that order, with CVS
last and explicitly permitted to conclude that a faithful import is not achievable and that the honest
deliverable is a lossy one, clearly labelled.

## 6. Where this sits against current work

**Not scheduled, and not next.** It depends on decisions that are open:

- **Badge criterion 2 (format stability)** — an importer writes a great deal of history in one go. Doing
  that before the compatibility promise exists means the largest repositories in the project's life are
  written against an unstated format contract.
- **Badge criterion 1 (sync)** — an imported repository that cannot be exchanged is half a migration.

**Neither blocks recording this, and recording it now is the point**: the design decisions above are
easier to make before an importer exists than after one has shipped and set precedent.

## 7. Non-goals

- **Not a Git compatibility layer.** prikk is not a Git wrapper and does not use `.git/` as storage; this
  is one-way import, not interoperability.
- **Not round-tripping.** Exporting prikk history back to Git is a separate question and not implied.
- **Not authorship laundering.** No mechanism here may make imported history appear natively authored or
  natively verified.
- **Not a promise of completeness.** §4.5's refusal list is a feature.
