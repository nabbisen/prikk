# DC-74 — Sequencing Recommendation: Do Not Sequence Multi-Parent Lineage First

**Responds to:** `implementation-handoff-v1-addendum.md` §3 ("Sequencing is yours... if you judge
multi-parent lineage the cheaper first step, say so and it will be scoped as its own increment.")

**Recommendation: proceed with §3's original scope (single-parent merge execution) now. Do not make
multi-parent block lineage a prerequisite.** The addendum's framing of multi-parent lineage as
"confined to derived-state machinery" undersells it — investigated below — but that is a reason to
scope it carefully later, not a reason to block this increment on it now.

## What I checked

Read every site the addendum named (`patch_replay/read.rs`, `patch_inverse/read.rs`,
`lifecycle_cache/incremental.rs`, `block_state.rs`'s four `.first()` sites, `cache_ladder.rs`) plus
`rollback_preview.rs` and the crate's existing single-parent-rejection tests, to size the actual work
rather than trust either the addendum's estimate or my own prior.

## What's actually true, corrected from the addendum

**The real gate is upstream of every site the addendum listed.** `block_state.rs`'s
`validate_block_v2_shape` currently makes it a **structural error** for a format-2 `Normal` block to
have more than one parent, and for **any** block to be `BlockKind::Merge` — a variant that already
exists in the wire format (code 3) but is rejected outright before parent count is even considered.
Nothing downstream — not `patch_replay`'s or `patch_inverse`'s `single_parent_chain`, not the
incremental cache — can be taught multi-parent until this gate is deliberately opened, and opening it
requires answering a real question, not extending a loop:

**When a block has two parents, what does "the state derived from this block" mean, and against which
parent(s) is it cryptographically verified?** Concretely: does one parent act as an authoritative
mainline that alone is walked and verified (git `-m`-style, the other parent kept only as evidence),
or must the merge block's own `patch_ids` be checked for consistency against *both* independently
replayed parent states? This single decision shapes `block_state.rs`, both `single_parent_chain`
functions, `merge_evidence.rs`'s candidate walks, and `cache_ladder.rs`'s already-reserved-but-empty
`ParentPolicy::Dc13MergeAware` variant. None of it is mechanical "iterate instead of `.first()`" —
every `.first()` site I found is a downstream consequence of the shape gate, not an independent bug.

**Confirming this is real, not hypothetical caution:** `cache_ladder.rs` already carries a
`ParentPolicy` enum with a second variant reserved and named for exactly this ("merge-aware
baselines"), unimplemented, fail-closed today. The project has already recognized this as its own
design question once, in scaffolding, without resolving it. Four existing tests
(`merge_lineage_fails_closed`, two `ParentPolicy::Dc13MergeAware` rejection tests,
`multi_parent_candidate_fails_before_report`, `format2_parent_and_kind_matrix_is_closed`) assert the
current single-parent-only behavior explicitly and would need to change, not just extend.

**And no code anywhere constructs a `BlockKind::Merge` block today** — this is greenfield write-side
design, not only a read-side extension. The wire format being ready (as §3.3 of the RFC correctly
established) is necessary but not close to sufficient.

Overall size: **large**, with one genuinely open design question at its center — closer in shape to
DC-74's own §1 ("four questions, answered before a line of design") than to a mechanical follow-on.

## Why this argues against sequencing it first, not for it

Making multi-parent lineage a prerequisite would mean resolving that open design question — itself
requiring its own investigation-before-design discipline — **before** starting work this RFC has
already cleared. That is a materially different, materially larger undertaking than "the cheaper first
step" the addendum's framing suggested.

Against that: the review's own disposition already separates the two concerns correctly. **The release
condition gates shipping, not building** (`MILESTONES.md`'s attached condition; the addendum's own
words, "Build and merge normally. Do not treat this as a hold"). Single-parent merge execution is
buildable, testable, and mergeable to `main` now — Q1 already proved the core mechanism (adoption)
works correctly and byte-exactly. It simply cannot be **released** until whatever eventually satisfies
the structural-record condition lands, whether that is multi-parent blocks or one of Finding 2's other
options. That is exactly what a release-blocked-not-build-blocked condition is for, and DC-43/DC-52
already use the same vehicle in `rfcs/README.md`.

Building single-parent merge execution now, knowing it may need revisiting once the record lands, is
real but bounded rework — the addendum already named this cost explicitly and accepted it. Blocking on
solving an unresolved, large, structurally central design question first is a worse trade: it delays
proven, working, useful capability behind a problem that does not yet have a shape agreed on.

## Request

Proceeding to §3 implementation (single-parent block adoption, seal, clean conflict refusal) unless
directed otherwise. Recording this sizing finding here so whoever eventually scopes the multi-parent/
merge-record increment has it, rather than inheriting the addendum's "confined to derived-state
machinery" framing uncorrected.
