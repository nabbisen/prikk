# DC-44 increment 3 — a self-describing bundle manifest

**Authority:** `rfcs/proposed/DC-44-MIGRATION-BACKUP-RESTORE-EVIDENCE.md`, design goal 1.
**Base:** `1c13ade` or later `main`. **Under `003-landing-work-on-main.md`** — commit locally on
`main`, do not push, do not tag.

**This is a bundle format change**, the first since DC-53 Stage 2. §3 is the precedent it must follow.

---

## 1. The honesty gap this closes, which matters more than the metadata

DC-44 asks for a manifest naming *"repository format, included refs/objects, expected digests, tool
version, and explicit exclusions."* **The last item is the one that changes what a backup means.**

**A bundle is one ref's closure.** `export_bundle(layout, ref_name)` takes a single ref, and the CLI
exposes `--ref`. **So "I backed up my repository" is, today, "I backed up one branch"** — and nothing
in the artifact, the output, or the verifier says so. A user with three branches who exported one has
a partial backup that looks complete.

**Nothing currently records what a bundle deliberately does not contain.** That is the gap. A manifest
that states its own exclusions turns a silent partial backup into a stated one — which is DC-44's own
last design goal ("document what backup proves and does not prove") expressed *in the artifact*,
where a restoring operator actually meets it, rather than only on a page they may not read.

## 2. The design input: what `verify_bundle` cannot check today

Increment 1 built the offline verifier first **precisely so this increment would not have to guess**,
so start there. `BundleVerifyReport` today carries `ref_name`, `ref_state_id`, `tip_block_id`,
`object_count`, `author_key_count` — all derived from the objects themselves.

**What it cannot tell you, and a reader needs:**

- **Which repository format produced this** — so whether your build can import it at all is
  discovered by trying.
- **What produced it** — no tool version, no provenance of any kind.
- **What is missing** — see §1. The verifier cannot say "this is one ref of several" because the
  bundle does not know.

**The criterion for every field you add: does it let `verify_bundle` answer a question it currently
cannot, or state a fact a restoring operator needs?** A field that fails both is ceremony, and a
manifest of ceremony is worse than none — it makes a bundle look better described than it is.
**Report the fields you rejected and why.**

## 3. The format bump must follow the documented precedent exactly

`docs/src/reference/release-compatibility.md` §"Bundle Format Transitions" states the rule, and it is
already correct — **do not invent a new policy:**

> Bundles are always **exported** as `PBNDL002` … the bump is fail-closed on the write side: an older
> client meeting a newer bundle refuses it outright with its own hardcoded magic check.
> **`PBNDL001` bundles are still accepted on import** … read what an older client wrote, write only
> the current format.

**So `PBNDL003`: emitted on export, with `PBNDL001` and `PBNDL002` still accepted on import.**

**And note why that read-compatibility is load-bearing, because it is easy to drop:** the same page
records that `PBNDL001` acceptance is what keeps the *repository-format* migration path usable — an
old repository can only be opened by an old build, and that build only ever produces `PBNDL001`.
**Breaking import of an older bundle severs a migration path two documents away from the code you are
editing.**

**This increment must update that section of `release-compatibility.md`** to describe `PBNDL003`.
**That is not DC-44's documentation page** — see §6.

## 4. What you must adjudicate

**4.1 — the manifest's contents**, per §2's criterion. DC-44 names five things; **argue each in or
out.** In particular: object digests may be redundant, since an object's id *is* a hash of its own
bytes and increment 1 already verifies closure references resolve — **say whether a digest adds
anything beyond that, and drop it if not.**

**4.2 — where the section sits in the wire format**, and how a `PBNDL002` decode path stays unchanged.
The author-key section is the precedent for adding one.

**4.3 — what `bundle verify` and `bundle export` report from it.** Increment 1's verifier prints a
short report; the manifest should make it more useful without making it noisy.

## 5. What must not change

- **No signing secrets in the manifest.** DC-44's own design goal says so explicitly.
- **`PBNDL001` and `PBNDL002` must still import.** §3. **A test proving each still imports is not
  optional.**
- **Object bytes and object ids.** The manifest describes the payload; it must not alter it. **An
  object exported under `PBNDL003` must have the identical id it had under `PBNDL002`.**
- **`export_bundle`'s ref selection.** Still one ref. **Multi-ref export is not this increment** — the
  manifest states the limitation, it does not remove it.
- **The durability work.** `durable_output` and the `--force` policy are settled; do not revisit.

## 6. Controls

1. **Round trip.** Export `PBNDL003`, verify it offline, import it, and `verify` the result.
2. **Older formats still import** — one test each for `PBNDL001` and `PBNDL002`, with real bytes, not
   a hand-built approximation if a real one can be produced.
3. **Object ids are unchanged across the bump** — the same repository exported under the old and new
   formats yields the same object ids. **This is the control that proves the manifest is additive.**
4. **A manifest that disagrees with the payload is refused**, by `verify` and by `import`, for the
   same reason — the agreement property increment 1 established must survive the format change.
5. **`verify_bundle` reports the manifest**, and states the §1 limitation where an operator meets it.
6. **Full gate set against the exact final commit**, plus `mdbook build` for the
   `release-compatibility.md` edit.
7. **Per-job CI**, re-derived for this diff.

## 7. What remains after this

**DC-44's documentation page** — what backup and restore prove and do not prove, including external
trust and key custody. **It comes after this increment and not before**, because the manifest changes
both halves of that sentence: what a backup contains, and what a restorer can check. Writing it first
means writing it twice.

**It is a different document from `release-compatibility.md`'s Bundle Format Transitions section**,
which this increment updates. Do not merge them.

## 8. The report

To `.git-exclude/review-request/`. Include §4's three adjudications, **the fields you rejected and
why** (§2), all seven controls quoted, the full gate set, and **anything in this handoff that was
wrong** — including §1's claim that nothing records a bundle's exclusions, and §2's list of what
`verify_bundle` cannot check, both of which I derived by reading `BundleVerifyReport` and
`export_bundle`'s signature rather than by attempting an export of a multi-branch repository.
