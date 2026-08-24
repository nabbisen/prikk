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

## 4. The question the reset must answer

**Not "how should the oracle be defined?" but "what does *this* project need a release policy to do?"**

Answered at prikk's actual scale: one maintainer, no production users, unofficial releases, a
five-crate shipped surface, and a release procedure that is now propose → authorize → execute.

**Then build only that.** Whatever survives should be justifiable by what it prevents *for this
project*, not by what a release policy ought to include.

**Expect the answer to be much smaller.** That is the point of the reset, not a risk of it.

## 5. Non-goals

- **Not blame.** §2 — the work was competent and built for a different project.
- **Not "delete all checks."** Some are load-bearing at any scale: the dependency boundary on a CLI
  with zero third-party dependencies, the unsafe boundary, packaging contents. **§7.2 decides which,
  by asking what each prevents here.**
- **Not a release-procedure change.** The 2026-08-23 grant governs.
- **Not RFC 118's stages** — a sibling application of the same principle.

## 6. What the author-review ceiling leaves unchecked

**The architect measured the apparatus, accepted the owner's framing, and wrote it up.** The
measurement is a fact; **the inference — that size relative to the product is the right lens — is a
judgment nobody has tested.**

**The specific risk: some of that apparatus may be genuinely load-bearing and merely look
disproportionate.** §7.2 exists to test that check by check, rather than by the aggregate number in §1.

## 7. Blocking prerequisites

1. **What do the 154 oracle cases assert, and is any of it recorded nowhere else?** **If they encode
   real policy decisions, recovering those is the first work, not the last** — retiring the oracle would
   otherwise destroy the only record of what the policy is.
2. **Check by check: what does each prevent, for this project, today?** `boundary-check`'s eleven
   categories, `reference-check`, `release-notes`, `differential-check`. **A check that prevents nothing
   here is a candidate for removal regardless of how well it is built.**
3. **What is `differential-check` for with one implementation?** It compares two.
4. **Does DC-94 survive the reframing, or is it withdrawn?**
