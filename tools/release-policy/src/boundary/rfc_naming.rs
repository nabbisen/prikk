//! RFC 105: the RFC naming gate. Turns RFC 100's naming rule ("new RFCs are `NNN-slug`, numbered
//! from 100; existing RFCs keep their names permanently") from prose into a checked control --
//! RFC 100 shipped without one, and DC-96 through DC-99 were created after its acceptance, in the
//! scheme it retired, unnoticed for six days and four increments (RFC 105 §0).
//!
//! **The rule.** Every entry directly under the five governed locations must either conform to the
//! pattern or appear in that location's frozen legacy allowlist below:
//! - files under `rfcs/proposed/`, `rfcs/accepted/`, `rfcs/done/`, `rfcs/archive/`:
//!   `^[0-9]{3}-[a-z0-9]+(-[a-z0-9]+)*\.md$`
//! - directories under `rfcs/handoffs/`: `^[0-9]{3}-[a-z0-9]+(-[a-z0-9]+)*$` (no `.md`)
//!
//! **The self-guard (RFC criterion 3).** Every allowlisted name must correspond to an entry that
//! actually exists **at that specific location** -- so pre-authorising a name before it exists (the
//! cheap "reserve `DC-100`" bypass) fails, and a file-form entry does not cover a directory of the
//! same identifier or vice versa, since each of the five lists below is checked only against its
//! own directory's listing. This does not, and is not meant to, stop a deliberate two-line edit
//! adding both a new legacy entry and its file/directory together -- that is the same standard
//! `unsafe_boundary.rs`'s `UNSAFE_EXEMPT_CRATES` sets: a visible edit to a reviewed constant is the
//! control, invisibility was the problem.
//!
//! **How the five lists below were generated (RFC 105 design-v1.md §3: derive, never
//! transcribe).** At the commit this module first landed:
//! ```sh
//! find rfcs/proposed rfcs/accepted rfcs/done rfcs/archive -maxdepth 1 -mindepth 1 -type f
//! find rfcs/handoffs -maxdepth 1 -mindepth 1 -type d
//! ```
//! each entry checked against the pattern above and kept only if non-conforming. **197 entries
//! total**: 5 `rfcs/proposed/`, 35 `rfcs/accepted/`, 79 `rfcs/done/`, 4 `rfcs/archive/`, 74
//! `rfcs/handoffs/`. `find`, not a shell glob -- a glob silently skips dotfiles, which mattered:
//! three `.gitkeep` housekeeping placeholders live in `rfcs/proposed/`, `rfcs/accepted/`, and
//! `rfcs/archive/` (none in `rfcs/done/` or `rfcs/handoffs/`, since the latter holds directories
//! only). They carry no special exemption of their own
//! (`.git-exclude/reviewed/RFC-105-investigation-ruling-v1.md` §4: this project has no
//! dotfile-exemption category, so they get ordinary allowlist entries like everything else, per
//! `boundary/publication.rs`'s own `collect_files` precedent of filtering by a positive predicate
//! rather than special-casing dotfiles).
//!
//! **One entry is not a legacy RFC name at all.** `consolidation`, under `rfcs/handoffs/`, is a
//! handoff directory (`dead-surface-consolidation-handoff-v1.md`) with **no governing RFC anywhere**
//! in `accepted/`, `done/`, `proposed/`, or `archive/` (`RFC-105-investigation-ruling-v1.md` §2) --
//! work that reached a handoff and merged without ever having had an RFC, a process gap of a
//! different kind than a naming violation and out of this gate's scope to fix. It is allowlisted
//! for exactly that reason, stated here rather than folded silently into "legacy naming" -- an
//! allowlist entry whose comment misdescribes why it exists is how the next reader concludes this
//! rule has exceptions it does not have.
//!
//! ## What this gate cannot see
//! - **Slug quality.** That a slug is lowercase and hyphenated is checkable; that it accurately
//!   describes its RFC is not (RFC 105 §4 non-goal) -- a gate that pretended otherwise would be
//!   worse than none.
//! - **A wrong but well-formed number.** `106` used twice would both pass this check; it verifies
//!   shape, not uniqueness or correctness of the number itself.
//! - **A deliberate two-line bypass.** Adding a new legacy file or directory and its allowlist
//!   entry together in one edit remains possible, by design -- see the self-guard paragraph above.

use super::{BoundaryError, push};

/// `rfcs/proposed/` -- 4 pre-RFC-100 `DC-*` proposals plus `.gitkeep`. Added to this gate's scope
/// by `RFC-105-investigation-ruling-v1.md` §3, correcting an omission in the RFC's first draft:
/// it is where a *new* RFC first appears, so a gate that skipped it would be blind at exactly the
/// moment the naming rule applies.
const RFC_PROPOSED_LEGACY: &[&str] = &[
    ".gitkeep",
    "DC-43-RELEASE-SECURITY-CONTROLS.md",
    "DC-44-MIGRATION-BACKUP-RESTORE-EVIDENCE.md",
    "DC-49-PORTABLE-LOGIC-PLATFORM-MATRIX.md",
    "DC-53-REPOSITORY-WIDE-AUTHOR-TRUST-VERIFICATION.md",
];

/// `rfcs/accepted/` -- 34 pre-RFC-100 `DC-*` RFCs plus `.gitkeep`. `100-*.md` through `106-*.md`
/// already conform and need no entry here.
const RFC_ACCEPTED_LEGACY: &[&str] = &[
    ".gitkeep",
    "DC-34-PUBLICATION-IDENTITY-AUTHORITY.md",
    "DC-35-RELEASE-COMPATIBILITY-STATUS-CORRECTION.md",
    "DC-36-EXISTING-OBJECT-PUBLICATION-INTEGRITY.md",
    "DC-37-REQUIRED-FILESYSTEM-DURABILITY.md",
    "DC-38-REF-PUBLICATION-CRASH-RECOVERY.md",
    "DC-41-INTEGRITY-EVIDENCE-CAMPAIGN.md",
    "DC-45-RELEASE-POLICY-TOOLING-CONSOLIDATION.md",
    "DC-50-FIRST-PARTY-SHA256-ROI-DECISION.md",
    "DC-51-PRODUCT-DEPENDENCY-PLACEMENT-GATE.md",
    "DC-54-OPERATION-PATH-VALIDATION-SYMMETRY.md",
    "DC-56-COMMIT-FULL-TREE-SCAN-COMPLIANCE.md",
    "DC-57-ACTIVE-PATCH-THRESHOLDS.md",
    "DC-58-SOURCE-STRUCTURE-AUDIT.md",
    "DC-59-COMMIT-BENCHMARK-HARNESS.md",
    "DC-60-BRANCH-MANAGEMENT-SURFACE.md",
    "DC-61-BRANCH-CLOSURE.md",
    "DC-63-TAG-SURFACE.md",
    "DC-64-BASELINE-RECONSTRUCTION-COST.md",
    "DC-65-TEXT-EDIT-BASELINE-CONTENT.md",
    "DC-66-MULTI-COMMIT-QUEUING.md",
    "DC-67-ORDINARY-USE-CONFORMANCE.md",
    "DC-69-LIFECYCLE-STATE-RETENTION.md",
    "DC-70-PREBUILT-BINARY-DISTRIBUTION.md",
    "DC-71-NON-LINUX-BUILD-CONFORMANCE.md",
    "DC-72-PATH-SAFETY-CONFORMANCE.md",
    "DC-73-NODE-MODEL-APPLY.md",
    "DC-87-WINDOWS-MUTATION.md",
    "DC-93-RELEASE-POLICY-PYTHON-RETIREMENT.md",
    "DC-94-RESPONSIBILITY-MAP-EXECUTABLE-BINDING.md",
    "DC-95-VERIFY-COVERAGE-AND-FINDING-ACCUMULATION.md",
    "DC-96-WINDOWS-ANCHOR-IDENTITY.md",
    "DC-97-WINDOWS-DURABILITY-EVIDENCE.md",
    "DC-98-WINDOWS-CRASH-INJECTION.md",
    "DC-99-WINDOWS-CAPABILITY-PARITY.md",
];

/// `rfcs/done/` -- 49 pre-RFC-100 `DC-*` RFCs and 30 `PR-*` handoffs. `000-rfc-lifecycle-policy.md`
/// already conforms and needs no entry here.
const RFC_DONE_LEGACY: &[&str] = &[
    "DC-10-ROLLBACK-DRAFT-SIGNING.md",
    "DC-11-MAINTAINER-TRUST-STORE.md",
    "DC-12-ARBITRARY-SPAN-TEXT-EDITS.md",
    "DC-13-NONDEFAULT-REF-GENESIS.md",
    "DC-14-ARBITRARY-SPAN-TEXT-INVERSE-ROLLBACK.md",
    "DC-15-ACTIVE-SESSION-INTEGRITY-HARDENING.md",
    "DC-16-PATCH-ALGEBRA-FOUNDATION.md",
    "DC-17-PATCH-ALGEBRA-EVIDENCE-CONTRACT.md",
    "DC-18-PATCH-ALGEBRA-COMMUTATION-CONFLUENCE.md",
    "DC-19-REPLAY-LIFECYCLE-CRATE-BOUNDARY.md",
    "DC-20-REPLAY-BOUNDARY-STABILIZATION.md",
    "DC-21-MERGE-CONFLICT-EVIDENCE-CONTRACT.md",
    "DC-22-PUBLIC-MERGE-EVIDENCE-UX.md",
    "DC-23-MERGE-EVIDENCE-UX-STABILIZATION.md",
    "DC-24-DATA-MODEL-TRUST-THREAT-DOCS.md",
    "DC-25-MERGE-PLANNING-SURFACE.md",
    "DC-26-DOCUMENTATION-HOME-CORRECTION.md",
    "DC-27-PATCH-ALGEBRA-MERGE-EVIDENCE-CONCEPTS.md",
    "DC-28-DURABILITY-CRASH-RECOVERY-REFERENCE.md",
    "DC-29-VERIFY-DOCTOR-INTEGRITY-RECOVERY-REFERENCE.md",
    "DC-30-KEY-MANAGEMENT-SIGNING-SETUP-GUIDE.md",
    "DC-31-REPOSITORY-LAYOUT-AUTHORITY-REFERENCE.md",
    "DC-32-PATH-WORKTREE-SAFETY-REFERENCE.md",
    "DC-33-CONCURRENCY-LOCKING-REFERENCE.md",
    "DC-39-SIGNATURE-ENVELOPE-AUTHORITY.md",
    "DC-40-STATE-MERKLE-FORMAT-TRANSITION.md",
    "DC-46-WORKSPACE-RUST-1.85-COMPATIBILITY.md",
    "DC-47-STABLE-CLIPPY-GATE-ALIGNMENT.md",
    "DC-48-LEGACY-CLIPPY-PRODUCTION-RETIREMENT.md",
    "DC-55-FIRST-PARTY-SHA256-REPLACEMENT.md",
    "DC-62-COMMIT-BENCHMARK-MEMORY-AXIS.md",
    "DC-74-MERGE-EXECUTION.md",
    "DC-75-MERGE-BLOCK-LINEAGE.md",
    "DC-76-FILESYSTEM-DURABILITY-CONTRACT.md",
    "DC-77-DOCS-MERMAID-RENDERING.md",
    "DC-78-HISTORY-EXCHANGE.md",
    "DC-79-SHA2-GETRANDOM-UPGRADE.md",
    "DC-80-ED25519-DALEK-UPGRADE.md",
    "DC-81-MACOS-MUTATION.md",
    "DC-82-MUTATION-DISPATCH-COLLAPSE.md",
    "DC-83-TEST-TEMP-DIR-UNIQUENESS.md",
    "DC-84-TEST-HELPER-UNIQUENESS-SWEEP.md",
    "DC-85-MERGE-FROM-RECEIVED-REF.md",
    "DC-86-BUNDLE-DECODER-HARDENING.md",
    "DC-88-DURABILITY-CONTRACT-REQUIREMENT-SHAPE.md",
    "DC-89-PLATFORM-CLAIM-DOCS-ACCURACY.md",
    "DC-90-UNSAFE-CODE-BOUNDARY-GATE.md",
    "DC-91-PUBLICATION-RECORD-SHAPE.md",
    "DC-92-LINEAGE-REPLAY-MEMOIZATION.md",
    "PR-001-IMPLEMENTATION-HANDOFF.md",
    "PR-002-CI-FIX-HANDOFF.md",
    "PR-003-PERSISTENT-STORE-HANDOFF.md",
    "PR-004-WAL-HANDOFF.md",
    "PR-005-CI-FIX-HANDOFF.md",
    "PR-006-VERIFY-HANDOFF.md",
    "PR-007-REF-PUBLICATION-HANDOFF.md",
    "PR-008-COMMIT-SCAFFOLD-HANDOFF.md",
    "PR-009-SEAL-SCAFFOLD-HANDOFF.md",
    "PR-010-VERIFY-HARDENING-HANDOFF.md",
    "PR-011-DOCTOR-HANDOFF.md",
    "PR-012-DOCTOR-REPAIR-HANDOFF.md",
    "PR-013-REF-RECOVERY-HANDOFF.md",
    "PR-014-HISTORY-HANDOFF.md",
    "PR-015-CHECKOUT-PLAN-HANDOFF.md",
    "PR-016-SNAPSHOT-PATH-SAFETY-HANDOFF.md",
    "PR-017-SNAPSHOT-MATERIALIZATION-HANDOFF.md",
    "PR-018-WORKTREE-STATUS-HANDOFF.md",
    "PR-019-WORKTREE-PATCH-HANDOFF.md",
    "PR-020-PATCH-REPLAY-HANDOFF.md",
    "PR-021-PATCH-MATERIALIZATION-HANDOFF.md",
    "PR-022-PATCH-DELETION-HANDOFF.md",
    "PR-023-TEXT-ANCHOR-HANDOFF.md",
    "PR-024-TEXT-REPLAY-HANDOFF.md",
    "PR-025-TEXT-GENERATION-HANDOFF.md",
    "PR-026-INVERSE-PLAN-HANDOFF.md",
    "PR-027-ROLLBACK-PREVIEW-HANDOFF.md",
    "PR-028-ROLLBACK-DRAFT-HANDOFF.md",
    "PR-029-ROLLBACK-DRAFT-VERIFY-HANDOFF.md",
    "PR-030-SEALED-ROLLBACK-HISTORY-HANDOFF.md",
];

/// `rfcs/archive/` -- 3 pre-RFC-100 `DC-*` RFCs plus `.gitkeep`. `101-*.md` and `104-*.md` already
/// conform and need no entry here.
const RFC_ARCHIVE_LEGACY: &[&str] = &[
    ".gitkeep",
    "DC-09-PHASE-4-NODE-MODEL.md",
    "DC-42-PERFORMANCE-MAINTAINABILITY-GATES.md",
    "DC-52-PYTHON-ORACLE-DECOMMISSIONING.md",
];

/// `rfcs/handoffs/` -- 73 pre-RFC-100 `DC-*` handoff directories, plus `consolidation` (not a
/// legacy RFC name; see the module doc). `101-…` through `103-…` and `105-…`/`106-…` already
/// conform and need no entry here.
const RFC_HANDOFFS_LEGACY: &[&str] = &[
    "consolidation",
    "DC-10-rollback-draft-signing",
    "DC-11-maintainer-trust-store",
    "DC-12-arbitrary-span-text-edits",
    "DC-13-nondefault-ref-genesis",
    "DC-14-arbitrary-span-text-inverse-rollback",
    "DC-15-active-session-integrity-hardening",
    "DC-16-patch-algebra-foundation",
    "DC-17-patch-algebra-evidence-contract",
    "DC-18-patch-algebra-commutation-confluence",
    "DC-19-replay-lifecycle-crate-boundary",
    "DC-20-replay-boundary-stabilization",
    "DC-21-merge-conflict-evidence-contract",
    "DC-22-public-merge-evidence-ux",
    "DC-23-merge-evidence-ux-stabilization",
    "DC-24-data-model-trust-threat-docs",
    "DC-25-merge-planning-surface",
    "DC-37-required-filesystem-durability",
    "DC-39-signature-envelope-authority",
    "DC-40-state-merkle-format-transition",
    "DC-41-integrity-evidence-campaign",
    "DC-42-performance-maintainability-gates-superseded",
    "DC-43-release-security-controls",
    "DC-44-migration-backup-restore-evidence",
    "DC-49-portable-logic-platform-matrix",
    "DC-50-first-party-sha256-roi-decision",
    "DC-51-product-dependency-placement-gate",
    "DC-52-python-oracle-decommissioning",
    "DC-53-repository-wide-author-trust-verification",
    "DC-54-operation-path-validation-symmetry",
    "DC-55-first-party-sha256-replacement",
    "DC-56-commit-full-tree-scan-compliance",
    "DC-57-active-patch-thresholds",
    "DC-58-source-structure-audit",
    "DC-59-commit-benchmark-harness",
    "DC-60-branch-management-surface",
    "DC-61-branch-closure",
    "DC-62-commit-benchmark-memory-axis",
    "DC-63-tag-surface",
    "DC-64-baseline-reconstruction-cost",
    "DC-65-text-edit-baseline-content",
    "DC-66-multi-commit-queuing",
    "DC-67-ordinary-use-conformance",
    "DC-69-lifecycle-state-retention",
    "DC-70-prebuilt-binary-distribution",
    "DC-71-non-linux-build-conformance",
    "DC-72-path-safety-conformance",
    "DC-73-node-model-apply",
    "DC-74-merge-execution",
    "DC-75-merge-block-lineage",
    "DC-76-filesystem-durability-contract",
    "DC-77-docs-mermaid-rendering",
    "DC-78-history-exchange",
    "DC-79-sha2-getrandom-upgrade",
    "DC-80-ed25519-dalek-upgrade",
    "DC-81-macos-mutation",
    "DC-82-mutation-dispatch-collapse",
    "DC-83-test-temp-dir-uniqueness",
    "DC-84-test-helper-uniqueness-sweep",
    "DC-85-merge-from-received-ref",
    "DC-86-bundle-decoder-hardening",
    "DC-87-windows-mutation",
    "DC-88-durability-contract-requirement-shape",
    "DC-89-platform-claim-docs-accuracy",
    "DC-90-unsafe-code-boundary-gate",
    "DC-91-publication-record-shape",
    "DC-92-lineage-replay-memoization",
    "DC-93-release-policy-python-retirement",
    "DC-94-responsibility-map-executable-binding",
    "DC-95-verify-coverage-and-finding-accumulation",
    "DC-96-windows-anchor-identity",
    "DC-97-windows-durability-evidence",
    "DC-98-windows-crash-injection",
    "DC-99-windows-capability-parity",
];

pub(super) fn check(root: &std::path::Path, errors: &mut Vec<BoundaryError>) {
    check_location(
        root,
        "rfcs/proposed",
        EntryKind::File,
        RFC_PROPOSED_LEGACY,
        errors,
    );
    check_location(
        root,
        "rfcs/accepted",
        EntryKind::File,
        RFC_ACCEPTED_LEGACY,
        errors,
    );
    check_location(root, "rfcs/done", EntryKind::File, RFC_DONE_LEGACY, errors);
    check_location(
        root,
        "rfcs/archive",
        EntryKind::File,
        RFC_ARCHIVE_LEGACY,
        errors,
    );
    check_location(
        root,
        "rfcs/handoffs",
        EntryKind::Directory,
        RFC_HANDOFFS_LEGACY,
        errors,
    );
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EntryKind {
    File,
    Directory,
}

fn check_location(
    root: &std::path::Path,
    location: &str,
    kind: EntryKind,
    legacy: &[&str],
    errors: &mut Vec<BoundaryError>,
) {
    let directory = root.join(location);
    let Ok(read_dir) = std::fs::read_dir(&directory) else {
        push(
            errors,
            "rfc-naming",
            format!("{location}: directory unreadable"),
        );
        return;
    };
    for entry in read_dir {
        let Ok(entry) = entry else {
            push(
                errors,
                "rfc-naming",
                format!("{location}: entry unreadable"),
            );
            continue;
        };
        let Ok(file_type) = entry.file_type() else {
            push(
                errors,
                "rfc-naming",
                format!("{location}: entry type unreadable"),
            );
            continue;
        };
        let is_kind_match = match kind {
            EntryKind::File => file_type.is_file(),
            EntryKind::Directory => file_type.is_dir(),
        };
        if !is_kind_match {
            // Not this location's governed entry kind (e.g. a stray directory under a
            // files-governed location) -- out of this check's scope, not a naming failure.
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let conforms = match kind {
            EntryKind::File => conforms_file(&name),
            EntryKind::Directory => conforms_slug(&name),
        };
        if !conforms && !legacy.contains(&name.as_ref()) {
            push(
                errors,
                "rfc-naming",
                format!("{location}/{name}: does not conform and is not in the legacy allowlist"),
            );
        }
    }
    // The self-guard (RFC criterion 3): every allowlisted name must correspond to an entry that
    // actually exists at this location, checked directly against the filesystem rather than
    // trusted -- a name here for something that does not exist would be a bypass ("pre-authorise
    // DC-100 before creating it") the allowlist must not offer.
    for name in legacy {
        let path = directory.join(name);
        let exists = match kind {
            EntryKind::File => path.is_file(),
            EntryKind::Directory => path.is_dir(),
        };
        if !exists {
            push(
                errors,
                "rfc-naming",
                format!("{location}/{name}: allowlisted but does not exist"),
            );
        }
    }
}

/// `^[0-9]{3}-[a-z0-9]+(-[a-z0-9]+)*\.md$` -- a file name.
fn conforms_file(name: &str) -> bool {
    name.strip_suffix(".md").is_some_and(conforms_slug)
}

/// `^[0-9]{3}-[a-z0-9]+(-[a-z0-9]+)*$` -- a bare slug (a directory name, or a file name with its
/// `.md` suffix already stripped). Hand-rolled rather than a `regex` dependency: `tools/release-\
/// policy` has none today, and this pattern is simple enough that adding one would be a new
/// supply-chain entrant for a handful of ASCII-byte comparisons.
fn conforms_slug(value: &str) -> bool {
    let Some((number, rest)) = value.split_once('-') else {
        return false;
    };
    if number.len() != 3 || !number.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    !rest.is_empty()
        && rest.split('-').all(|segment| {
            !segment.is_empty()
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
}

#[cfg(test)]
#[path = "rfc_naming/tests.rs"]
mod tests;
