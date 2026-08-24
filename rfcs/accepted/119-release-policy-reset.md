# RFC 119 — Release policy tooling: reset

**Status.** **ACCEPTED by the project owner 2026-08-24.** The owner's ruling: DC-45's assets *"do not
match our reality, the currency and our perspective"*, and **the previous architect *"did not understand
the project perspective — it was just their 'ideal' product, far from the project reality."*** Rewriting
design and scripts is authorized. **§7's prerequisites precede design.**

**Independence.** Author-reviewed — the standing ceiling. **§6 states what that leaves unchecked.**

**Supersedes.** DC-45's outstanding obligations. **Reframes** DC-93 and DC-94.

---

## 1. The measurement

| | Size |
|---|---|
| `tools/release-policy` (Rust) | **6,970 lines** |
| `release/` Python | **2,895 lines** |
| `release/` JSON artifacts | **1.66 MB** |
| **`prikk-cli` — the product it gates** | **4,981 lines** |
| Signers the policy authorizes | **none** — `authorized_primary_fingerprints = []` |

**The release-policy apparatus is larger than the product it governs**, by any measure, and it
authorizes nobody to release anything.

**That is the finding.** Not a defect in any check, and not a mistake in any single decision.

## 2. What was built, and why it does not fit

**This is a rigorous, well-engineered release-governance system for a mature, multi-party project
shipping to production users under an official-release regime with a signer authority.**

**prikk is a single-maintainer, pre-1.0 project with no production users, no official release, and an
empty signer allowlist.** It has never entered the regime the apparatus governs.

**The apparatus is not wrong in itself. It is wrong *here*** — an ideal product's release policy,
applied to this project's reality. **Nothing in it was built carelessly; it was built for a different
project.**

**The same pattern is visible elsewhere and the owner has already been trimming it:**
- **Criterion 4's two-natural-persons signer rule** — ruled *"not wrong and just too early to be
  applied."*
- **The three-authority release-lane transition** — superseded 2026-08-24 by a proposal-authorize-execute
  procedure that matches how releases are actually cut.
- **The official-release boundary** — machinery for a regime prikk has never been in.

**RFC 119 applies the same judgment to the tooling.**

## 3. One symptom worth keeping, because it shows the shape

`tools/release-policy/src/oracle/verify.rs:23`:

```rust
const OBSERVATIONS_PATH: &str = "release/oracle/python-observations-v1.json";
```

**The standing gate validates the Rust implementation against recorded observations of the Python
harness it replaced** — so its definition of *correct* is *"matches what the Python did."*

**This is evidence, not the diagnosis.** It shows the apparatus reasoning about **itself** rather than
about prikk: a correctness criterion defined by an internal predecessor, with no statement of what the
policy asserts about a release. **A system sized for its own consistency rather than for its subject.**

**And it explains why DC-93 and DC-94 stalled.** Deleting the Python was never blocked by the files; it
was blocked by what defines correctness. DC-94's map binds Rust categories to *Python* categories, which
is why its own prerequisite could not answer *"what is an executed check registry?"*

## 4. The method: subtract, do not rewrite

**Owner's ruling, 2026-08-24, narrowing this RFC:** *"I do NOT want to either replace the whole codebase
or edit it widely. What I want is to omit what was defined as either policy or rule ... but is actually
unnecessary."*

**So this is not a redesign.** The question is not *"what should a release policy be?"* — answering that
invites building a new one. The question is:

> **Which currently-defined policies and rules are unnecessary for this project, and can therefore be
> removed?**

**Everything that survives stays as it is.** No restructuring, no rewriting, no "while we are here."
**The deliverable is a list of removals with reasons, not a new design.**

**Bias toward leaving things alone.** A rule that is merely oversized but does prevent something here
**stays**. Only rules that prevent nothing *for this project* are candidates.

## 5. Where the defined rules live

Enumeration precedes adjudication. **They are not all in one place**, which is part of why none of this
has been reviewed as a whole:

- **`boundary-check`'s eleven categories** — `workspace-members`, `default-members`, `tool-metadata`,
  `lockfile-boundary`, `dependency-boundary`, `dependency-placement`, `unsafe-boundary`, `rfc-naming`,
  `publication-allowlist`, `package-contents`, `source-archive-contents`.
- **The 154 oracle cases** and the manifest defining them.
- **`reference-check`'s** required-live paths and inventory.
- **`differential-check`**, whose counterpart implementation is being retired.
- **`release-notes`'** own assertions.
- **DC-35's signer-authority rules**, including the two-natural-persons requirement the owner already
  ruled *"too early to be applied."*
- **The official-release boundary** in `release-compatibility.md`.

**Some are code, some are documents, some are RFC prose.** A rule removed from code but left asserted in
a document is not removed.

## 6. Non-goals

- **Not blame.** §2 — competent work, built for a different project.
- **Not "delete all checks."** Some are load-bearing at any scale: the **dependency boundary** on a CLI
  with zero third-party dependencies, the **unsafe boundary**, **package contents**. **§7.2 decides each
  on what it prevents *here*.**
- **Not a rewrite of anything that stays** (§4).
- **Not a release-procedure change.** The 2026-08-23 grant governs.
- **Not RFC 118's stages** — a sibling application of the same principle.

## 7. What the author-review ceiling leaves unchecked

**The architect measured the apparatus and accepted the owner's framing.** The measurement is fact; **the
inference that size relative to the product is the right lens is a judgment nobody has tested.**

**The specific risk: something may look disproportionate and be load-bearing.** §8.2 tests that rule by
rule rather than by §1's aggregate.

## 8. Prerequisites — discharged 2026-08-24/25

**All four are answered.** Working documents, in order:
`release-policy-research-v1.md` → `release-policy-derivation-v1.md` (+ stage addendum) →
`release-policy-test-design-v1.md` → `release-policy-reconciliation-v1.md`. **Owner reviewed and
accepted the derivation and the reconciliation.**

1. **What do the 154 cases assert?** Eight suites. **Not homogeneous** — `release-evidence` alone holds
   G4-identity cases, superseded lane-transition cases, governance cases and schema validation. **No
   policy decision was found recorded only there**, but see §10's FINER track.
2. **What does each rule prevent here?** Reconciliation §2-§4, verdict per rule.
3. **`differential-check` with one implementation?** **NEVER** — migration scaffolding.
4. **Does DC-94 survive?** Its map binds Rust categories to Python categories. **Withdraw or reframe
   under §10's outcome**; it cannot stand as written once the Python is not the reference.

## 9. The derived policy

**Pre-1.0: G2 and G3 in full, G1 as declaration, G4 as identity.** Post-1.0 strengthens G1 to prevention
and G4 to authority. **G2 and G3 do not change with stage.**

## 10. Three independent tracks

**Deliberately separate — none blocks another, and they carry different risks.**

**A — Park what is LATER.** 43 signer cases, `publication-allowlist`, DC-35's rules, the official-release
boundary. **All of it runs today.** Parking is unhooking it from today's gates without deleting the
thinking. **Clearly decided; lowest risk; removes a cost paid daily.**

**B — Resolve FINER, then remove NEVER.** `release-evidence`'s 73 cases, `dependency-placement`,
`tool-metadata`/`lockfile-boundary`, `reference-check`. **Every removal decision depends on this**, and
it is where the architect's confidence is lowest — a suite-level verdict would already have discarded
~16 cases prikk needs.

**C — Build G1.** The largest gap: nothing compares a candidate's persisted-data contract against the
released one. `0.23.0`'s break was declared by hand and detected by nothing.

**Sequencing note, recorded against the architect's own bias:** the reconciliation flagged that G1 was
designed for first and then placed at the centre. **Track C is not automatically first**, and the owner
should decide the order rather than inherit that emphasis.

## 11. Non-goals restated

**Not blame** (§2). **Not "delete all checks"** — several are load-bearing at any scale.
**Not a rewrite of anything that stays.** **Not a release-procedure change.**

**And placement is not existence:** `rfc-naming` and `unsafe-boundary` are verdicted *not release
policy*, which is **not** a proposal to delete them.
