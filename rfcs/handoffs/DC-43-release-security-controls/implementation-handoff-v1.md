# DC-43 Release Security and Distribution Controls - Implementation Handoff

**Prepared in advance.** Implementation may **not** begin until `rfcs/proposed/DC-43-…` moves to
`rfcs/accepted/` through design review **and** its security review, and it should consume the settled
post-DC-52 tooling gate rather than extend the retained Python engine.
**Authored by** the architect (function-designer role). Implementation review remains independent.
**Size:** medium. Design work can proceed without credentials; anything touching signing keys or registry
credentials cannot.
**Touches:** repository security metadata, release artifact controls, CI release workflow. No product
code.

## Why this exists

Architect review N7: the repository publishes crates and documentation but has no tracked `SECURITY.md`,
no SBOM or release-attestation workflow, no dependency policy configuration, and no documented
vulnerability-reporting path. The README's experimental warning is appropriate and should stay — but a
project asking users to trust signed, verifiable history needs a route for someone to report a problem in
it.

## Scope

1. **Vulnerability reporting.** A tracked `SECURITY.md` with a reporting channel, expected response
   posture, and explicit scope (what is in and out given the experimental status).
2. **Dependency policy.** Configuration making the current implicit posture explicit — the workspace has
   a deliberately minimal production set (`ed25519-dalek`, `getrandom`, `rustix`) and 169 locked packages.
   `cargo audit` already runs; this makes the *policy* reviewable, not just the scan.
3. **SBOM and provenance.** Generated per release artifact, verifiable offline.
4. **Distribution verification.** How a consumer verifies what they fetched matches what was published.

## Sequencing constraints

- **Consume, do not extend.** DC-45's consolidation made the Rust command authoritative; DC-52 retires the
  Python path. DC-43 must build on the settled gate. Extending the retained Python engine would re-create
  the dual-ownership debt DC-45 spent eleven rounds removing.
- **Interacts with DC-51.** If DC-51 has landed, the dependency-placement gate is the enforcement point
  for part of the dependency policy — reference it rather than duplicating it.
- **Release-lane boundary.** DC-43 defines and builds controls. It does **not** perform a release, request
  a fingerprint, bootstrap a signer, or activate the release lane. Artifact-signing controls may be
  designed and tested without ever exercising a real signer.

## Traps

- **Do not** let SBOM/provenance work drift into signing-key handling. Signer governance is DC-35's, the
  allowlist is empty and fail-closed, and touching it is a release-lane action.
- **Do not** add a CI job without the accompanying classifier amendment — `.github/workflows/ci.yml` is a
  governed procedure file, and any new `run:` command must match an accepted production or
  `boundary-check`/`reference-check` fail closed. This is the DC-46 pattern.
- **Do not** upgrade the public posture as a side effect. Adding a `SECURITY.md` does not make the project
  production-ready, and the README's experimental warning stays.
- SBOM tooling may add dependencies. If it does, the DC-41 stage-3/4 discipline applies: dev-only
  placement, MSRV re-verified on the integrated workspace, `Cargo.lock` re-freeze recorded, advisory
  surface reported.

## Definition of done

- `SECURITY.md` tracked, with reporting channel and honest scope.
- Dependency policy configuration present and enforced or explicitly advisory (state which).
- SBOM generated per release artifact and verifiable offline.
- Distribution verification documented end-to-end from a consumer's position.
- Any new CI command carries its classifier amendment in the same increment.
- No signer, release-lane, tag, or publication action taken.
- Full gate set green (`rfcs/EXECUTION-ORDER.md` §6.8).

## Submit with

Diff; evidence note covering each of the four scope items and their verification method; any dependency
addition with placement, MSRV, and lockfile evidence; gate output; explicit statement that no signer or
release-lane state changed and no public readiness claim was upgraded.
