# DC-46 - Workspace Rust 1.85 Compatibility

**Status:** Proposed  
**Milestone:** M2  
**Release target:** 0.19.0 or an explicitly rescheduled release  
**Trigger:** Begin after DC-45 Rust policy-tool cutover is accepted and before the 0.19.0 release
candidate.

## Problem

The workspace declares Rust 1.85 as its minimum supported Rust version, but the existing product
workspace does not currently pass its full locked gate on Rust 1.85. Architect review of DC-45
classified this as pre-existing and non-blocking for the isolated release-policy tool, while requiring
a separately tracked corrective increment.

This mismatch makes the declared minimum version unreliable and prevents release evidence from
distinguishing a product compatibility failure from a DC-45 tooling regression.

## Required Design Work

Before implementation, architect review must establish:

1. the authoritative Rust-version declaration and exact Rust 1.85 gate;
2. whether compatibility is restored by dependency resolution, source changes, or a reviewed minimum
   version amendment;
3. lockfile and package-publication implications across the seven product crates and the unpublished
   release-policy tool;
4. CI evidence and failure attribution for product and internal-tool surfaces; and
5. rollback and release-blocking behavior if the selected compatibility contract cannot be met.

## Scope Boundaries

This RFC does not authorize dependency changes, source compatibility repairs, a minimum-version
increase, release-policy authority cutover, signer bootstrap, publication, or release. Those actions
require an accepted design and their own implementation evidence.

## Completion Gate

DC-46 is complete only when the accepted compatibility decision is implemented, the exact locked
workspace and package gates pass on the selected minimum Rust version, current-toolchain gates remain
green, release documentation states the reviewed contract, and architect implementation review
accepts the evidence.
