//! RFC 130 / v2 handoff: the module coupling invariant over `prikk-store`'s production module
//! graph.
//!
//! **Two things this gate enforces, both as an allowlist-with-reasons, neither as a bare bound**
//! (RFC 130 §4.1, amended by §4b to cover cycles the same way §4.1 always covered hubs — "the gate
//! forces a recorded decision; it does not adjudicate one"):
//!
//! 1. **Every directed edge inside a strongly-connected component must be declared** in
//!    [`DECLARED_CYCLES`], with a reason *and* a statement of what would remove it (§4b.3 —
//!    stricter than the hub treatment, because a cycle is the thing itself, not a proxy for it:
//!    "what breaks if I change this?" is unanswerable inside one). **Checked per edge, not per
//!    elementary cycle** — a strongly-connected component of N modules can admit combinatorially
//!    many distinct elementary cycles built from the same handful of underlying edges, and
//!    blessing every recombination separately would both multiply entries without adding
//!    information and miss a genuinely new one hiding among old edges. An edge is the unit an
//!    entry can actually explain and a diff can actually change.
//! 2. **A module newly crossing the hub threshold fails until declared** in [`DECLARED_HUBS`], with
//!    a reason.
//!
//! **What this gate does not do** (§5 / v2 handoff §6): it does not gate line count or module
//! count, it does not repair the graph, and declaring an entry is not licence to act on it.
//!
//! ## This round's own re-derivation found more than RFC 130's own table names
//!
//! §4b.4 names four cycles across six modules. Re-measuring independently (v2 handoff §3.2's own
//! instruction, not optional) found **fifteen distinct directed edges across seven modules** —
//! five edges and one module (`recognition_claim`) beyond that table. Every one of the five extra
//! edges is confirmed at source and, via `git log -S`, predates `04e9391` (RFC 130's own
//! measurement commit) by weeks to months — present when the external review counted "exactly one
//! cycle" and missed since, the same way `trust ↔ refs` was. The seventh module,
//! `recognition_claim`, joined the component only because *this session's own* RFC 138 round added
//! `trust -> recognition_claim` (`trust.rs`'s `load_maintainer_trust_policy_or_empty`, calling an
//! existing helper by its current address rather than relocating it) atop the pre-existing
//! `recognition_claim -> trust` edge. See this round's report for the full derivation and the
//! git-blame evidence; every one of the resulting ten entries below states its own reason and
//! removal statement, per §4b.3 — declaring them **is** the evaluation RFC 130 keeps saying has
//! never happened, extended to cover what re-measuring actually found rather than what was
//! expected.

mod cfg_expr;
mod graph;

use std::collections::BTreeSet;
use std::path::Path;

use super::{BoundaryError, push};

/// A module pair is a hub if it has at least this much fan-in *and* fan-out (`min(fan_in,
/// fan_out) >= HUB_THRESHOLD`). Derived from this round's own measured distribution (report
/// §3.5.1), not asserted -- sorted by `min(fan_in, fan_out)`, today's ranking is 13 (`refs`), 11
/// (`patch_replay`), 8 (`lifecycle_cache`), then a three-way tie at 6 (`wal`, `active`, `trust`),
/// then a clean drop to 5. The break sits between 6 and 5, not between 8 and 6: `active` tying
/// `wal` was already flagged as a live trend by the coupling-gate-graph-contradiction round: this
/// derivation confirms it, and finds `trust` has now joined them too -- specifically because this
/// session's own RFC 138 round added one outgoing edge (`trust -> recognition_claim`) that moved
/// `trust` from fan-out 5 to 6. See `graph::tests::the_scc_has_exactly_this_edge_set` for the exact
/// numbers this constant is checked against.
const HUB_THRESHOLD: usize = 6;

/// One declared cycle-forming edge: the reason it exists, and — the property that makes a cycle
/// entry stricter than a hub entry (§4b.3) — a statement of what would have to change to remove
/// it. Neither string may be empty or a placeholder
/// (`declared_cycles_have_real_reasons_and_removal_statements` below). Grouped as `edges` rather
/// than one entry per edge purely so a genuinely mutual pair (`a <-> b`) reads as one relationship
/// with one explanation instead of two identical-looking half-entries — the gate itself checks
/// each edge independently regardless of how entries group them.
struct DeclaredCycle {
    edges: &'static [(&'static str, &'static str)],
    reason: &'static str,
    what_would_remove_it: &'static str,
}

/// Every edge found inside `prikk-store`'s one strongly-connected component today. Ten entries,
/// covering all fifteen directed edges among `active`, `refs`, `trust`, `worktree_patch`,
/// `patch_replay`, `lifecycle_cache`, `recognition_claim` — five mutual pairs plus five one-way
/// edges that close longer cycles through them. Checked exhaustively against the real graph by
/// `graph::tests::every_scc_edge_is_covered_by_a_declared_cycle`.
const DECLARED_CYCLES: &[DeclaredCycle] = &[
    DeclaredCycle {
        edges: &[("active", "refs"), ("refs", "active")],
        reason: "the only cycle anyone had evaluated before this gate: active-session/WAL state \
                  and ref publication are two views of the same commit boundary, and each \
                  legitimately needs to ask the other about it (RFC 130 §2.1's own grandfather)",
        what_would_remove_it: "extracting the shared commit-boundary question both sides ask into \
                                a third module neither depends on -- not attempted here, since \
                                this pair predates this gate and no defect motivates touching it",
    },
    DeclaredCycle {
        edges: &[("refs", "trust"), ("trust", "refs")],
        reason: "refs enforces no-incomplete-publication before a trust-policy write \
                  (`ensure_no_incomplete_publication`), and trust supplies the adopted-key policy \
                  refs' own publication-trust verification reads back -- present since before \
                  `04e9391` and missed by two independent extractions at that time (RFC 130 §4a), \
                  not new, and not yet evaluated by anyone until this gate's own derivation",
        what_would_remove_it: "moving the incomplete-publication guard trust currently calls into \
                                a module neither refs nor trust depends on, or accepting the \
                                policy snapshot as a plain argument at the one call site instead \
                                of reading it back from refs -- a real option, not attempted here \
                                because no defect motivates it and RFC 130 §7 rules out module \
                                moves in this increment",
    },
    DeclaredCycle {
        edges: &[
            ("lifecycle_cache", "patch_replay"),
            ("patch_replay", "lifecycle_cache"),
        ],
        reason: "created by RFC 122 (`7a01168`), consolidating two duplicate baseline \
                  derivations into `lifecycle_cache::incremental::resolve_baseline_state` -- \
                  exactly the consolidation RFC 130 §4's own counter-example describes: sharing a \
                  derivation necessarily concentrates edges at the shared site, and a bare \
                  absolute-acyclicity rule would have rejected this correct, audit-required fix",
        what_would_remove_it: "giving `patch_replay` its own copy of the baseline derivation \
                                again, reintroducing the duplication RFC 122 removed -- worse, not \
                                better, and not a change this gate should ever encourage",
    },
    DeclaredCycle {
        edges: &[("active", "worktree_patch"), ("worktree_patch", "active")],
        reason: "found by this round's own re-derivation, present since 2026-07-03/2026-08-02 \
                  (git log -S on each leg) and missed by every prior pass: `active` asks \
                  `worktree_patch::active_patch_limit_exceeded` whether the queue is full, and \
                  `worktree_patch`'s node authoring asks `active` to prepare/validate the active \
                  ref metadata it needs before authoring -- two co-designed layers each checking \
                  the half of the operation the other owns, the same shape as `active <-> refs`",
        what_would_remove_it: "extracting the patch-limit check into a module neither depends on, \
                                and having callers pass already-prepared active-ref metadata into \
                                worktree_patch's authoring functions instead of worktree_patch \
                                asking active for it mid-call -- a real, scoped option, not \
                                attempted here for the same §7 reason as the pair above",
    },
    DeclaredCycle {
        edges: &[
            ("trust", "recognition_claim"),
            ("recognition_claim", "trust"),
        ],
        reason: "NOT the same shape as the others, and said so plainly: `recognition_claim -> \
                  trust` is old and legitimate (a claim's signer must be checked against trust \
                  policy). `trust -> recognition_claim` is new -- added by this session's own RFC \
                  138 round, whose `load_maintainer_trust_policy_or_empty` wrapper called the \
                  existing `maintainer_trust_policy_or_empty` helper by its current address \
                  (`recognition_claim.rs`) rather than relocating it. This is the gate finding \
                  exactly the failure mode RFC 130 exists to catch, one round after the gate was \
                  proposed, in this developer's own prior work",
        what_would_remove_it: "moving `maintainer_trust_policy_or_empty` out of `recognition_claim.rs` \
                                and into `trust.rs`, where it reads more naturally anyway (it is a \
                                trust-policy read helper, not a recognition-claim-specific one), \
                                and updating its three callers (`recognition_claim.rs` itself, \
                                `tag_travel.rs`, `seal_from_accepted.rs`) to call it from `trust` \
                                instead -- a real, mechanical, low-risk fix, recommended as a \
                                near-term follow-up and not attempted here because RFC 130 §7 rules \
                                out module changes in this increment",
    },
    DeclaredCycle {
        edges: &[
            ("worktree_patch", "patch_replay"),
            ("patch_replay", "active"),
        ],
        reason: "the original RFC 130 §4a.2/§4b.4 3-cycle's own two edges (its third leg, `active \
                  -> worktree_patch`, now has its own entry above since this round also found the \
                  return leg `worktree_patch -> active`): worktree_patch's authoring resolves a \
                  folded baseline via `patch_replay::resolve_folded_worktree_baseline`, and \
                  `patch_replay` reads active-ref metadata back through `active`'s own crate-root \
                  re-exports (`read_active_ref_metadata`/`ActiveRefMetadata`) -- the concrete case \
                  RFC 130 §1 warned an edge extractor could miss: `patch_replay.rs` never writes \
                  `crate::active::` anywhere, only the re-exported names",
        what_would_remove_it: "having `worktree_patch` pass the active-ref metadata it already \
                                holds into `patch_replay`'s call directly, so `patch_replay` never \
                                needs to ask `active` for it a second time -- a real, scoped fix, \
                                not attempted here for the same §7 reason as the others",
    },
    DeclaredCycle {
        edges: &[("worktree_patch", "lifecycle_cache")],
        reason: "found by this round's own re-derivation (2026-08-02, DC-66): worktree_patch's \
                  node authoring uses `lifecycle_cache`'s `TextCache`/`materialize_edited_text` \
                  to author text-edit patches against the cached derived state -- an ordinary \
                  consumer-of-a-cache relationship, closing a longer cycle only because \
                  `lifecycle_cache` already reaches back to `active`'s own layer through \
                  `patch_replay`",
        what_would_remove_it: "giving worktree_patch's text-edit authoring its own text \
                                materialization instead of reusing lifecycle_cache's, which would \
                                reintroduce duplication this crate has otherwise been removing \
                                (RFC 122's own consolidation) -- not a change worth making",
    },
    DeclaredCycle {
        edges: &[("worktree_patch", "refs")],
        reason: "found by this round's own re-derivation (2026-07-15, DC-38): worktree_patch's \
                  node authoring calls `refs::ensure_no_incomplete_publication` before authoring, \
                  the identical pre-mutation guard `trust.rs` calls at two of its own call sites -- \
                  the same crash-recovery precondition enforced at every mutation entry point, not \
                  something specific to worktree_patch",
        what_would_remove_it: "moving the incomplete-publication guard into a module every mutator \
                                (worktree_patch, trust, and refs' own callers) depends on instead \
                                of refs itself -- the same option named for the refs/trust pair \
                                above, and it would remove this edge along with that one",
    },
    DeclaredCycle {
        edges: &[("patch_replay", "refs")],
        reason: "found by this round's own re-derivation (2026-06-27/2026-07-29): patch_replay \
                  needs `RefStore` to resolve which ref/block it is replaying from -- an ordinary \
                  \"replay engine reads the ref it targets\" dependency, closing a longer cycle \
                  only because refs already reaches back into the active/worktree_patch layer",
        what_would_remove_it: "passing the already-resolved block id into patch_replay's replay \
                                functions instead of having them resolve a ref via RefStore \
                                themselves -- callers already have this information in every \
                                current call site, so this is a real, scoped option, not attempted \
                                here for the same §7 reason as the others",
    },
];

/// One declared hub: a module with high fan-in *and* high fan-out, and why that is consolidation
/// rather than sprawl (RFC 130 §4.1 — the idiom this constant already existed for, before §4b
/// extended it to cycles too).
struct DeclaredHub {
    module: &'static str,
    reason: &'static str,
}

/// Today's six hubs by `min(fan_in, fan_out)` (report §3.5.1's re-derivation). The RFC's original
/// four (`refs`, `patch_replay`, `wal`, `lifecycle_cache`) plus two new entrants tied with `wal` at
/// the threshold: `active` (a trend the coupling-gate-graph-contradiction round already flagged)
/// and `trust` (newly crossing specifically because this session's own RFC 138 round added one
/// outgoing edge).
const DECLARED_HUBS: &[DeclaredHub] = &[
    DeclaredHub {
        module: "refs",
        reason: "the ref-publication layer: every publishing operation (seal, merge, sync seal, \
                  sync adopt-tag, tag create, branch create/close) reads or writes through it, and \
                  it in turn reads from most of the object/patch layer it publishes -- high \
                  fan-in and fan-out are both structural to being the one place publication is \
                  gated, not a sign the module is doing unrelated things",
    },
    DeclaredHub {
        module: "patch_replay",
        reason: "the shared replay engine every checkout, verification, and merge path drives, \
                  and RFC 122's consolidation concentrated a second baseline derivation into it on \
                  top of that -- fan-out grew for the same reason the lifecycle_cache cycle above \
                  exists: a correct consolidation, not sprawl",
    },
    DeclaredHub {
        module: "lifecycle_cache",
        reason: "the node-lifecycle cache every replay path reads through and every mutation path \
                  invalidates -- a cache's whole purpose is sitting between a wide set of readers \
                  and a wide set of writers, so both-sides-high fan is the shape a cache is \
                  supposed to have",
    },
    DeclaredHub {
        module: "wal",
        reason: "the active write-ahead-log container every active-session operation (commit, \
                  seal, doctor, unlock, compact) reads or appends through -- a single shared queue \
                  necessarily has wide fan-in from everything that queues work and wide fan-out to \
                  everything that shapes a queued record",
    },
    DeclaredHub {
        module: "active",
        reason: "the active-session/ref-metadata layer every commit-boundary operation touches on \
                  both sides -- readers asking whether an active ref exists and is valid, writers \
                  preparing or clearing it. Newly crossing the threshold as a trend already flagged \
                  by the coupling-gate-graph-contradiction round, driven by the same cyclic \
                  relationships (with refs, worktree_patch, patch_replay) declared above, not by \
                  unrelated scope creep",
    },
    DeclaredHub {
        module: "trust",
        reason: "the publication-trust layer every publish-gated operation checks a signer \
                  against, with fan-out already near this threshold before this round. Crosses it \
                  specifically because of this session's own RFC 138 change (`trust -> \
                  recognition_claim`, declared above) -- named here rather than treated as sprawl \
                  because the underlying edge already has its own honest accounting, including a \
                  recommended fix that would remove both the cycle and this hub crossing together",
    },
];

/// Whether `cycle` (a sequence of module names, implicitly closing back to its own first member)
/// traverses the directed edge `from -> to` at some point, wraparound included.
fn cycle_contains_edge(cycle: &[String], from: &str, to: &str) -> bool {
    cycle
        .windows(2)
        .any(|pair| matches!(pair, [a, b] if a == from && b == to))
        || cycle.last().is_some_and(|last| last == from)
            && cycle.first().is_some_and(|first| first == to)
}

pub(super) fn check(root: &Path, errors: &mut Vec<BoundaryError>) {
    check_allowlists_are_well_formed(errors);
    let src_root = root.join("crates/prikk-store/src");
    let graph = match graph::build(&src_root) {
        Ok(graph) => graph,
        Err(message) => {
            push(
                errors,
                "module-coupling",
                format!("graph build failed: {message}"),
            );
            return;
        }
    };

    let declared_edges: BTreeSet<(String, String)> = DECLARED_CYCLES
        .iter()
        .flat_map(|entry| {
            entry
                .edges
                .iter()
                .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
        })
        .collect();
    // Computed once, only to make an undeclared-edge finding actionable: pointing at one concrete
    // elementary cycle the new edge participates in is far more useful than the bare edge alone.
    let elementary_cycles = graph.elementary_cycles();
    for component in graph::strongly_connected_components(&graph) {
        if component.len() < 2 {
            continue;
        }
        let members: BTreeSet<&str> = component.iter().map(String::as_str).collect();
        for (from, to) in &graph.edges {
            if members.contains(from.as_str())
                && members.contains(to.as_str())
                && !declared_edges.contains(&(from.clone(), to.clone()))
            {
                let example = elementary_cycles
                    .iter()
                    .find(|cycle| cycle_contains_edge(cycle, from, to));
                let cycle_note = example
                    .and_then(|cycle| {
                        cycle
                            .first()
                            .map(|first| format!(" (e.g. {} -> {first})", cycle.join(" -> ")))
                    })
                    .unwrap_or_default();
                push(
                    errors,
                    "module-coupling",
                    format!(
                        "undeclared cycle-forming edge: {from} -> {to}{cycle_note} -- add a \
                         DECLARED_CYCLES entry with a reason and a statement of what would remove \
                         it, or this is real accidental coupling to fix instead"
                    ),
                );
            }
        }
    }

    let declared_hub_names: BTreeSet<&str> =
        DECLARED_HUBS.iter().map(|entry| entry.module).collect();
    for module in &graph.modules {
        let fan_in = graph.fan_in(module);
        let fan_out = graph.fan_out(module);
        if fan_in.min(fan_out) >= HUB_THRESHOLD && !declared_hub_names.contains(module.as_str()) {
            push(
                errors,
                "module-coupling",
                format!(
                    "undeclared hub: {module} (fan-in {fan_in}, fan-out {fan_out}) -- add a \
                     DECLARED_HUBS entry saying why this is consolidation and not sprawl"
                ),
            );
        }
    }
}

/// Self-guard on both allowlists, the same idiom `DECLARED_UNDOCUMENTED`'s own tests use
/// (`crates/prikk-cli/src/commands/tests.rs`): an entry with an empty or placeholder reason
/// documents nothing, and for a cycle specifically, one that cannot say what would remove it is an
/// entry nobody understands (§4b.3) rather than a real evaluation.
fn check_allowlists_are_well_formed(errors: &mut Vec<BoundaryError>) {
    for entry in DECLARED_CYCLES {
        if entry.edges.is_empty() {
            push(
                errors,
                "module-coupling",
                "DECLARED_CYCLES entry has no edges".to_owned(),
            );
        }
        if is_placeholder(entry.reason) {
            push(
                errors,
                "module-coupling",
                format!("DECLARED_CYCLES entry {:?} has no real reason", entry.edges),
            );
        }
        if is_placeholder(entry.what_would_remove_it) {
            push(
                errors,
                "module-coupling",
                format!(
                    "DECLARED_CYCLES entry {:?} does not state what would remove it",
                    entry.edges
                ),
            );
        }
    }
    for entry in DECLARED_HUBS {
        if is_placeholder(entry.reason) {
            push(
                errors,
                "module-coupling",
                format!("DECLARED_HUBS entry `{}` has no real reason", entry.module),
            );
        }
    }
}

fn is_placeholder(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.is_empty() || trimmed.len() < 20
}

#[cfg(test)]
#[path = "coupling/tests.rs"]
mod tests;
