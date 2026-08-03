# RFC (proposed) - DC-70 Prebuilt Binary Distribution

**Status.** **Proposed 2026-08-03.** Awaits owner acceptance.
**Authored by** the architect, who holds minor/patch release scheduling by owner delegation.
**Independence.** Author-reviewed — the standing ceiling.
**Arises from.** The owner's 2026-08-03 direction that installation should not require a Rust toolchain.
**Requirement.** None. This is adoption surface, not a requirement gate.

## 1. The problem

`cargo install prikk` works as of 0.18.1 and is a real improvement over building from a clone — but it
**requires a Rust toolchain**, which is a prerequisite most evaluators will not have and some will not
install. For a project whose first intended persona is a reviewer who *verifies* rather than authors, an
install path that begins "first install Rust" is a barrier at exactly the wrong point.

**Prebuilt binaries attached to GitHub Releases remove that**, and also unlock `cargo binstall` for free —
it resolves from release assets when they follow a recognised naming convention.

## 2. What this is

Per-target binaries built in CI, attached to each release tag, with checksums. `.github/workflows/` already
exists (`ci.yml`, `docs.yml`), so this adds a workflow rather than a build system.

## 3. What must be established before designing — blocking

| Question | Why it blocks |
|---|---|
| **Does the release-evidence model extend to N artifacts?** | `release/fixtures/release-evidence-*.json` has a **singular** `archive` object (`name`, `checksum_name`, `archive_sha256`, `checksum_sha256`), modelled on one source tarball. Per-target binaries are N artifacts. Either the schema extends, or evidence stops describing what was actually published — **the second is not acceptable** for this project |
| **Does `release-policy` validate those fields?** | If `check`/`boundary-check` encode the archive shape, adding artifacts changes oracle cases, not just a workflow |
| **Which targets, and does the code build on them?** | DC-37 restricts *mutation* to Linux. Read-only commands may build elsewhere — **verify, do not assume**. A published macOS binary that cannot commit is a support burden unless its limits are stated at the download |
| **What does `cargo binstall` actually require?** | Asset naming and `[package.metadata.binstall]`. Getting it wrong yields a silently unused feature |

**All four are answerable by reading and one trial build.**

## 4. The question this increment must not answer by accident

**Are release binaries signed, and by whom?**

`release-signers.toml` is empty and fail-closed; no release passes the DC-35 signer gate; 0.18.1 says so in
its own notes. **Publishing binaries makes that gap more consequential**, because a binary is what a user
executes, and a checksum published beside it on the same page proves only integrity of transport, not
authority of origin.

**This increment must not create the appearance of signed releases.** Either binaries carry the same
explicit "not signer-authority-audited" statement the 0.18.1 notes carry, or signing is deferred to DC-43
and said so plainly at the download. **Silence is the failure mode.**

## 5. Acceptance criteria

1. §3's four questions answered and reported before a workflow is written.
2. Binaries built in CI for the agreed targets, attached to the release tag, with per-artifact checksums.
3. **Release evidence describes what was actually published** — every artifact, or the schema is extended
   to allow that. Evidence that omits published artifacts is a defect, not a simplification.
4. `cargo binstall prikk` works end to end, demonstrated — not merely configured.
5. **Every download surface states the release-authority position** per §4, in the same terms as 0.18.1's
   notes.
6. Non-Linux artifacts, if published, state the mutation limit at the download.
7. The build is reproducible from the tag by a third party — documented command, not implicit CI state.
8. Full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9, plus release-policy `check`, `boundary-check`,
   `reference-check`.

## 6. Non-goals

- **Package-manager integration** — Homebrew, AUR, distro packages, containers. The owner named this a
  **future theme** on 2026-08-03; it is deliberately out of scope here, and this increment should leave
  artifact naming and checksums in a shape those can consume later.
- **Signing binaries.** §4: state the position, do not invent authority. DC-43 owns release security.
- **Changing what `cargo install prikk` does.** It stays the toolchain path.
- **Windows/macOS mutation support.** DC-37's boundary is unchanged.
