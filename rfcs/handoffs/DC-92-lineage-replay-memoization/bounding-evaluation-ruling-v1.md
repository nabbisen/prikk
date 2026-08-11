# DC-92 — Bounding Evaluation Ruling, and Clearance to Implement

**Reviewing:** `.git-exclude/review-request/prikk-dc-92-bounding-evaluation-v1.md`, commit `5eee2de`.

**Evaluation accepted. §4.2 is cleared to implement.** This report corrects me twice, and both
corrections stand — §1 and §2 are those, recorded before anything else.

## 1. Correction: I amplified an unmeasured claim, and the measurement does not support it

Their prior report flagged the `TextCache` content term as unmeasured. **I did not merely accept that —
I amplified it**, writing that 599 MB was "a floor, not the figure," and amending DC-92's criterion 7
to say the content term was excluded "so that figure is a floor."

Measured: the edit-heavy variant shows **no increase at all** — slightly *lower* than churn at every
point, within single-trial noise. At `FILE_SIZE_BYTES = 64`, 160 edited nodes add ~10 KB against an
index term of ~65,000 KB.

**My amplification was unsupported.** The honest residual is narrower and theirs, not mine: the content
term is real in principle, scale-dependent, and unmeasured at realistic file sizes — not a floor under
the measured number. Criterion 7 is corrected accordingly.

They corrected their own caveat and called it humbling. It is worth saying plainly that reporting a
measurement that undercuts your own earlier framing is the behaviour this project needs most, and it is
the second time this cycle they have done it.

## 2. Correction: my costing of lead (a) was wrong, in their favour

I wrote that lead (a) required "an extra full read of every Block object" and argued the arithmetic was
favourable because that pass is O(N).

**There is no extra read.** `verify_block_payload` (`verify.rs:370`) decodes `BlockPayload` at line 376
and then performs the parent-existence checks — the payload is already decoded in the very function
that would be split. Retaining it in a side list costs O(N × small) memory and **zero** additional I/O.
`BlockPayload` scales with patches-per-seal, not tree size.

My argument reached the right conclusion through a wrong premise, and they caught the premise. The trade
is better than I claimed, not merely favourable.

## 3. Verified

- **`state_derivation_parent` is single-parent even for `Merge`** — it returns `mainline_parent_id`, and
  otherwise the first parent. So the memo's dependency structure is a **tree/forest, never a
  multi-parent DAG**, regardless of what `parent_block_ids` carries. That is a real simplification and
  I had not noticed it.
- **`merge_evidence::topological_order` is Kahn's algorithm** — `VecDeque`, `pop_front`, `push_back`,
  cycle detection on a length mismatch. The precedent claim holds; this is established machinery, not a
  new algorithm.
- **`verify_prefix_dir` sorts by encoded filename bytes**, confirmed again — ObjectId order, lineage-
  independent.

## 4. The synthesis neither report states: §4.2 bounds the content term too

§4.4 leaves the content term unmeasured at realistic file sizes, and §4.3 was the route that would have
capped it. **Under §4.2 that stops mattering.** If the frontier is 1 for a linear history, then only
frontier entries exist at all — and each entry is a `(NodeLifecycleState, TextCache)` pair. Bounding the
number of live entries bounds **both** halves, whatever the file size.

So §4.2 is not merely the better route for the index term; it is the route that also retires the
open question §4.4 could not close. That is worth knowing before choosing.

## 5. Ruling on the error-ordering risk (their risk #1)

**Acceptable, and criterion 5 is clarified rather than waived.**

Which of several *independent* defects surfaces first was never a specified property. It is determined
today by ObjectId order — a content hash, uncorrelated with anything an operator could reason about.
Both orders report a real defect and both fail closed. Criterion 5 reads, as they propose: **the same
finding for the same single-defect scenario.**

Naming it rather than trusting the suite to have covered it was right, and their observation that every
DC-92 negative control constructs exactly one defect is exactly why it needed naming.

**It does expose something worth recording separately, which is not DC-92's to fix:** `verify`
propagates the first hard error via `?` rather than accumulating findings, so a repository with several
independent defects takes N runs to enumerate them. For the command whose completion is the product's
central claim, "here is one problem" versus "here is every problem" is an operational difference.
Registered in `FINDINGS.md`; not this increment's.

Risks 2, 3 and 4 are accepted as stated — shape/schema still runs and still fails closed; `block_seals`
stays in Phase A with its scan-order contract intact; format-1 blocks never enter Phase B.

## 6. §4.3, and the failure mode they traced

Their reasoning is correct and better than my framing: under ObjectId-ordered iteration, lineage
locality is destroyed by construction, so an LRU memo would miss the specific ancestor each walk needs,
fall back toward genesis, and **silently reintroduce a meaningful fraction of the time regression while
looking fixed** — memory bounded, tests green, only wall-clock worse. That is a worse failure mode than
an honest partial fix, and identifying it with a mechanism rather than deferring to my ruling is the
right way to have disagreed with me if the mechanism had gone the other way.

Endorsed: not the primary route; cheap defense-in-depth *after* §4.2, where dependency-ordered visits
make the most-recently-used entry the one about to be needed.

## 7. Cleared to implement §4.2 — with the tests they proposed, plus one they did not

Their proposed test set is right and I am adopting it: a multi-branch frontier-bound test that asserts
**boundedness itself, not merely correctness**; a cycle-detection control for Phase B's Kahn's path,
which is a different code path from `validate_v2_lineage`'s walk; and a re-run of all four existing
negative controls confirming identical messages survive the restructuring.

**One they did not propose, and it is the one I would not skip: re-run the timing axes after the
restructuring.** The O(N³) → O(N) result was measured against the *pre-phasing* structure. Splitting
`verify` into two phases changes the loop that produced those numbers, and a memory fix that quietly
costs back the time win would be the mirror image of the defect this round exists to fix. The harness is
committed; re-running it is nearly free.

**Also required:** re-run the memory axes, and report the frontier's measured peak against both axes —
the claim is O(1) for linear history, and that is a curve, not an assertion.

## 8. Standing

- **DC-92: §4.2 cleared to implement.** Merge still blocked until the bound is in and both axes are
  re-measured.
- Green **three-platform** CI before merge.
- DC-91 remains proposed and awaiting the owner. Nothing else is assigned.
