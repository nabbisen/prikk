# `data-model.md` — the format-1 verification sentence: implementation handoff

**Base:** current `main` (`f69779c`, CI + Docs green). **Under `003-landing-work-on-main.md`.**
**Origin:** found and reported by the recovery-reference increment (`f69779c`), correctly not touched
because that handoff scoped it out.

**One file, two defects, both small.** Issued separately rather than folded into the last increment
because that one's scope boundary was explicit and honouring it was right.

---

## 1. The defective sentence — `data-model.md:43-44`

> *"**Format-1 verification** preserves older structurally readable bytes and reports malformed shape,
> duplicate, or non-canonical ordering as warnings instead of rewriting them."*

**It describes a diagnostic layer that no longer exists.** `f69779c` established this against the code:
RFC 103 retired format-1, a format-1 repository is refused at `RepositoryLayout::open` before any
command runs, and `signature_diagnostics.rs`'s own module doc says the layer is *"provably unreachable
through `verify_repository`'s pipeline."*

**Why this sentence was easy to miss — worth understanding before rewriting it.** There are **three
different things called "format N"** in this project, and `layout.rs:44` says so in its own doc comment:

| Axis | Numbering | Names |
|---|---|---|
| **On-disk repository layout** | 1 … 6 | loose files vs. RFC 102 containers — **this is "format-1"** |
| **DC-40 wire schema** | "format-2" | Block/Patch shape and Merkle rules — *"keeps its own name regardless"* |
| **`schema_version`** | per object type | `Tag` schema 1, `Block` schema 2 |

**The sentence welds the layout axis onto signature-envelope behaviour**, which belongs to a different
axis. That is the same mis-axing that made `integrity-recovery.md`'s *"format-1 signature-envelope
diagnostics"* wrong. **Do not reproduce it in the replacement.**

**Adjudicate, do not sweep.** The rest of that paragraph — Ed25519 must be 64 bytes, duplicate tuples
rejected, ordering by key-id / signer-role / algorithm / signature bytes, advisory timestamps not
affecting order — **is current and true. Verify it, then leave it.** Only the last sentence is defective.

## 2. The second defect — a reference page narrating its own correction history

**`data-model.md:48`:**

> *"...— this page's claim otherwise is **stale and corrected here**."*

**A reference states what is true. It does not report on its own edits.** This will age worse than the
error it replaced: a reader a year from now learns that something *was once* wrong here, about a claim
they cannot see, and cannot tell whether the note still applies.

**Landed in `828e631`, and I accepted it** — this is a miss in my review, not a dev-team error.

**Fix:** state the fact plainly. *"Tag objects are produced by `prikk tag create` and `sync adopt-tag`
(RFC 117)."* **Keep the Attestation contrast** — that half is true and load-bearing, and it is the
sentence's actual point.

**Check for the pattern, not just the instance.** I found one occurrence across both data-model files;
**re-derive that yourself** and report what you searched for.

## 3. Also adjudicate — do not assume

The anchor table cites **DC-39** at `data-model.md:227` and `:235`. **DC-39 added both** the strict
new-envelope rules (**current**) and the format-1 signature diagnostics (**dead**). Row 227's own claim
— *"strict new envelopes enforce Ed25519 shape, tuple uniqueness, and canonical order"* — **reads as
current to me, but that is a reading, not a verified fact.** Check both rows: is the *claim* still true,
independent of what else DC-39 once added?

**Precedent from `f69779c`:** a citation to work that is partly retired is fine as long as the claim
being cited is still true. **Do not delete a row for citing DC-39.**

## 4. Out of scope

- **Everything except `data-model.md`.** `data-model-lifecycle.md` was checked clean by `f69779c` for
  this defect — **re-check it and report**, but do not edit it unless you find something.
- **The three recovery reference pages** — just corrected at `f69779c`.
- **No code.** If a doc turns out right and the code wrong, **report it.**
- **`MILESTONES.md`, `ROADMAP.md`, the badge.**

## 5. What to report

1. **§1's replacement** — what you removed, what if anything replaced it, and the authority.
2. **Your verification that the rest of the paragraph is current** (§1) — each clause.
3. **§2's rewrite**, and **what you searched for** to establish the self-narration is a single instance.
4. **§3's two rows adjudicated** — `CURRENT` or `DEFECTIVE`, with the authority. **A `CURRENT` verdict
   is as much of the deliverable as a fix.**
5. `data-model-lifecycle.md` re-check result (§4).
6. **Full gate set against the exact commit, after the last edit**, plus `mdbook build`.
7. Test counts — **expected unchanged**.
8. Anything here that was wrong. **My §1 three-axis reading and my §3 "reads as current to me" are both
   mine to be wrong about.** The last two increments each corrected one of my scope guesses — **`f69779c`
   found my "may be perfectly current" guess wrong in the under-fixing direction.** Assume the same here.

**Stop and escalate, do not guess**, if: removing the sentence leaves the paragraph making a claim it no
longer supports; an anchor row's claim turns out false rather than merely citing retired work; or you
find self-narration of the §2 kind in a page this handoff does not name — **that would be a pattern, and
its own increment.**
